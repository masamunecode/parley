//! Locally generated world: servers, channels, members and deterministic messages.
//!
//! Messages are stored as 12-byte metas; their text, code blocks, images and
//! reactions are regenerated on demand from a per-message seed.
use gpui::SharedString;
use std::collections::HashSet;

pub const MAIN_MESSAGES: usize = 100_000;
pub const MEMBER_COUNT: usize = 1000;

// ---------------------------------------------------------------- rng

pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ 0xD1B5_4A32_D192_ED03)
    }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
    pub fn range(&mut self, a: usize, b: usize) -> usize {
        a + self.below(b - a + 1)
    }
    pub fn f(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn chance(&mut self, p: f32) -> bool {
        self.f() < p
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}

// ---------------------------------------------------------------- clock

#[derive(Clone, Copy)]
pub struct Clock {
    pub now: i64,
    pub tz: i64,
}

const MONTHS: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}

impl Clock {
    pub fn system() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(1_794_000_000);
        let tz = unsafe {
            let t = now as libc::time_t;
            let mut tm: libc::tm = std::mem::zeroed();
            libc::localtime_r(&t, &mut tm);
            tm.tm_gmtoff as i64
        };
        Clock { now, tz }
    }

    pub fn day(&self, ts: u32) -> i64 {
        (ts as i64 + self.tz).div_euclid(86_400)
    }

    pub fn time(&self, ts: u32) -> String {
        let s = (ts as i64 + self.tz).rem_euclid(86_400);
        let (h, m) = (s / 3600, (s % 3600) / 60);
        let (h12, ap) = match h {
            0 => (12, "AM"),
            1..=11 => (h, "AM"),
            12 => (12, "PM"),
            _ => (h - 12, "PM"),
        };
        format!("{h12}:{m:02} {ap}")
    }

    pub fn stamp(&self, ts: u32) -> String {
        let day = self.day(ts);
        let today = (self.now + self.tz).div_euclid(86_400);
        if day == today {
            format!("Today at {}", self.time(ts))
        } else if day == today - 1 {
            format!("Yesterday at {}", self.time(ts))
        } else {
            let (y, m, d) = civil(day);
            format!("{m:02}/{d:02}/{y} {}", self.time(ts))
        }
    }

    pub fn long_date(&self, ts: u32) -> String {
        let (y, m, d) = civil(self.day(ts));
        format!("{} {}, {}", MONTHS[(m - 1) as usize], d, y)
    }
}

// ---------------------------------------------------------------- users

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Online,
    Idle,
    Dnd,
    Offline,
}

pub struct RoleDef {
    pub name: &'static str,
    pub color: u32,
}

pub const ROLES: [RoleDef; 6] = [
    RoleDef { name: "Founders", color: 0xff8370 },
    RoleDef { name: "Moderators", color: 0x4fd1c5 },
    RoleDef { name: "Core Team", color: 0xf6a03c },
    RoleDef { name: "Contributors", color: 0xb79cf7 },
    RoleDef { name: "Bots", color: 0x7aa7ff },
    RoleDef { name: "Online", color: 0 },
];

pub struct User {
    pub name: SharedString,
    pub initials: SharedString,
    pub hue: f32,
    pub status: Status,
    pub role: u8,
    pub activity: Option<SharedString>,
    pub bot: bool,
}

impl User {
    pub fn name_color(&self) -> u32 {
        let c = ROLES[self.role as usize].color;
        if c == 0 {
            crate::theme::TEXT_BRIGHT
        } else {
            c
        }
    }
}

const FIRST: &[&str] = &[
    "Mira", "Tobias", "Anouk", "Kenji", "Sasha", "Imani", "Leif", "Priya", "Mateo", "Yuki", "Noor",
    "Felix", "Ingrid", "Dario", "Zofia", "Kofi", "Elena", "Arjun", "Maeve", "Luca", "Hana", "Omar",
    "Freya", "Niko", "Tessa", "Ravi", "Sol", "Juno", "Emeka", "Ada", "Bram", "Callum", "Dalia",
    "Esme", "Farid", "Greta", "Hugo", "Isla", "Jonas", "Kaia", "Lior", "Maren", "Nadia", "Otto",
    "Petra", "Quinn", "Rhea", "Soren", "Talia", "Uma", "Viktor", "Wren", "Xavi", "Yara", "Zane",
    "Amara", "Bodhi", "Cleo", "Dmitri", "Edda", "Finn", "Gwen", "Hiro", "Ines", "Jude", "Kira",
    "Lars", "Mina", "Nils", "Oona", "Pavel", "Rosa", "Sven", "Thea", "Ugo", "Vera", "Wim", "Zora",
];

const LAST: &[&str] = &[
    "Okafor", "Lindqvist", "Tanaka", "Moreau", "Castellanos", "Novak", "Haddad", "Brennan",
    "Kowalski", "Ishikawa", "Marchetti", "Solberg", "Adeyemi", "Varga", "Petrov", "Nakamura",
    "Oyelaran", "Fontaine", "Reyes", "Halloran", "Aziz", "Dubois", "Sorensen", "Mbeki",
];

const ADJ: &[&str] = &[
    "velvet", "lunar", "quiet", "rusty", "neon", "cosmic", "sleepy", "feral", "amber", "pixel",
    "static", "hollow", "turbo", "mossy", "salty", "glitchy", "paper", "cobalt", "fuzzy", "brisk",
    "atomic", "lucid", "wired", "frosty", "solar", "dusty", "vivid", "humble", "rapid", "tidal",
    "minty", "burnt", "polar", "woven", "idle", "async", "unsafe", "const", "inline", "lazy",
];

