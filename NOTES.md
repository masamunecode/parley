# Parley — notes

Parley is an offline, Discord-style community chat client written in Rust with
GPUI 0.2.2 (crates.io, `runtime_shaders` feature). It is UI only: every server,
channel, member and message is generated locally at start-up. The name, the
two-speech-bubble mark, the amber-on-ink palette and all icons (hand-written SVG
strokes) are original.

## Running

```
./snap.sh [seconds] [flags]     # build (dev profile, fully optimized), launch unfocused, screenshot
target/debug/parley             # interactive
target/debug/parley --demo      # self-playing tour, ~45 s, loops
target/debug/parley --hud       # fps / frame time / draw time / worst frame / memory overlay
```

Flags combine (`--demo --hud`). Extra start-up flags that are handy for
screenshots: `--server=N --channel=N --at=MESSAGE_INDEX --switcher=QUERY`.

`SNAP=1` opens the window without activating the app or taking focus.

## What works

- **Server rail** — round gradient icons that square off on hover/selection, left
  pill indicator (unread dot / hover / selected), red mention badges, name
  tooltip, home / add / discover buttons. 9 servers.
- **Channel list** — collapsible categories, text and announcement channels,
  unread dots, mention badges, hover and selected states, one voice channel per
  server with 3 participants whose rings pulse while they "speak".
- **User panel** — avatar with status badge, name, status line, working mute and
  deafen toggles, settings button.
- **Chat** — `#general` in the first server holds exactly 100,000 generated
  messages (other channels 14–6,400). The list is GPUI's virtualized `list`:
  only visible rows plus a 1,000 px overdraw are built and measured. Messages
  are stored as 12-byte metas and their content is regenerated from a seed when
  a row scrolls into view (small LRU-ish cache). Rows have author grouping,
  hover timestamps, day dividers, role-coloured names, BOT tags, @mentions
  (with highlighted rows when you are mentioned), inline code, links, "(edited)",
  syntax-tinted code blocks, image placeholders, reactions, replies with a
  preview line, and a hover toolbar.
- **Scrollbar** — custom thumb mapped to message index, draggable across the
  whole history with a date / index label; "Jump to present" pill when scrolled
  away; PageUp / PageDown; Esc jumps to present.
- **Live activity** — in `#general` people start typing (animated indicator) and
  their messages arrive every few seconds; the list stays pinned to the bottom
  unless you have scrolled up.
- **Composer** — type, Enter sends, the message appears at the bottom. Caret
  movement (arrows, alt/cmd variants, Home/End), Backspace/Delete, Cmd+V paste.
  `@name`, `` `code` `` and links are styled in sent messages.
- **Member list** — 1,000 members in the main server (subsets elsewhere),
  grouped by role then Online / Offline, status badges (online, idle, DND,
  offline), activity lines, virtualized with `uniform_list`. Toggle with the
  people icon in the header.
- **Quick switcher** — Cmd+K (or Ctrl+K): fuzzy subsequence search over every
  channel and server with matched letters highlighted, arrows / Tab to move,
  Enter or click to open, Esc or click outside to close. Empty query shows
  unread channels first.
- **Voice view** — clicking the voice channel shows a stage with three tiles,
  animated speaking rings, expanding pulses, level meters and call controls.
- **Switching** — channel and server switches are immediate; each channel keeps
  its own scroll position.
- **`--demo`** — scripted tour with a drawn cursor and a caption in the title
  bar: fling-scrolls the 100k list, scrubs the scrollbar through the whole
  history, jumps to present, types and sends a message, opens the voice
  channel, switches channels and servers, uses Cmd+K with fuzzy queries,
  scrolls the member list, toggles mute, then loops.
- **`--hud`** — FPS, average frame interval, draw time (render + layout + paint
  on the main thread), worst frame of the last 120, resident memory (same number
  as `ps -o rss`), and a frame-time graph.

Measured on this machine with the dev profile used by `snap.sh` (opt-level 3):
about 2.5–4.5 ms of draw time per frame, including while flinging and scrubbing
the 100k channel. The HUD read 120 fps when the window was covered and frames
came from the 120 Hz timer pump, and 60 fps (16.67 ms) when the window was
visible and vsync-driven by the display. Resident memory was ~105–150 MB. The
"worst" spike at the right edge of the HUD graph in screenshots coincides with
the screenshot capture itself.

## What doesn't / caveats

- **The demo does not inject OS-level mouse events.** GPUI 0.2.2 keeps the
  return type of `Window::dispatch_event` private, so synthetic mouse events
  can't be dispatched from outside the crate. Keystrokes in the demo do go
  through the real key path (`Window::dispatch_keystroke`); clicks, drags and
  flings call the same handlers the mouse listeners call, and hover under the
  drawn cursor is simulated from element bounds recorded during paint. The demo
  layer swallows real mouse input while it plays.
- **Real mouse / trackpad interaction was not exercised by me.** I could only
  look at the app through `snap.sh` screenshots (unfocused window), so clicking,
  wheel scrolling, scrollbar dragging and typing on a physical keyboard are
  verified only indirectly (same handlers as the demo, standard GPUI listeners).
- **Occluded windows.** GPUI stops its display link when macOS reports the
  window as occluded, which is what happens under `snap.sh`. `src/pump.rs`
  drives frames from a 120 Hz timer in that case, or when no frame has been
  rendered for 60 ms (only when `SNAP=1` or `--demo`), so HUD numbers in a
  covered window are timer-paced, not vsync-paced. One screenshot out of ~45
  launches came back black once, before the 60 ms stall check was added; I
  could not reproduce it, so I can't confirm the stall check is what fixes it.
- The whole window re-renders every frame because of the time-based animations
  (speaking rings, typing dots). It stays well inside the frame budget, but idle
  CPU use is not zero.
- Text input is a simple single-line field: no selection, no IME / dead-key
  composition, no multi-line (Shift+Enter does nothing), no emoji picker.
- No text selection or copy in messages; links and mentions are not clickable.
- Decorative only: header icons other than the member toggle, search box,
  reactions, the message hover toolbar, add/discover server, settings, the gift
  / sticker / emoji buttons, the voice-stage video / screen / hang-up buttons.
- Voice is a visual simulation; "you" never join the channel.
- Images are generated placeholders; there are no real image assets or network
  access of any kind.
- macOS only (the HUD memory reading and the frame pump use macOS APIs).

## Layout of the code

- `src/main.rs` — flags, window options.
- `src/app.rs` — root view state, input handling, top-level layout, title bar.
- `src/sidebar.rs` — server rail, channel list, user panel.
- `src/chat.rs` — header, virtualized message list, composer, member list.
- `src/switcher.rs` — Cmd+K quick switcher.
- `src/voice.rs` — voice stage.
- `src/data.rs` — generated world, clock helpers, message content generator.
- `src/ui.rs` — icons, avatars, status badges, text field, fuzzy matcher.
- `src/demo.rs` — scripted tour. `src/hud.rs` — overlay. `src/pump.rs` — frame
  pump for occluded windows. `src/icons.rs` — embedded SVG icons.
  `src/theme.rs` — colours and layout constants.
