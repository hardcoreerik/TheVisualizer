# Architecture

This document describes both the implemented prototype foundation and proposed later boundaries. Runtime evidence remains in [Project Truth](../PROJECT_TRUTH.md).

## System boundary

TheVisualizer is one native desktop process for v0.1. It captures an approved local audio source, derives normalized features, and renders a selected visual. It does not intercept media files, modify playback, or provide a media library.

```text
Windows audio source
        |
        v
bounded sample handoff --> analysis snapshot --> preset/plugin parameters
                                                   |
                                                   v
window/input --> player state --> host-owned wgpu renderer --> display
```

## Technology direction

- Rust 2024 edition for the native application and shared logic
- `eframe` 0.35.0 for the application/window lifecycle and lightweight `egui` overlay
- `eframe`'s `wgpu` backend for GPU presentation
- CPAL 0.18.1 for Windows WASAPI loopback and microphone capture
- RustFFT 6.4.1 for the current FFT implementation

`wgpu` is designed to run over native graphics backends including Direct3D 12, Metal, Vulkan, and OpenGL. This supports a portable renderer direction but does not prove platform compatibility for TheVisualizer. See the official [wgpu documentation](https://wgpu.rs/doc/wgpu/).

## Runtime responsibilities

### Capture

The platform capture boundary enumerates sources and produces audio samples plus format and device-state changes. Capture callbacks do no FFT, rendering, shader compilation, plugin loading, disk access, or UI work.

The current Windows prototype enumerates CPAL output and input devices, identifies current defaults by device ID, and opens the selected output as an input stream for loopback capture or the selected microphone as ordinary input. The device list can be refreshed without restarting the current capture.

The System and Microphone shortcuts enter automatic mode. Once per second on the UI thread, the host compares the active device ID with the current Windows default for that source class and reopens capture when they differ. It also reopens after a backend stream error even when Windows restores the same default device ID. A device chosen from the menu enters pinned mode and is not silently replaced by a later default change. Failed automatic recovery keeps the previous stream alive where possible, displays an actionable error, and retries on the next poll. The overlay records detection-to-stream-open and detection-to-first-packet time; first-packet time includes any period in which the new endpoint remains quiet, and the polling design leaves an unknown 0–1000 ms before detection. Physical no-replacement device loss and later same-ID recovery remain unverified.

CPAL's WASAPI backend transparently enables loopback when an output device is opened for input. Microsoft documents loopback capture of the system mix through a render endpoint in shared mode; see [Loopback Recording](https://learn.microsoft.com/en-us/windows/win32/coreaudio/loopback-recording).

### Analysis

Analysis consumes bounded sample blocks and publishes the latest immutable feature snapshot. The snapshot contains time, waveform, spectrum, RMS, peak, and low/mid/high energy. If rendering falls behind, stale intermediate snapshots may be replaced; capture must not wait for the renderer.

The capture callback records CPAL's backend capture/callback timestamps and the local callback instant. Because the backend capture timestamp describes the first frame in a packet, the host subtracts the packet span to estimate the newest sample's age. The UI copies one bounded sample snapshot, releases the capture mutex, performs analysis, and then records the newest-sample-to-feature duration once per observed callback. The overlay reports current, running average, peak, and the separate FFT-window duration. It also reports a smoothed interval between application UI frames and its reciprocal FPS. These are pipeline and application-cadence instruments, not playback-to-display measurements: they exclude upstream application/output buffering, compositor/display scanout, monitor refresh behavior, and human visual response.

Silence produces stable zero or decaying values rather than invalid numbers. Source loss produces a state transition visible to the player.

### Player state

Player state owns the selected source, selected visual, overlay visibility, presentation mode, and status. The default overlay keeps source, preset, response, named Scope/Particles/Preset selection, levels, presentation, and status visible. Capture format, GPU identity, native-extension review, and performance telemetry use one discoverable details disclosure; actionable capture, device, and shader errors remain outside it. Presentation mode is an explicit windowed, borderless, or fullscreen state; `Escape` returns a presentation state to windowed mode before it closes the app. Changing a visual does not restart capture. Resizing or changing presentation mode changes surfaces through the host renderer, not through plugins.

### Rendering

The current host draws the scope and particle visuals with egui painting. One bounded ten-snapshot history adds real waveform trails and attack/release-smoothed spectrum trails without changing the shared feature contract; switching capture sources clears that visual history. The particle view maps low-to-high spectrum order around a radial spiral with a reactive core. The GPU visual uses a discovered WGSL preset, render pipeline, uniform buffers, one fixed-size read-only feature buffer, and an eframe wgpu paint callback. Each frame supplies output size, time, delta, response gain, RMS, peak, low/mid/high energy, the 256-point waveform, and the 64-band spectrum. Preset replacement is compiled under a wgpu validation scope and becomes active only after successful validation; failure keeps the last working pipeline. A CPU-painted tunnel remains the fallback if eframe does not provide a wgpu render state.

The host owns the `wgpu` instance, adapter, device, queue, surfaces, textures, and frame timing. The overlay reports the selected adapter, and an optional `THEVISUALIZER_GPU` environment value can request a named adapter for focused compatibility checks. v0.1 does not expose raw GPU or native window handles to extensions.

### Extensions

The initial single-file `.tvpreset` format provides strict identity, author, license, format-version, bounded response-parameter, and WGSL metadata consumed by the host. The player discovers files from the configured preset directory and reloads them on explicit refresh.

Native plugins use the size-tagged C ABI v1 in the [Plugin SDK](../plugin-sdk/README.md). Discovery parses strict manifests and hashes libraries without loading them. Only `Approve & Load` re-hashes and loads the selected artifact; the host then passes read-only feature slices to a synchronous process callback and clamps its response multiplier before applying it to the host-owned shader. Unload, processing failure, refresh, and app shutdown call the plugin shutdown path. No audio, GPU, window, or frame-lifecycle handle crosses the ABI. The detailed boundary is in [Presets and Plugins](PRESETS_AND_PLUGINS.md).

## Platform path

- Windows: WASAPI loopback and microphone capture for v0.1.
- macOS: investigate Core Audio taps after v0.1. Apple documents system-audio taps for macOS 14.2 and later with user permission; see [Capturing system audio with Core Audio taps](https://developer.apple.com/documentation/coreaudio/capturing-system-audio-with-core-audio-taps).
- Linux: investigate PipeWire capture after v0.1; see the official [PipeWire audio documentation](https://pipewire.pages.freedesktop.org/pipewire/page_audio.html).

Shared analysis and visual concepts should remain portable. Device enumeration, permissions, default-device behavior, sample formats, packaging, and capture recovery remain platform responsibilities.

## Failure behavior

- No active audio: render a stable quiet state and show `Silent`.
- Device removed or default changed: attempt the defined recovery path and show the selected source and outcome.
- Unsupported format: stop that source cleanly and show an actionable error.
- GPU unavailable or surface lost: recover when supported or retain the UI-level error without claiming a fallback renderer.
- Invalid shader: reject the preset and keep the last working visual.
- Native plugin load failure: disable that plugin for the session and retain the player.
- Native plugin crash: v0.1 in-process native code may terminate the process; approval is not isolation.

## Deferred boundaries

Media playback, preset editing, plugin distribution, cloud services, additional data adapters, cross-process plugin isolation, and embedded runtimes are separate milestones.