const NOUN: &[&str] = &[
    "fox", "moth", "otter", "kernel", "lantern", "comet", "badger", "pointer", "sprocket", "heron",
    "walrus", "quark", "tapir", "gecko", "marmot", "byte", "raven", "lynx", "pangolin", "yak",
    "ferret", "beacon", "cipher", "wombat", "falcon", "pebble", "socket", "newt", "crab", "willow",
    "vector", "mutex", "shader", "thread", "buffer", "packet", "glyph", "sprite", "voxel", "tensor",
];

const ACTIVITIES: &[&str] = &[
    "Playing Factory Lines",
    "Listening to lo-fi beats",
    "Writing shaders",
    "Compiling…",
    "In a meeting",
    "Playing Deep Rock Survey",
    "Touching grass",
    "Reviewing pull requests",
    "Streaming a build log",
    "Reading the Rust book",
    "Debugging a lifetime",
    "Out for coffee ☕",
    "Playing Hollow Circuit",
    "Listening to Synthwave FM",
    "cargo check --watch",
    "Fighting the linker",
    "Pair programming",
    "Drawing pixel art",
];

fn cap(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn initials(name: &str) -> String {
    let mut out = String::new();
    let mut prev_alnum = false;
    let mut prev_lower = false;
    for ch in name.chars() {
        let alnum = ch.is_alphanumeric();
        let boundary = alnum && (!prev_alnum || (ch.is_uppercase() && prev_lower));
        if boundary && out.chars().count() < 2 {
            out.extend(ch.to_uppercase());
        }
        prev_lower = ch.is_lowercase();
        prev_alnum = alnum;
    }
    if out.is_empty() {
        out.push('?');
    }
    out
}

fn gen_users() -> Vec<User> {
    let mut rng = Rng::new(0xC0FFEE);
    let mut seen: HashSet<String> = HashSet::new();
    let mut users = Vec::with_capacity(MEMBER_COUNT);

    users.push(User {
        name: "Juniper".into(),
        initials: "J".into(),
        hue: 0.08,
        status: Status::Online,
        role: 2,
        activity: Some("Building a chat client".into()),
        bot: false,
    });
    seen.insert("Juniper".into());

    let bots = ["Foundry Bot", "Clippy Jr.", "Gatekeeper", "Changelog", "Dice Goblin", "Uptime Owl"];

    for i in 1..MEMBER_COUNT {
        let role: u8 = match i {
            1..=3 => 0,
            4..=12 => 1,
            13..=34 => 2,
            35..=124 => 3,
            125..=130 => 4,
            _ => 5,
        };
        let bot = role == 4;
        let name = if bot {
            bots[i - 125].to_string()
        } else {
            let mut name;
            let mut tries = 0;
            loop {
                name = match rng.below(10) {
                    0 | 1 => rng.pick(FIRST).to_string(),
                    2 | 3 => format!("{}{}", cap(rng.pick(ADJ)), cap(rng.pick(NOUN))),
                    4 | 5 => format!("{}_{}", rng.pick(ADJ), rng.pick(NOUN)),
                    6 => format!("{}{}", rng.pick(FIRST).to_lowercase(), rng.range(2, 99)),
                    7 | 8 => format!("{} {}", rng.pick(FIRST), rng.pick(LAST)),
                    _ => format!("{}.{}", rng.pick(NOUN), rng.pick(NOUN)),
                };
                tries += 1;
                if tries > 4 {
                    name = format!("{}{}", name, rng.range(2, 99));
                }
                if !seen.contains(&name) {
                    break;
                }
            }
            seen.insert(name.clone());
            name
        };

        let offline_p = match role {
            0 | 1 => 0.1,
            2 => 0.2,
            3 => 0.35,
            4 => 0.0,
            _ => 0.56,
        };
        let status = if rng.chance(offline_p) {
            Status::Offline
        } else {
            match rng.below(10) {
                0..=5 => Status::Online,
                6 | 7 => Status::Idle,
                _ => Status::Dnd,
            }
        };
        let status = if bot { Status::Online } else { status };
        let activity = if bot {
            None
        } else if status != Status::Offline && rng.chance(0.3) {
            Some(SharedString::from(*rng.pick(ACTIVITIES)))
        } else {
            None
        };
        users.push(User {
            initials: initials(&name).into(),
            name: name.into(),
            hue: rng.f(),
            status,
            role,
            activity,
            bot,
        });
    }
    users
}

// ---------------------------------------------------------------- servers

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ChanKind {
    Text,
    Announce,
    Voice,
}

#[derive(Clone, Copy)]
pub struct MsgMeta {
    pub ts: u32,
    pub seed: u32,
    pub author: u16,
    pub flags: u8,
}

pub const F_REPLY: u8 = 1;
pub const F_EDITED: u8 = 2;
pub const F_CUSTOM: u8 = 4;

pub struct Channel {
    pub name: SharedString,
    pub topic: SharedString,
    pub kind: ChanKind,
    pub cat: usize,
    pub unread: bool,
    pub mentions: u32,
    pub metas: Vec<MsgMeta>,
    pub custom: Vec<SharedString>,
    pub voice: Vec<usize>,
    pub dev: bool,
}

pub enum MemberRow {
    Header(SharedString),
    User(u32),
}

pub struct Server {
    pub name: SharedString,
    pub initials: SharedString,
    pub hue: f32,
    pub cats: Vec<SharedString>,
    pub channels: Vec<Channel>,
    pub rows: Vec<MemberRow>,
    pub member_count: usize,
    pub online_count: usize,
    pub default_channel: usize,
    pub active: Vec<u16>,
}

impl Server {
    pub fn unread(&self) -> bool {
        self.channels.iter().any(|c| c.unread)
    }
    pub fn mentions(&self) -> u32 {
        self.channels.iter().map(|c| c.mentions).sum()
    }
}

struct ChanDef(&'static str, ChanKind, &'static str, usize, u8, u32);
// name, kind, topic, message count, unread(0/1), mentions

struct ServerDef {
    name: &'static str,
    initials: &'static str,
    hue: f32,
    fraction: f32,
    cats: &'static [(&'static str, &'static [ChanDef])],
}

use ChanKind::{Announce as A, Text as T, Voice as V};

const SERVERS: &[ServerDef] = &[
    ServerDef {
        name: "Ferrous Foundry",
        initials: "FF",
        hue: 0.07,
        fraction: 1.0,
        cats: &[
            (
                "Welcome",
                &[
                    ChanDef("rules", T, "Be kind, stay curious, no crossposting.", 14, 0, 0),
                    ChanDef("announcements", A, "Releases, events and the occasional victory lap.", 220, 1, 0),
                ],
            ),
            (
                "Community",
                &[
                    ChanDef("general", T, "Everything and nothing. 100,000 messages and counting.", MAIN_MESSAGES, 0, 0),
                    ChanDef("introductions", T, "Say hi! Tell us what you're hacking on.", 900, 0, 0),
                    ChanDef("off-topic", T, "Keyboards, coffee, cats.", 4200, 1, 0),
                    ChanDef("showcase", T, "Show us what you made. Screenshots encouraged.", 1800, 1, 3),
                    ChanDef("memes", T, "It compiles, ship it.", 2600, 0, 0),
                ],
            ),
            (
                "Engineering",
                &[
                    ChanDef("help", T, "Ask anything. Post the full error, please.", 6400, 1, 1),
                    ChanDef("gpui", T, "GPU-accelerated UI: layout, text, scenes.", 3800, 0, 0),
                    ChanDef("async-await", T, "Futures, executors and Send bounds.", 2100, 0, 0),
                    ChanDef("shaders", T, "WGSL, MSL and signed distance fields.", 1500, 1, 0),
                    ChanDef("perf-tuning", T, "Flamegraphs or it didn't happen.", 1900, 0, 0),
                    ChanDef("code-review", T, "Fresh eyes for your pull requests.", 1100, 0, 0),
                ],
            ),
            ("Voice", &[ChanDef("Lounge", V, "", 0, 0, 0)]),
        ],
    },
    ServerDef {
        name: "Pixel Forge",
        initials: "PF",
        hue: 0.82,
        fraction: 0.34,
        cats: &[
            (
                "Studio",
                &[
                    ChanDef("announcements", A, "Jam dates and studio news.", 90, 0, 0),
                    ChanDef("devlogs", T, "Weekly progress, wins and disasters.", 1300, 1, 0),
                ],
            ),
            (
                "Make",
                &[
                    ChanDef("pixel-art", T, "16 colors is plenty.", 2400, 1, 2),
                    ChanDef("level-design", T, "Teach without tutorials.", 800, 0, 0),
                    ChanDef("audio", T, "Bleeps, bloops and trackers.", 600, 0, 0),
                    ChanDef("shaders", T, "CRT filters for everyone.", 700, 0, 0),
                ],
            ),
            (
                "Play",
                &[
                    ChanDef("playtests", T, "Break each other's builds.", 1100, 0, 0),
                    ChanDef("game-jams", T, "48 hours, one theme.", 950, 1, 0),
                    ChanDef("Jam Room", V, "", 0, 0, 0),
                ],
            ),
        ],
    },
    ServerDef {
        name: "Synth Garden",
        initials: "SG",
        hue: 0.47,
        fraction: 0.22,
        cats: &[
            (
                "Start",
                &[ChanDef("welcome", T, "Patch cables are a love language.", 60, 0, 0)],
            ),
            (
                "Sound",
                &[
                    ChanDef("patches", T, "Share your presets.", 1700, 0, 0),
                    ChanDef("modular", T, "Rack pics and wallet regrets.", 2200, 1, 0),
                    ChanDef("gear-talk", T, "What to buy next (nothing, you say).", 1400, 0, 0),
                    ChanDef("wip-tracks", T, "Unfinished is fine.", 900, 0, 0),
                    ChanDef("feedback", T, "Honest ears, kind words.", 500, 0, 0),
                    ChanDef("Listening Party", V, "", 0, 0, 0),
                ],
            ),
        ],
    },
    ServerDef {
        name: "Trailheads",
        initials: "TH",
        hue: 0.36,
        fraction: 0.28,
        cats: &[
            (
                "Basecamp",
                &[
                    ChanDef("trip-reports", T, "Where did you go this weekend?", 1600, 1, 5),
                    ChanDef("route-planning", T, "Maps, permits, water sources.", 1200, 0, 0),
                    ChanDef("gear", T, "Ultralight or ultra-comfy.", 2100, 0, 0),
                    ChanDef("photos", T, "Summits, sunsets, soggy boots.", 1900, 1, 0),
                    ChanDef("meetups", T, "Find a hiking buddy.", 400, 0, 0),
                    ChanDef("Campfire", V, "", 0, 0, 0),
                ],
            ),
        ],
    },
    ServerDef {
        name: "Kernel Panic",
        initials: "KP",
        hue: 0.58,
        fraction: 0.4,
        cats: &[
            (
                "Userspace",
                &[
                    ChanDef("distro-wars", T, "Be nice. It's just an operating system.", 5200, 0, 0),
                    ChanDef("dotfiles", T, "Show your config.", 1800, 0, 0),
                    ChanDef("ricing", T, "Screenshots of terminals.", 1500, 0, 0),
                ],
            ),
            (
                "Kernel",
                &[
                    ChanDef("homelab", T, "It's always DNS.", 2700, 0, 0),
                    ChanDef("help-desk", T, "Have you tried turning it off and on?", 3300, 0, 0),
                    ChanDef("Server Room", V, "", 0, 0, 0),
                ],
            ),
        ],
    },
    ServerDef {
        name: "Night Owls",
        initials: "NO",
        hue: 0.69,
        fraction: 0.16,
        cats: &[
            (
                "Library",
                &[
                    ChanDef("study-hall", T, "Quiet-ish co-working.", 1400, 1, 0),
                    ChanDef("accountability", T, "Post your goal, report back.", 1000, 0, 0),
                    ChanDef("resources", T, "Notes, decks and good links.", 450, 0, 0),
                    ChanDef("wins", T, "Small ones count.", 700, 0, 0),
                    ChanDef("Focus Room", V, "", 0, 0, 0),
                ],
            ),
        ],
    },
    ServerDef {
        name: "Tabletop Tavern",
        initials: "TT",
        hue: 0.97,
        fraction: 0.25,
        cats: &[
            (
                "The Tavern",
                &[
                    ChanDef("looking-for-group", T, "Find a table.", 1300, 0, 0),
                    ChanDef("rules-lawyers", T, "RAW vs RAI, forever.", 2000, 0, 0),
                    ChanDef("homebrew", T, "Monsters, classes and cursed items.", 1500, 1, 1),
                    ChanDef("session-recaps", T, "Last time, on our campaign…", 800, 0, 0),
                    ChanDef("minis", T, "Thin your paints.", 650, 0, 0),
                    ChanDef("Table One", V, "", 0, 0, 0),
                ],
            ),
        ],
    },
    ServerDef {
        name: "Type Foundry",
        initials: "TF",
        hue: 0.15,
        fraction: 0.18,
        cats: &[
            (
                "Studio",
                &[
                    ChanDef("critique", T, "Kern with compassion.", 1200, 0, 0),
                    ChanDef("typefaces", T, "New releases and old favorites.", 1700, 0, 0),
                    ChanDef("inspiration", T, "Posters, specimens, signage.", 1400, 0, 0),
                    ChanDef("tools", T, "Editors, scripts, plugins.", 600, 0, 0),
                    ChanDef("jobs", T, "Paid gigs only.", 200, 0, 0),
                    ChanDef("Studio", V, "", 0, 0, 0),
                ],
            ),
        ],
    },
    ServerDef {
        name: "Bread Lab",
        initials: "BL",
        hue: 0.11,
        fraction: 0.2,
        cats: &[
            (
                "Kitchen",
                &[
                    ChanDef("sourdough", T, "Feed your starter.", 2300, 1, 0),
                    ChanDef("crumb-shots", T, "Cross sections or it didn't rise.", 1500, 0, 0),
                    ChanDef("troubleshooting", T, "Why is it gummy?", 1100, 0, 0),
                    ChanDef("recipes", T, "Weights, not cups.", 900, 0, 0),
                    ChanDef("Kitchen", V, "", 0, 0, 0),
                ],
            ),
        ],
    },
];

fn gen_metas(rng: &mut Rng, count: usize, active: &[u16], now: i64) -> Vec<MsgMeta> {
    let mut rel: Vec<(i64, u16, u32, u8)> = Vec::with_capacity(count);
    let mut t: i64 = 0;
    let mut prev_author = u16::MAX;
    while rel.len() < count {
        let n = active.len();
        let mut author = active[rng.below(n).min(rng.below(n))];
        if author == prev_author {
            author = active[rng.below(n)];
        }
        prev_author = author;
        let mut burst = 1;
        while burst < 6 && rng.chance(0.5) {
            burst += 1;
        }
        let f = rng.f();
        t += 15 + (f * f * f * 1200.) as i64;
        if rng.chance(0.01) {
            t += rng.range(2 * 3600, 8 * 3600) as i64;
        }
        for b in 0..burst {
            if rel.len() >= count {
                break;
            }
            if b > 0 {
                t += rng.range(3, 60) as i64;
            }
            let mut flags = 0;
            if b == 0 && rel.len() > 40 && rng.chance(0.11) {
                flags |= F_REPLY;
            }
            if rng.chance(0.04) {
                flags |= F_EDITED;
            }
            rel.push((t, author, rng.next() as u32, flags));
        }
    }
    let shift = now - 70 - t;
    rel.into_iter()
        .map(|(t, author, seed, flags)| MsgMeta {
            ts: (t + shift).max(0) as u32,
            seed,
            author,
            flags,
        })
        .collect()
}

fn gen_servers(users: &[User], clock: &Clock) -> Vec<Server> {
    let mut out = Vec::new();
    for (si, def) in SERVERS.iter().enumerate() {
        let mut rng = Rng::new(0xFACADE + si as u64 * 7919);
        // Membership
        let members: Vec<u32> = (0..users.len() as u32)
            .filter(|&u| u == 0 || def.fraction >= 1.0 || Rng::new(u as u64 * 31 + si as u64).f() < def.fraction)
            .collect();
        let mut rows = Vec::new();
        let mut online_count = 0;
        for role in 0..6u8 {
            let mut group: Vec<u32> = members
                .iter()
                .copied()
                .filter(|&u| users[u as usize].role == role && users[u as usize].status != Status::Offline)
                .collect();
            if group.is_empty() {
                continue;
            }
            group.sort_by(|a, b| {
                users[*a as usize]
                    .name
                    .to_lowercase()
                    .cmp(&users[*b as usize].name.to_lowercase())
            });
            online_count += group.len();
            rows.push(MemberRow::Header(
                format!("{} — {}", ROLES[role as usize].name, group.len()).into(),
            ));
            rows.extend(group.into_iter().map(MemberRow::User));
        }
        let mut offline: Vec<u32> = members
            .iter()
            .copied()
            .filter(|&u| users[u as usize].status == Status::Offline)
            .collect();
        offline.sort_by(|a, b| {
            users[*a as usize]
                .name
                .to_lowercase()
                .cmp(&users[*b as usize].name.to_lowercase())
        });
        if !offline.is_empty() {
            rows.push(MemberRow::Header(format!("Offline — {}", offline.len()).into()));
            rows.extend(offline.into_iter().map(MemberRow::User));
        }

        // Active posters: biased towards staff, includes "me".
        let mut active: Vec<u16> = Vec::new();
        let pool_size = (members.len() / 6).clamp(24, 160);
        let mut tries = 0;
        while active.len() < pool_size && tries < 10_000 {
            tries += 1;
            let u = members[rng.below(members.len()).min(rng.below(members.len()))] as u16;
            if !active.contains(&u) && u != 0 {
                active.push(u);
            }
        }
        active.insert(active.len().min(9), 0);

        let online: Vec<usize> = members
            .iter()
            .map(|&u| u as usize)
            .filter(|&u| u != 0 && !users[u].bot && users[u].status == Status::Online)
            .collect();

        let mut cats = Vec::new();
        let mut channels = Vec::new();
        let mut default_channel = 0;
        for (ci, (cat, chans)) in def.cats.iter().enumerate() {
            cats.push(SharedString::from(*cat));
            for ch in chans.iter() {
                let voice = if ch.1 == ChanKind::Voice {
                    let mut v = Vec::new();
                    while v.len() < 3 {
                        let u = online[rng.below(online.len())];
                        if !v.contains(&u) {
                            v.push(u);
                        }
                    }
                    v
                } else {
                    Vec::new()
                };
                if ch.3 >= 800 && default_channel == 0 && ch.1 == ChanKind::Text {
                    default_channel = channels.len();
                }
                if ch.3 == MAIN_MESSAGES {
                    default_channel = channels.len();
                }
                let metas = gen_metas(&mut rng, ch.3, &active, clock.now);
                channels.push(Channel {
                    name: ch.0.into(),
                    topic: ch.2.into(),
                    kind: ch.1,
                    cat: ci,
                    unread: ch.4 == 1,
                    mentions: ch.5,
                    metas,
                    custom: Vec::new(),
                    voice,
                    dev: si == 0 || si == 4,
                });
            }
        }

        out.push(Server {
            name: def.name.into(),
            initials: def.initials.into(),
            hue: def.hue,
            cats,
            channels,
            rows,
            member_count: members.len(),
            online_count,
            default_channel,
            active,
        });
    }
    out
}

pub struct World {
    pub users: Vec<User>,
    pub servers: Vec<Server>,
    pub clock: Clock,
}

impl World {
    pub fn generate() -> Self {
        let clock = Clock::system();
        let users = gen_users();
        let servers = gen_servers(&users, &clock);
        World { users, servers, clock }
    }
}

// ---------------------------------------------------------------- content

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Span {
    Plain,
    Mention,
    Code,
    Link,
    Muted,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tok {
    Plain,
    Keyword,
    Str,
    Comment,
    Num,
    Type,
}

pub struct Code {
    pub lang: &'static str,
    pub text: SharedString,
    pub spans: Vec<(usize, Tok)>,
}

pub struct Image {
    pub w: f32,
    pub h: f32,
    pub hue: f32,
    pub name: SharedString,
    pub style: u8,
}

pub struct Reaction {
    pub emoji: &'static str,
    pub count: u32,
    pub mine: bool,
}

pub struct Content {
    pub text: SharedString,
    pub spans: Vec<(usize, Span)>,
    pub preview: SharedString,
    pub code: Option<Code>,
    pub image: Option<Image>,
    pub reactions: Vec<Reaction>,
    pub reply_to: Option<usize>,
    pub mentions_me: bool,
}

struct TextBuilder {
    text: String,
    spans: Vec<(usize, Span)>,
}

impl TextBuilder {
    fn push(&mut self, s: &str, kind: Span) {
        if s.is_empty() {
            return;
        }
        self.text.push_str(s);
        match self.spans.last_mut() {
            Some((len, k)) if *k == kind => *len += s.len(),
            _ => self.spans.push((s.len(), kind)),
        }
    }
}

const DEV: &[&str] = &[
    "has anyone tried the new {t} API yet?",
    "I think {t} is still allocating on every frame",
    "the trick is to cache the layout and only reshape text when the width changes",
    "that fixed it for me, went from {n}ms to under 2ms per frame",
    "you can't hold that borrow across an await point, the compiler is right to yell",
    "honestly the borrow checker saved me from a real bug there",
    "try wrapping it in an Rc<RefCell<_>> first and see if the design even works",
    "clippy has a lint for exactly this",
    "we should really write that down in the docs somewhere",
    "profile it before you optimize it, the hot spot is never where you think",
    "release builds are a different universe, don't benchmark in debug",
    "I'd reach for {c} here, it's small and has almost no dependencies",
    "this is the third time this week {t} has bitten me",
    "the flamegraph says {n}% of the time is spent in {t}",
    "virtualize the list: only lay out what's on screen and measure lazily",
    "it's a sum tree under the hood, so seeking to an offset is O(log n)",
    "the GPU side is fine, it's the CPU-side layout pass that hurts",
    "what does your Cargo.toml look like?",
    "can you post the full error? the first line is usually not the useful one",
    "rebase on main, I think that was fixed yesterday",
    "CI is red again but I'm fairly sure it's the flaky {t} test",
    "the PR is up, would love a second pair of eyes on the unsafe block",
    "lifetimes clicked for me once I started reading them as \"lives at least as long as\"",
    "you want an arena for that, individual allocations will kill you",
    "just shipped {n} commits of pure deletions, best day of the month",
    "have you considered not doing that at all? genuinely asking",
    "hot reload for shaders is a game changer",
    "turns out the bug was a stale cache key. of course it was",
    "instancing fixed it, we're down to {n} draw calls",
    "I measured it: 120 fps with 100k rows and the fans never spin up",
    "the trait bound is unsatisfied because the future isn't Send",
    "that's a great question for the weekly office hours",
    "make it work, make it right, then make it fast",
    "both executors are fine, pick one and move on",
    "I rewrote the parser by hand and it's {n}x faster than the generated one",
    "the system text stack handles fallback fonts, so emoji just work",
    "scroll position should be an item index plus an offset, never a raw pixel value",
    "immediate mode vs retained is mostly a false dichotomy at this point",
    "is there an issue tracking this? I'd like to subscribe",
    "my incremental builds went from {n}s to 4s after splitting the crate",
    "turn off debug info for dependencies, linking gets way faster",
    "what's the story for accessibility in {t} these days?",
    "every frame under 8ms or it doesn't ship",
    "{c} just cut a new release, changelog looks spicy",
    "I keep forgetting that {t} runs before paint, not after",
    "the atlas was getting evicted every frame, that's where the hitches came from",
    "don't shape the same string twice, key the cache on text + font + size",
    "a Vec of 12-byte structs beats a Vec of Strings every single time",
    "you only need the seed; regenerate the row when it scrolls into view",
    "does this still repro on the latest nightly?",
    "I'll write up a minimal example tonight",
    "so the fix was one line and the investigation was three days. classic",
    "the docs example is out of date, the signature takes a window now",
    "frame pacing matters more than raw fps, watch the 99th percentile",
    "we could batch those quads, they share the same texture",
    "subpixel positioning makes small text so much nicer",
    "the hit test runs against last frame's layout, that's why hover lags by one",
    "if it's not in the benchmark suite it will regress, guaranteed",
    "that struct is {n} bytes, you could halve it by reordering fields",
    "TIL you can pass a closure there instead of implementing the whole trait",
];

const CASUAL: &[&str] = &[
    "good morning everyone ☕",
    "how is it already Thursday",
    "anyone around for a quick call later?",
    "that's the best thing I've seen all week",
    "I can't believe that actually worked",
    "okay I'm back, what did I miss",
    "sorry for the wall of text earlier",
    "congrats on the launch!! 🎉",
    "this is why I love this place",
    "going to grab lunch, back in a bit",
    "has anyone here actually read the whole thing?",
    "that's a really good point, I hadn't thought about it that way",
    "I'll take a look after work",
    "can someone pin that?",
    "new keyboard day, ask me anything",
    "my cat just walked across the desk and closed everything",
    "the weather finally turned and I am thriving",
    "who's going to the meetup next month?",
    "I've been staring at this for two hours and it was a typo",
    "that screenshot is gorgeous",
    "welcome! glad you found us 👋",
    "ha, I was about to post the exact same thing",
    "strong agree, with one small caveat",
    "let's take this to a thread so we don't flood the channel",
    "I made coffee and then forgot to drink it. again",
    "what's everyone working on this weekend?",
    "thanks, that's super helpful",
    "not me refreshing the page every five minutes",
    "it's so quiet in here today",
    "updated the doc with everything we talked about",
    "this aged well",
    "I need to stop buying side-project domains",
    "big if true",
    "plot twist: it was working the whole time",
    "today's standup is cancelled, enjoy the extra fifteen minutes",
    "putting this in my notes for later",
    "I have opinions about this but I'll save them for voice",
    "there's leftover pizza in the kitchen, first come first served",
    "woke up at 5am for no reason, so here I am",
    "the before/after on that is wild",
    "you love to see it",
    "screenshots or it didn't happen",
    "we should do this more often",
    "ok that's actually really clever",
    "adding that to the list of things I didn't know I needed",
    "no rush at all, whenever you get a chance",
    "counterpoint: what if we just didn't",
    "happy friday, don't deploy anything 🙏",
];

const SHORTS: &[&str] = &[
    "lol", "true", "nice!", "+1", "this ^", "same", "oh no", "wait what", "brb", "gg", "ty!",
    "can confirm", "love that", "😂", "big mood", "o7", "sounds good", "on it", "yep", "hmm",
    "100%", "oof", "ship it 🚀", "nice catch", "👀", "fair", "agreed", "let's go 🔥",
];

const TECH: &[&str] = &[
    "the layout pass", "text shaping", "hit testing", "the scroll handler", "the async runtime",
    "the allocator", "the glyph atlas", "the scene graph", "the Metal backend", "the borrow checker",
    "incremental compilation", "the linker", "the proc macro", "the event loop", "the list view",
    "prepaint", "the render pipeline", "the path rasterizer",
];

const CRATES: &[&str] = &[
    "smallvec", "bumpalo", "rayon", "parking_lot", "slotmap", "ahash", "thiserror", "bytemuck",
    "crossbeam", "indexmap",
];

const INLINE: &[&str] = &[
    "cargo build --release", "cx.notify()", "Rc<RefCell<T>>", "opt-level = 3", "impl IntoElement",
    "&mut Window", "ListState::splice", "#[inline]", "Arc<str>", "cargo clippy --fix",
];

const LINKS: &[&str] = &[
    "https://docs.foundry.example/guide/virtual-lists",
    "https://foundry.example/blog/frame-pacing",
    "https://paste.foundry.example/x9f2k",
    "https://docs.foundry.example/api/list-state",
    "https://foundry.example/rfcs/0042-text-runs",
];

const TAILS: &[&str] = &[" 😄", " 🎉", " 🔥", " 👀", " 🚀", " 😅", " 🙏", " lol", " tbh", " fwiw"];

pub const EMOJI: &[&str] = &["👍", "❤️", "😂", "🔥", "🎉", "👀", "🚀", "🙏", "💯", "🦀", "✨", "😮"];

const SNIPPETS: &[(&str, &str)] = &[
    (
        "rust",
        "fn visible_range(&self, top: f32, height: f32) -> Range<usize> {\n    let start = self.offsets.partition_point(|&y| y < top);\n    let end = self.offsets.partition_point(|&y| y < top + height);\n    start.saturating_sub(1)..end\n}",
    ),
    (
        "rust",
        "let state = ListState::new(messages.len(), ListAlignment::Bottom, px(1000.));\nlist(state, |ix, _window, cx| render_message(ix, cx))",
    ),
    (
        "toml",
        "[profile.dev.package.\"*\"]\nopt-level = 3\ndebug = false",
    ),
    (
        "rust",
        "#[derive(Clone, Copy)]\nstruct MsgMeta {\n    ts: u32,\n    seed: u32,\n    author: u16,\n    flags: u8, // 12 bytes per message\n}",
    ),
    (
        "sh",
        "$ cargo build --release\n   Compiling parley v0.1.0\n    Finished `release` profile [optimized] in 41.2s",
    ),
    (
        "rust",
        "match rx.recv().await {\n    Ok(frame) => renderer.submit(frame),\n    Err(_) => break, // sender dropped\n}",
    ),
    (
        "rust",
        "impl Drop for Guard<'_> {\n    fn drop(&mut self) {\n        self.pool.release(self.slot);\n    }\n}",
    ),
    (
        "wgsl",
        "@fragment\nfn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {\n    let d = sdf_rounded_rect(in.pos, in.size, in.radius);\n    return vec4(in.color.rgb, in.color.a * smoothstep(0.5, -0.5, d));\n}",
    ),
    (
        "python",
        "for batch in chunks(rows, 512):\n    upload(batch)  # 100k rows in ~200 batches",
    ),
    (
        "rust",
        "let content = cache\n    .entry(ix)\n    .or_insert_with(|| Rc::new(generate(seed)));",
    ),
];

const KEYWORDS: &[&str] = &[
    "fn", "let", "mut", "pub", "struct", "impl", "use", "for", "in", "if", "else", "match",
    "return", "async", "await", "const", "def", "import", "from", "break", "self", "false", "true",
    "debug",
];

pub fn highlight(code: &str) -> Vec<(usize, Tok)> {
    let mut out: Vec<(usize, Tok)> = Vec::new();
    let mut push = |len: usize, tok: Tok| {
        if len == 0 {
            return;
        }
        match out.last_mut() {
            Some((l, t)) if *t == tok => *l += len,
            _ => out.push((len, tok)),
        }
    };
    let bytes = code.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'/' && bytes.get(i + 1) == Some(&b'/') || b == b'#' && bytes.get(i + 1) == Some(&b' ') {
            let end = code[i..].find('\n').map(|p| i + p).unwrap_or(bytes.len());
            push(end - i, Tok::Comment);
            i = end;
        } else if b == b'"' {
            let end = code[i + 1..].find('"').map(|p| i + p + 2).unwrap_or(bytes.len());
            push(end - i, Tok::Str);
            i = end;
        } else if b.is_ascii_alphabetic() || b == b'_' {
            let mut j = i;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            let word = &code[i..j];
            let tok = if KEYWORDS.contains(&word) {
                Tok::Keyword
            } else if word.as_bytes()[0].is_ascii_uppercase() {
                Tok::Type
            } else {
                Tok::Plain
            };
            push(j - i, tok);
            i = j;
        } else if b.is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'.') {
                j += 1;
            }
            push(j - i, Tok::Num);
            i = j;
        } else {
            // Advance one full UTF-8 character.
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j] & 0xC0) == 0x80 {
                j += 1;
            }
            push(j - i, Tok::Plain);
            i = j;
        }
    }
    out
}

