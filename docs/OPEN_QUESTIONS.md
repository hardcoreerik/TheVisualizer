# Open Questions

Resolve these questions with official documentation, focused code spikes, measured runtime results, or recorded licensing review. Do not turn assumptions into facts by repetition.

## Windows capture

- Which Windows versions form the supported v0.1 minimum?
- How should the automatic default-output choice react when Windows changes the default endpoint?
- What happens when an endpoint disappears, sleeps, changes format, or remains silent?
- Can microphone and loopback capture share one internal sample contract without hiding important device behavior?
- What audio-to-feature latency is achievable on representative integrated and discrete GPU systems?

## Analysis

- What sample-block size, FFT size, overlap, window function, and smoothing produce stable visuals at acceptable latency?
- How should low, mid, and high bands scale across sample rates?
- Which beat or onset feature, if any, earns a place beyond the v0.1 energy inputs?

## Rendering and player

- What minimum GPU and driver capabilities can be supported honestly?
- Is a non-GPU fallback necessary after hardware testing?
- How do `winit`, `wgpu`, and `egui` behave across resize, sleep/wake, HDR, mixed-DPI monitors, and fullscreen changes?
- Should fullscreen target the current monitor or a user-selected monitor in v0.1?
- Do Display, 60 FPS, and 30 FPS remain the right choices after representative GPU board-power, thermal, and battery measurements?

## Presets and native plugins

- Which non-slider parameter types or additional feature bindings earn a place beyond the fixed format-2 scene and control buffers?
- Which `.tvscene` migration, naming, deletion, sharing, and provenance features are justified after real saved-view usage?
- Which shader limits prevent accidental or hostile resource exhaustion?
- Is in-process crash risk acceptable after the example plugin, or is a separate host justified?
- How should native libraries be packaged on future operating systems and architectures?

## Compatibility

- Which MilkDrop preset versions and behaviors matter to prospective users?
- Can projectM provide useful compatibility without dictating the renderer or license?
- Should compatibility use conversion, interpretation, an adapter process, or remain unsupported?

## macOS and Linux

- What macOS minimum follows from Core Audio tap requirements and permissions?
- Which Linux distributions, PipeWire versions, display servers, and packaging formats are supportable?
- How do platform permission, device-switching, and system-mix semantics differ from Windows?
- Which platform-specific failures need visible user guidance?

## Future input adapters

- Which concrete MIDI, OSC, sensor, telemetry, or network use case should be first?
- Do future values join the audio feature snapshot or use a separately timed input map?
- Which discovery, authentication, and rate limits are required by the chosen input?

## Embedded

- Which exact ESP32-class boards, M5Tab5 revision, microphones/codecs, displays, and LED controllers are available for testing?
- Which SDKs, graphics APIs, memory budgets, and audio peripherals are verified on each target?
- What analysis and frame rates fit within measured CPU, memory, thermal, and power limits?
- Which visual concepts degrade gracefully without GPU feedback rendering?
- Does any measured use case justify desktop-to-embedded feature streaming?

## Resolved

- Windows desktop is the first release target.
- v0.1 is a player, not a media player or editor.
- System loopback and microphone are the v0.1 inputs.
- CPAL 0.18.1 successfully provided default-output WASAPI loopback and default-microphone capture on the first Windows development host.
- CPAL device IDs support explicit selection of enumerated output-loopback and microphone endpoints; four outputs and two microphones were exercised on the first development host.
- One-second device-ID probing successfully followed a real Windows default-output change from Speakers to HP X34 and back; explicit device selection remained pinned.
- With continuous sound, post-detection recovery opened HP X34 in 12.3 ms and produced a first packet in 25.4 ms; restoring Speakers took 14.7 ms and 23.2 ms respectively. The one-second poll still leaves 0–1000 ms of event-to-detection uncertainty, and quiet endpoints can delay the first packet independently of stream-open time.
- The visual identity is modern-retro.
- Shader presets and explicitly trusted native plugins are separate extension tiers.
- The host owns rendering and does not expose direct GPU or window handles in the v0.1 plugin ABI.
- Embedded research begins after desktop v0.1 and targets standalone operation first.
- An application-owned WGSL callback renders and responds to normalized audio features on the NVIDIA GeForce RTX 5070 Ti in windowed and fullscreen modes.
- Windowed, borderless, fullscreen, restore, and maximize-resize transitions preserved the host-owned WGSL renderer on the first development host; system capture also remained live during fullscreen rendering.
- A bounded single-file `.tvpreset` format discovers eight bundled format-2 presets with 10–35 grouped sliders; an invalid WGSL probe was rejected while the last working pipeline remained usable.
- WGSL presets now receive bounded 256-point waveform and 64-band spectrum buffers plus peak and frame delta, in addition to the original scalar uniform contract. All eight bundled presets compiled in the live player; the richer data path was visibly exercised in windowed and fullscreen validation.
- ABI v1 uses size-tagged initialize/process/shutdown callbacks, call-scoped read-only feature pointers, and one host-clamped response output with no GPU/window handles.
- Native-plugin approval is session-only and tied to the full SHA-256 digest; discovery and restart leave the example disabled, and approval is explicitly not a signature or sandbox.
- Windows x86_64 local testing uses one portable ZIP with package-relative plugin paths and SHA-256 checksums; a fresh-directory smoke test passed on the development host.
- CPAL/WASAPI capture timestamps now drive newest-sample-to-feature telemetry. Short 48,000 Hz runs on the first host showed low-single-digit-millisecond averages while the separate 2,048-sample FFT window measured 42.7 ms; external playback-to-display latency remains unresolved.
- The UI snapshots the latest bounded sample buffer and analyzes only when the capture callback sequence advances. Slow rendering may coalesce intermediate callbacks; faster rendering reuses the latest immutable feature set instead of rerunning the FFT on stale samples. Capture replacement resets the gate.
- TheVisualizer's original code, documentation, bundled presets, plugin SDK, and example plugin use Apache-2.0. No MilkDrop/projectM code or community presets are included. Future third-party visual content requires explicit per-artifact provenance; unlabeled community presets are not assumed to be public domain.
- The installed AMD GPU was not enumerated by wgpu during a forced-adapter check. Windows PnP reports that adapter as `CM_PROB_DISABLED` (Code 22), explaining why application-level selection cannot use it; integrated-GPU rendering remains unvalidated.
- Smoothed application-frame telemetry measured the NVIDIA GeForce RTX 5070 Ti at about 168–170 FPS and 5.9–6.0 ms per frame for the WGSL visual in windowed and 3440×1440 fullscreen modes. This measures UI cadence, not monitor scanout or playback-to-photon latency.
- Microsoft Basic Render Driver rendered Feedback Tunnel with live audio, but measured only about 9–10 FPS windowed and about 3 FPS at 3440×1440 fullscreen. This proves a software-adapter compatibility path, not acceptable minimum performance.
- The player now offers Display, 60 FPS, and 30 FPS pacing beside measured cadence. In one quiet extracted-release comparison, limiting a ~167 FPS Feedback Tunnel reduced process CPU use from 28.1% of one core to 6.7% at both 60 and 30 FPS; GPU board power, thermals, and battery impact remain unmeasured.
- The current host exposes one active 3440×1440 HP X34 at 96 DPI, so mixed-DPI and multi-monitor validation requires another environment.
