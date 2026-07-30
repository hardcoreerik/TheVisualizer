# Roadmap

This document is the authority for milestone status. A milestone is complete only when its exit gate has been run and recorded in [Project Truth](PROJECT_TRUTH.md).

| Milestone | Status | Exit gate |
| --- | --- | --- |
| 1. Documentation and feasibility | In progress | Canonical docs agree; focused WASAPI, GPU, and plugin-loading spikes answer the blocking questions |
| 2. Windows audio foundation | In progress | System loopback and microphone sources feed bounded normalized audio features with measured latency and clear error states |
| 3. Renderer and player | In progress | Eleven bundled modes and shared interaction controls run in windowed, borderless, and fullscreen modes |
| 4. Presets and native plugins | Complete | Format-2 WGSL presets load safely; one explicitly approved example native plugin passes the versioned host-owned lifecycle |
| 5. Windows desktop v0.1 | In progress | Acceptance checks pass on supported Windows hardware and a clean packaged install |
| 6. macOS and Linux | Not started | Native capture backends and packages pass equivalent platform acceptance checks |
| 7. MilkDrop/projectM compatibility | Research only | Format, rendering, licensing, and maintenance options produce a recorded implement/defer decision |
| 8. Standalone embedded visualizers | Deferred until after v0.1 | Exact target hardware independently captures audio and renders a bounded visual demo |

## Milestone 1 — Documentation and feasibility

- Keep verified facts separate from proposals.
- Prove Windows loopback capture and microphone enumeration.
- Render one application-owned WGSL effect through `wgpu` on the available integrated and discrete GPUs.
- Measure an audio-to-frame path before fixing FFT sizes or latency targets.
- Prove discovery and explicit approval of one minimal native library while the host retains rendering ownership.
- Research license compatibility without importing third-party code or presets.

Windows loopback and microphone enumeration/selection, automatic default-device following, the shared analysis path, three visual directions, bounded CPU-painted waveform/spectrum trails, windowed/borderless/fullscreen transitions, resize continuity, safe switching between discovered WGSL presets, and an explicitly approved example native plugin are now proven on the development host and NVIDIA GeForce RTX 5070 Ti. WGSL presets now receive bounded waveform and spectrum buffers, peak, and frame delta as well as the original energy uniforms. The player overlay now uses named visual choices, keeps immediate controls compact, gives source-aware guidance while waiting or silent, clears stale source features on capture replacement, and discloses technical diagnostics and native-extension review on demand, including at the minimum viewport. Automatic capture now retries a backend failure even when the restored default device ID is unchanged; the decision path is tested, while physical no-default removal and restoration remain unverified. The extracted package passed the three-visual acceptance pass for a selected microphone, a local Windows audio source, and an isolated desktop Chrome Web Audio source, including stable silence and recovery without reopening capture. Invalid WGSL retained the last working pipeline. A portable release archive also passed a fresh-directory extraction, checksum, preset, capture, plugin-approval, and latency-telemetry smoke test on that same host. Timestamp instrumentation observed low-single-digit-millisecond average newest-sample-to-feature age during short runs and reports the separate 42.7 ms FFT window. Application-cadence telemetry measured the WGSL visual at about 168–170 FPS on the NVIDIA adapter in windowed and 3440×1440 fullscreen modes, while Microsoft Basic Render Driver fell to about 9–10 FPS windowed and 3 FPS fullscreen; this establishes software rendering as a compatibility probe, not a viable performance baseline. Active default-output changes opened the replacement capture stream in 12.3–14.7 ms after detection and produced a first packet in 23.2–25.4 ms; the one-second poll still leaves event-to-detection uncertainty. The original project work is now Apache-2.0, and the Windows package generates a target-resolved third-party license bundle; MilkDrop/projectM code and unlabeled community presets remain excluded. Windows PnP reports the installed AMD adapter as disabled (Code 22), which explains its absence from wgpu. Integrated-GPU validation, externally timestamped playback-to-display latency, no-replacement device-loss validation, mixed-DPI and multi-monitor behavior, and clean-machine packaging remain open.

The current development tree expands that proven baseline to Neon Scope, Particle Forge, Studio, Visual Canvas, twenty-seven original format-2 WGSL presets, and eight provenance-pinned MIT ISF adaptations: thirty-nine distinct visuals in total. The searchable `L` library groups them by family and supports session favorites. Visual Canvas turns bounded freehand and geometric drawing into selectable, layered, audio-reactive forms over any preset or host-visual background; it intentionally owns a mode-specific right-click workflow. Ten full-screen `Sampled · Visual Fields` modes translate the project's visual references into Pulse Trace, Spectrum Skyline, Radial Burst, Spectrogram City, Spectral Terrain, Wave Tunnel, Particle Ocean, Wireframe Terrain, Halo Spectrum, and Atomic Orbits; the reference PNGs are not runtime assets. Cascading Falls reads shared spectrum history through a host-owned GPU storage buffer for its mirrored SDR-style trace and scrolling false-color waterfall. Event Horizon now layers procedural deep stars, nebulae, gravitational lensing, turbulent Doppler-beamed disk matter, a photon ring, jets, bloom, and film grain. All thirty-five preset packages pass strict discovery and wgpu shader validation; Visual Canvas and the newest original scenes still need live visual and performance acceptance. Named scene save, restart discovery, and cross-mode restore passed an earlier live isolated-directory check; the previous eight-preset portable package snapshot passed its build, staged/fresh checksum, discovery, live-capture, preset-rendering, onset-telemetry, and default-disabled-plugin smoke on the development host. The expanded package remains pending. Clean-machine validation remains open.

