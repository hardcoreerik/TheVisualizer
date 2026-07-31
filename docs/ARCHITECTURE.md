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
        |
        +--> local Visual Director brief (no network request)
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

Analysis consumes bounded sample blocks and publishes the latest immutable feature snapshot. The snapshot contains time, waveform, spectrum, RMS, peak, low/mid/high energy, positive spectral flux, and a bounded adaptive onset pulse. The pulse uses a slowly moving transient floor plus hysteresis; it marks isolated attacks for shared visual timing but does not estimate beat position, tempo, or BPM. If rendering falls behind, stale intermediate snapshots may be replaced; capture must not wait for the renderer.

The capture callback records CPAL's backend capture/callback timestamps and the local callback instant. Because the backend capture timestamp describes the first frame in a packet, the host subtracts the packet span to estimate the newest sample's age. The UI copies the latest bounded sample snapshot, releases the capture mutex, and runs analysis only when the callback sequence has advanced. Intermediate callbacks may coalesce into the latest snapshot if rendering falls behind; UI frames without new audio reuse the current immutable feature set. Capture replacement resets the sequence gate, adaptive onset state, and onset counters. The host records newest-sample-to-feature duration once per analyzed callback. The collapsed Details section reports current/running-average/peak feature age, separate FFT-window duration, onset count and elapsed rate, time since the last onset, current flux, and a smoothed application-frame interval plus reciprocal FPS. These are pipeline, detector, and application-cadence instruments, not playback-to-display or beat measurements: they exclude upstream application/output buffering, compositor/display scanout, monitor refresh behavior, and human visual response.

Silence produces stable zero or decaying values rather than invalid numbers. Source loss produces a state transition visible to the player.

### Player state

Player state owns the selected source, selected visual, overlay visibility, presentation mode, frame pacing, and status. The default overlay keeps source, preset, response, numbered Scope/Particles/Preset selection, levels, presentation, and status visible. Number keys select each visual directly while Left/Right cycles them; the shortcut footer uses portable ASCII rather than depending on optional font glyphs. Waiting, silence, and capture failure add a concise source-aware next step; live capture removes that guidance. Capture replacement clears the previous source's features and visual history before waiting for new packets. Capture format, GPU identity, native-extension review, performance telemetry, and Display/60/30 FPS pacing use one discoverable details disclosure; actionable capture, device, and shader errors remain outside it. Presentation mode is an explicit windowed, borderless, or fullscreen state; `Escape` returns a presentation state to windowed mode before it closes the app. Changing a visual does not restart capture. Resizing or changing presentation mode changes surfaces through the host renderer, not through plugins.

Named `.tvscene` snapshots persist host-owned creative state without changing preset packages. A strict bounded parser records the required mode identity, relevant parameter values, routed colors/materials, camera, and at most eight zones. Restore selects the required host mode or declarative preset, rejects missing dependencies, clamps parameter values against current preset metadata, and then applies the snapshot. Scene files contain values only and cannot introduce WGSL or native code. The experimental Studio stack is not yet part of this scene format.

### Rendering

The current host draws the scope and particle visuals with egui painting. One bounded ten-snapshot history adds real waveform trails and attack/release-smoothed spectrum trails without changing the shared feature contract; switching capture sources clears that visual history and the onset detector. The particle view maps low-to-high spectrum order around a radial spiral with a reactive core. The GPU visual uses a discovered WGSL preset, render pipeline, uniform buffers, and fixed-size read-only feature, scene, and parameter buffers through an eframe wgpu paint callback. Each frame supplies output size, time, delta, response gain, RMS, peak, transient intensity, onset pulse, low/mid/high energy, the 256-point waveform, the 64-band spectrum, up to eight sound zones, camera state, routed colors/materials, and up to forty mode controls. The expanded frame-extras buffer preserves its original first four fields for existing format-2 shaders. Preset replacement is compiled under a wgpu validation scope and becomes active only after successful validation; failure keeps the last working pipeline. A CPU-painted tunnel remains the fallback if eframe does not provide a wgpu render state.

The experimental Studio reuses the existing GPU-preset callback plus the host-painted waveform and particle paths inside a bounded layer list and adds one host-owned image texture. One opaque GPU preset can act as the bottom layer; lowering the image opacity exposes it while host-painted layers remain above. Local desktop drops are decoded on the UI path with file, dimension, and allocation limits; capture callbacks and the shared analysis path do no image or disk work. The first slice uses direct egui compositing with Normal and additive paint colors. It has no offscreen effect graph, persistent GPU particles, network source, or in-player generator.

The host owns the `wgpu` instance, adapter, device, queue, surfaces, textures, and frame timing. Display pacing follows the presentation cadence; the 60 and 30 FPS choices apply a host-side minimum frame interval and retain the existing immutable-feature/coalescing behavior. The overlay reports both the choice and measured application cadence. It also reports the selected adapter, and an optional `THEVISUALIZER_GPU` environment value can request a named adapter for focused compatibility checks. v0.1 does not expose raw GPU or native window handles to extensions.

### Extensions

The single-file `.tvpreset` format provides strict identity, author, license, format-version, bounded parameter, and WGSL metadata consumed by the host. Format 1 retains the original response-only contract; format 2 adds grouped controls and fixed scene/parameter bindings. The player discovers files from the configured preset directory and reloads them on explicit refresh.

Native plugins use the size-tagged C ABI v1 or v2 in the [Plugin SDK](../plugin-sdk/README.md). Discovery parses strict manifests and hashes libraries without loading them. Only `Approve & Load` re-hashes and loads the selected artifact. ABI v1 returns one clamped response multiplier. ABI v2 receives read-only audio, onset/transient, macro, and external-lane values and returns at most 24 commands plus bounded beat/take flags; every target, index, operation, and value passes through existing host-owned setters and ranges. Unload, processing failure, refresh, replacement, and app shutdown call the plugin shutdown path. Only one plugin is active, and no audio, GPU, window, capture, or frame-lifecycle handle crosses either ABI. The detailed boundary is in [Presets and Plugins](PRESETS_AND_PLUGINS.md).

### Visual Director

The implemented local Visual Director augments the normalized feature snapshot with spectral centroid, 85% rolloff, flatness, crest factor, and positive spectral flux. A bounded 12-second history derives movement, onset density, dynamic contrast, and passage direction. It combines that Visual DNA with the active mode, routed colors, material finish, capture profile, and explicit user controls.

Scene invention independently selects subject, environment, era, event, scale, weather, composition, and lighting. It scores several candidates against a bounded structured history, weighting repeated subject and location more strongly than repeated surface treatment. A versioned append-only local file preserves the last loaded concept records across application restarts and records the generation metadata and full prompt; malformed records are skipped without blocking the player. The director then selects a small plausible subset of photographic imperfections, applies contextual cliché constraints, and produces the scene/camera/light/material/motion specification and generation prompt. A deterministic preflight reports concept novelty, physical plausibility, cliché risk, and construction notes. It does not infer unmeasured tempo, BPM, key, genre, lyrics, or mood, and it makes no network request.

The proposed first image provider is the OpenAI Image API using `gpt-image-2`, behind an explicit user action and environment-supplied credential. That API connection, response decoding, generated-image persistence, critique, and generated-asset ingestion are not implemented. See [Visual Director](VISUAL_DIRECTOR.md).

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

Media playback, preset editing, plugin distribution, paid image generation, Studio composition persistence, offscreen layer effects, cloud services, additional data adapters, cross-process plugin isolation, and embedded runtimes are separate milestones.
