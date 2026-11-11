// wincap: record ONE window via ScreenCaptureKit (no desktop, no notifications, works while covered)
// usage: wincap list | wincap rec <app-or-title substring> <seconds> <out.mp4> [fps] [scale]
import Foundation
import ScreenCaptureKit
import AVFoundation
import AppKit
_ = NSApplication.shared

final class Del: NSObject, SCRecordingOutputDelegate, SCStreamDelegate {
  func recordingOutputDidStartRecording(_ r: SCRecordingOutput) { FileHandle.standardError.write("rec started\n".data(using: .utf8)!) }
  func recordingOutput(_ r: SCRecordingOutput, didFailWithError e: Error) { FileHandle.standardError.write("rec error \(e)\n".data(using: .utf8)!) }
  func stream(_ s: SCStream, didStopWithError e: Error) { FileHandle.standardError.write("stream error \(e)\n".data(using: .utf8)!) }
}

let a = CommandLine.arguments
let content = try await SCShareableContent.excludingDesktopWindows(true, onScreenWindowsOnly: false)
let wins = content.windows.filter { $0.windowLayer == 0 && $0.frame.width > 200 }
if a.count < 2 || a[1] == "list" {
  for w in wins { print(w.windowID, w.owningApplication?.applicationName ?? "?", "|", w.title ?? "", "|", Int(w.frame.width), "x", Int(w.frame.height), w.isOnScreen ? "on" : "off") }
  exit(0)
}
if a[1] == "shotpid" {
  let pid = pid_t(a[2])!, outp = URL(fileURLWithPath: a[3])
  guard let w = wins.filter({ $0.owningApplication?.processID == pid }).sorted(by: { $0.frame.width * $0.frame.height > $1.frame.width * $1.frame.height }).first else { print("no window for pid \(pid)"); exit(1) }
  let c = SCStreamConfiguration(); c.width = Int(w.frame.width * 2); c.height = Int(w.frame.height * 2); c.showsCursor = false; c.ignoreShadowsSingleWindow = true
  let img: CGImage = try await withCheckedThrowingContinuation { k in
    SCScreenshotManager.captureImage(contentFilter: SCContentFilter(desktopIndependentWindow: w), configuration: c) { im, e in
      if let im { k.resume(returning: im) } else { k.resume(throwing: e ?? NSError(domain: "wincap", code: 1)) } } }
  let rep = NSBitmapImageRep(cgImage: img)
  try rep.representation(using: .png, properties: [:])!.write(to: outp)
  print(outp.path); exit(0)
}
let q = a[2].lowercased(), secs = Double(a[3])!, out = URL(fileURLWithPath: a[4])
let fps = a.count > 5 ? Int32(a[5])! : 60, scale = a.count > 6 ? Double(a[6])! : 2
guard let w = wins.sorted(by: { $0.frame.width * $0.frame.height > $1.frame.width * $1.frame.height }).first(where: { ($0.owningApplication?.applicationName.lowercased().contains(q) ?? false) || ($0.title?.lowercased().contains(q) ?? false) }) else { print("no window matching \(q)"); exit(1) }
print("recording", w.windowID, w.owningApplication?.applicationName ?? "", w.title ?? "")
let cfg = SCStreamConfiguration()
cfg.width = Int(w.frame.width * scale); cfg.height = Int(w.frame.height * scale)
cfg.minimumFrameInterval = CMTime(value: 1, timescale: fps)
cfg.showsCursor = false; cfg.capturesAudio = false; cfg.queueDepth = 8
cfg.ignoreShadowsSingleWindow = true
try? FileManager.default.removeItem(at: out)
let rc = SCRecordingOutputConfiguration(); rc.outputURL = out; rc.outputFileType = .mov; rc.videoCodecType = .hevc
let d = Del()
let stream = SCStream(filter: SCContentFilter(desktopIndependentWindow: w), configuration: cfg, delegate: d)
let ro = SCRecordingOutput(configuration: rc, delegate: d)
try stream.addRecordingOutput(ro)
try await stream.startCapture()
try await Task.sleep(nanoseconds: UInt64(secs * 1e9))
try await stream.stopCapture()
try await Task.sleep(nanoseconds: 500_000_000)
print("saved", out.path)