Zone Studio now adds one shared GPU alpha-overlay pass above every base visual. Up to eight independent zones can select among ten original procedural visual types and tune band, position, scale, strength, rotation, speed, density, thickness, trails, and color shift. Format-5 scenes persist the complete state and retain formats 1–4 migration. Shader-contract and automated state validation are complete; live visual quality and 3440×1440 performance acceptance remain open.

An experimental Studio slice now adds a bounded Image/GPU Preset/Waveform/Particles layer stack, native local-image drop, and one original photorealistic `Living Photograph` greenhouse composition. Its bundled 49-frame animated WebP supplies real plant, vine, rain, and puddle motion. An optional three-region 2.5D rig color-segments each current frame in memory and bends the left plant, right plant, and hanging growth around independent anchors: bass and mids drive broad movement, treble accelerates leaf detail, and onsets kick short gusts while the table, greenhouse frame, and visible frog remain stable. Optional graphic layers remain available but default off to preserve realism. The still, real-motion, and rigged compositions, layer controls, live/silent behavior, preset switching, camera interaction, windowed rendering, and 3440×1440 fullscreen rendering passed development-host visual checks. The motion revision passed an isolated package build, fresh extraction, and every staged checksum; its extracted executable launched, but packaged motion rendering still needs visual confirmation. Clean-machine validation remains open. Ten further Studio modes are specified in [Interactive Modes](docs/INTERACTIVE_MODES.md); they remain planned.

The development tree now also includes a persistent Performance Studio: two independently tunable WGSL decks, full-frame GPU A/B mixing, scene search/favorites, ten transitions and audio-modulated macros, a ten-layer image/animated-WebP media stack, safe preset live reload, and bounded MIDI plus localhost OSC control. Host modes remain take-only. MP4/video/webcam input, ISF import, spatial transition masks, and external video output remain open.

Native extensions now retain the response-only ABI v1 while adding ABI v2 with 24 bounded host-owned command routes, beat/take events, ten macro inputs, and eight MIDI/localhost-OSC telemetry lanes. One shared Creative Suite DLL supplies eight selectable plugin profiles. Automated ABI, lifecycle, routing, manifest, release-build, package, and checksum validation passed; live behavior and performance acceptance remain open.

## Milestone 2 — Windows audio foundation

- Select the active output loopback or microphone source. Explicit selections remain pinned.
- Follow Windows default-device changes while automatic mode is active and report recovery failures.
- Produce time, waveform, spectrum, RMS, peak, low/mid/high-band values, and one bounded adaptive onset pulse without claiming tempo or BPM.
- Bound cross-thread buffering and report silence, disconnection, permission, and unsupported-format states.
- Report capture-to-feature age separately from the analysis-window duration and external display latency.

## Milestone 3 — Renderer and player

- Implement one host-owned render loop and compact overlay.
- Add oscilloscope/neon trails, spectrum/particles, and feedback-tunnel visuals.
- Support preset switching and windowed, borderless, and fullscreen modes.
- Offer Display, 60 FPS, and 30 FPS pacing with visible measured cadence.
- Keep audio capture alive through normal resize and presentation-mode changes.

## Milestone 4 — Presets and native plugins

- Load declarative WGSL presets with metadata and adjustable parameters.
- Reject invalid or incompatible presets without taking down the player.
- Define a versioned cross-platform C ABI for trusted native plugins.
- Require explicit approval before an unknown native plugin runs.
- Ship one example plugin without exposing direct GPU or window handles.

## Milestone 5 — Windows desktop v0.1

- Pass the acceptance criteria in [v0.1 Scope](docs/V0.1_SCOPE.md).
- Validate integrated and discrete GPU behavior.
- Validate the portable package on a clean machine, including first run, source selection, device loss, fullscreen exit, and folder-based uninstall.
- Truth-sync all public documentation before publishing.

## Later milestones

macOS and Linux use platform-native capture boundaries while sharing analysis, rendering concepts, and preset behavior where measurements permit. MilkDrop/projectM compatibility is a research decision, not an assumed feature. Embedded work is a separate standalone-runtime lane described in [Embedded Vision](docs/EMBEDDED_VISION.md).

## Explicitly deferred

- Media playback
- Preset editor
- Automatic preset rotation
- Marketplace or online account
- Signed-plugin infrastructure
- Legacy Winamp DLL loading
- Sensor, telemetry, or broader network adapters
- Mobile releases