fn expand(tb: &mut TextBuilder, template: &str, rng: &mut Rng) {
    let mut rest = template;
    while let Some(p) = rest.find('{') {
        tb.push(&rest[..p], Span::Plain);
        let key = &rest[p..p + 3];
        match key {
            "{t}" => tb.push(rng.pick(TECH), Span::Plain),
            "{c}" => tb.push(rng.pick(CRATES), Span::Code),
            "{n}" => {
                let n = [3, 7, 12, 16, 24, 38, 40, 64, 90, 140][rng.below(10)];
                tb.push(&n.to_string(), Span::Plain)
            }
            _ => tb.push(key, Span::Plain),
        }
        rest = &rest[p + 3..];
    }
    tb.push(rest, Span::Plain);
}

fn make_preview(text: &str) -> SharedString {
    let flat: String = text.chars().map(|c| if c == '\n' { ' ' } else { c }).collect();
    if flat.chars().count() > 96 {
        let cut: String = flat.chars().take(96).collect();
        format!("{cut}…").into()
    } else {
        flat.into()
    }
}

/// Parse a user-typed message into spans (@mentions, `code`, links).
pub fn custom_content(text: &str) -> Content {
    let mut tb = TextBuilder { text: String::new(), spans: Vec::new() };
    let mut in_code = false;
    for (i, part) in text.split('`').enumerate() {
        if i > 0 {
            in_code = !in_code;
        }
        if in_code {
            tb.push(part, Span::Code);
            continue;
        }
        let mut word_start = 0;
        let flush = |tb: &mut TextBuilder, w: &str| {
            if w.len() > 1 && w.starts_with('@') {
                tb.push(w, Span::Mention);
            } else if w.starts_with("http://") || w.starts_with("https://") {
                tb.push(w, Span::Link);
            } else {
                tb.push(w, Span::Plain);
            }
        };
        for (j, ch) in part.char_indices() {
            if ch.is_whitespace() {
                flush(&mut tb, &part[word_start..j]);
                tb.push(&part[j..j + ch.len_utf8()], Span::Plain);
                word_start = j + ch.len_utf8();
            }
        }
        flush(&mut tb, &part[word_start..]);
    }
    let preview = make_preview(&tb.text);
    Content {
        text: tb.text.into(),
        spans: tb.spans,
        preview,
        code: None,
        image: None,
        reactions: Vec::new(),
        reply_to: None,
        mentions_me: false,
    }
}

