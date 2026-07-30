# Open Questions

Resolve these questions with official documentation, focused code spikes, measured runtime results, or recorded licensing review. Do not turn assumptions into facts by repetition.

## Still open

### Windows capture and latency

- Which Windows versions form the supported v0.1 minimum?
- What happens when an endpoint disappears, sleeps, changes format, or remains silent with **no** replacement default?
- What externally timestamped playback-to-display latency is achievable on representative integrated and discrete GPU systems?
- Is the automatic default-device poll's unmeasured 0–1000 ms event-to-detection interval acceptable for v0.1 release notes?

### Analysis

- Does the implemented adaptive onset pulse remain musically useful across genres, levels, capture devices, and callback cadences, or do measured false hits require revising its transient floor and hysteresis?
- Should low, mid, and high band boundaries be retuned after multi-rate field measurements beyond 44.1/48 kHz synthetic checks?

### Rendering and player

- What minimum GPU and driver capabilities can be supported honestly after integrated-GPU and clean-machine runs?
- How do `winit`, `wgpu`, and `egui` behave across sleep/wake, HDR, mixed-DPI monitors, and multi-monitor fullscreen selection?
- Should fullscreen target the current monitor or a user-selected monitor in v0.1?
- Do Display, 60 FPS, and 30 FPS remain the right choices after GPU board-power, thermal, and battery measurements?
- Does the bounded Studio stack remain readable and responsive across image aspect ratios and representative music before composition persistence is expanded?
- Which offscreen blend or effect earns implementation after direct Normal/Add composition is more broadly measured?
- Live visual quality and 3440×1440 performance for Zone Studio, Visual Canvas, Event Horizon motion clips, and every Creative Suite profile.

### Presets and native plugins

- Which non-slider parameter types or additional feature bindings earn a place beyond the fixed format-2 buffers?
- Which `.tvscene` rename, delete, share, and provenance features are justified after real saved-view usage?
- Which shader limits prevent accidental or hostile resource exhaustion beyond current size and validation bounds?
- Is in-process crash risk acceptable beyond the repository-owned plugins, or is a separate host justified?
- How should native libraries be packaged on future operating systems and architectures?

### Compatibility

- Which MilkDrop preset versions and behaviors matter to prospective users?
- Can projectM provide useful compatibility without dictating the renderer or license?
- Should compatibility use conversion, interpretation, an adapter process, or remain unsupported?

### macOS and Linux

- What macOS minimum follows from Core Audio tap requirements and permissions?
- Which Linux distributions, PipeWire versions, display servers, and packaging formats are supportable?
- How do platform permission, device-switching, and system-mix semantics differ from Windows?
- Which platform-specific failures need visible user guidance?

### Future input adapters

- Which concrete sensor, telemetry, or non-localhost network use case should be first after MIDI/localhost OSC?
- Do future values join the audio feature snapshot or use a separately timed input map?
- Which discovery, authentication, and rate limits are required by the chosen input?

### Embedded

- Which exact ESP32-class boards, M5Tab5 revision, microphones/codecs, displays, and LED controllers are available for testing?
- Which SDKs, graphics APIs, memory budgets, and audio peripherals are verified on each target?
- What analysis and frame rates fit within measured CPU, memory, thermal, and power limits?
- Which visual concepts degrade gracefully without GPU feedback rendering?
- Does any measured use case justify desktop-to-embedded feature streaming?

### Experimental AI lane (not v0.1 exit)

- Does the offline AI Pack / worker path meet local safety, license, and hash gates without affecting capture or render?
- When, if ever, should generated assets be ingestible as Studio layers?

## Resolved

- Windows desktop is the first release target.
- v0.1 is a player, not a media player or editor.
- System loopback and microphone are the v0.1 inputs.
- CPAL 0.18.1 provided default-output WASAPI loopback and default-microphone capture on the first Windows development host.
- CPAL device IDs support explicit selection of enumerated output-loopback and microphone endpoints; four outputs and two microphones were exercised on the first development host.
- One-second device-ID probing followed a real Windows default-output change from Speakers to HP X34 and back; explicit device selection remained pinned.
- With continuous sound, post-detection recovery opened HP X34 in 12.3 ms and produced a first packet in 25.4 ms; restoring Speakers took 14.7 ms and 23.2 ms respectively. Quiet endpoints can delay the first packet independently of stream-open time.
- Automatic capture retries a backend failure even when the restored default device ID is unchanged (decision path tested; physical no-default removal remains open above).
- Microphone and loopback share one internal sample contract with source-specific format display and feature reset on replacement.
- The visual identity is modern-retro.
- Shader presets and explicitly trusted native plugins are separate extension tiers.
- The host owns rendering and does not expose direct GPU or window handles in the v0.1 plugin ABI.
- Embedded research begins after desktop v0.1 and targets standalone operation first.
- An application-owned WGSL callback renders and responds to normalized audio features on the NVIDIA GeForce RTX 5070 Ti in windowed and fullscreen modes.
- Windowed, borderless, fullscreen, restore, and maximize-resize transitions preserved the host-owned WGSL renderer on the first development host; system capture also remained live during fullscreen rendering.
- A bounded single-file `.tvpreset` format discovers format-2 presets with 10–35 grouped sliders; invalid WGSL is rejected while the last working pipeline remains usable.
- WGSL presets receive bounded 256-point waveform and 64-band spectrum buffers plus peak, frame delta, onset, and transient intensity in addition to scalar energy uniforms.
- Analysis uses a 2,048-sample Hann-windowed FFT; UI analyzes only when the capture callback sequence advances.
- ABI v1 retains size-tagged initialize/process/shutdown callbacks and one host-clamped response output. ABI v2 adds call-scoped onset/transient/controller input, 24 bounded host-owned commands, and beat/take flags without GPU/window handles.
- Native-plugin approval is session-only and tied to the full SHA-256 digest; discovery and restart leave plugins disabled; approval is not a signature or sandbox.
- Windows x86_64 local testing uses one portable ZIP with package-relative plugin paths and SHA-256 checksums; same-host fresh-directory smokes have passed (clean-machine remains open).
- CPAL/WASAPI capture timestamps drive newest-sample-to-feature telemetry: short 48,000 Hz runs showed low-single-digit-millisecond averages with a separate 42.7 ms FFT window.
- TheVisualizer original code, documentation, bundled presets, plugin SDK, and example plugins use Apache-2.0. No MilkDrop/projectM code or community presets are included. Future third-party visual content requires explicit per-artifact provenance.
- The installed AMD GPU was not enumerated by wgpu during a forced-adapter check because Windows PnP reports `CM_PROB_DISABLED` (Code 22).
- Smoothed application-frame telemetry measured NVIDIA at about 168–170 FPS / 5.9–6.0 ms for WGSL windowed and 3440×1440 fullscreen. Microsoft Basic Render Driver proved compatibility only (~9–10 / ~3 FPS).
- The player offers Display, 60 FPS, and 30 FPS pacing beside measured cadence; quiet CPU samples on one host dropped from 28.1% of one core at Display to 6.7% at 60 and 30 FPS.
- The current host exposes one active 3440×1440 HP X34 at 96 DPI, so mixed-DPI and multi-monitor validation requires another environment.
- Performance Studio implements bounded MIDI CC/note mapping and localhost-only OSC under `/thevisualizer` as host-owned external control, not a generic sensor framework.
- Eight MIT ISF adaptations are provenance-pinned; the other 319 reviewed ISF shaders remain excluded for unsupported multi-pass/texture requirements.