/// Deterministically build the content for message `ix` from its seed.
pub fn gen_content(ix: usize, meta: &MsgMeta, users: &[User], active: &[u16], dev: bool) -> Content {
    let mut rng = Rng::new(meta.seed as u64 ^ 0xA11CE);
    let mut tb = TextBuilder { text: String::new(), spans: Vec::new() };
    let mut mentions_me = false;

    let kind = rng.below(100);
    let has_code = dev && kind < 5;
    let has_image = (5..11).contains(&kind);
    let image_only = has_image && kind < 7;

    if !image_only {
        // Optional leading mention.
        if rng.chance(0.07) {
            let me = rng.chance(0.12) && meta.author != 0;
            let target = if me { 0 } else { active[rng.below(active.len())] as usize };
            mentions_me = me;
            tb.push(&format!("@{}", users[target].name), Span::Mention);
            tb.push(" ", Span::Plain);
        }

        if !has_code && rng.chance(0.16) {
            tb.push(rng.pick(SHORTS), Span::Plain);
        } else {
            let sentences = match rng.below(100) {
                0..=56 => 1,
                57..=85 => 2,
                86..=96 => 3,
                _ => rng.range(5, 8),
            };
            for s in 0..sentences {
                if s > 0 {
                    let last = tb.text.chars().last().unwrap_or('.');
                    tb.push(if matches!(last, '.' | '?' | '!' | '☕' | '🎉' | '👋' | '🙏') { " " } else { ". " }, Span::Plain);
                }
                let corpus = if dev && rng.chance(0.72) { DEV } else { CASUAL };
                let template = *rng.pick(corpus);
                expand(&mut tb, template, &mut rng);
            }
            if rng.chance(0.07) {
                tb.push(" — try ", Span::Plain);
                tb.push(rng.pick(INLINE), Span::Code);
            }
            if rng.chance(0.08) {
                tb.push(rng.pick(TAILS), Span::Plain);
            }
            if rng.chance(0.035) {
                tb.push(" ", Span::Plain);
                tb.push(rng.pick(LINKS), Span::Link);
            }
            if has_code {
                tb.push(":", Span::Plain);
            }
        }
        if meta.flags & F_EDITED != 0 {
            tb.push(" (edited)", Span::Muted);
        }
    }

    let code = if has_code {
        let (lang, text) = *rng.pick(SNIPPETS);
        Some(Code { lang, text: text.into(), spans: highlight(text) })
    } else {
        None
    };

    let image = if has_image {
        let (w, h) = *rng.pick(&[(400., 225.), (320., 240.), (360., 200.), (260., 320.), (420., 260.)]);
        let exts = ["png", "jpg", "webp"];
        let stems = ["screenshot", "IMG", "capture", "render", "photo", "frame", "wip"];
        Some(Image {
            w,
            h,
            hue: rng.f(),
            name: format!("{}_{}.{}", rng.pick(&stems), rng.range(1000, 9999), rng.pick(&exts)).into(),
            style: rng.below(3) as u8,
        })
    } else {
        None
    };

    let mut reactions = Vec::new();
    if rng.chance(if has_image || has_code { 0.5 } else { 0.11 }) {
        let n = rng.range(1, 4);
        let mut used = [false; 12];
        for _ in 0..n {
            let e = rng.below(EMOJI.len());
            if used[e] {
                continue;
            }
            used[e] = true;
            let count = 1 + (rng.f() * rng.f() * 24.) as u32;
            reactions.push(Reaction { emoji: EMOJI[e], count, mine: rng.chance(0.2) });
        }
    }

    let reply_to = if meta.flags & F_REPLY != 0 && ix > 40 {
        Some(ix - rng.range(2, 36))
    } else {
        None
    };

    let preview = if tb.text.is_empty() {
        SharedString::from("Click to see attachment")
    } else {
        make_preview(&tb.text)
    };

    Content {
        text: tb.text.into(),
        spans: tb.spans,
        preview,
        code,
        image,
        reactions,
        reply_to,
        mentions_me,
    }
}
