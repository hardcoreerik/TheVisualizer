# Initial Architecture Decisions

These are project decisions, not verified implementation results. Supersede decisions explicitly instead of silently rewriting history after implementation begins.

## D-001 — Windows-first desktop delivery

**Decision:** Deliver and validate Windows desktop v0.1 before macOS and Linux packages.

**Rationale:** The current development environment can directly validate WASAPI loopback, while platform capture and packaging still remain distinct work.

## D-002 — Native Rust player

**Decision:** Use Rust with `winit`, `wgpu`, and a lightweight `egui` overlay, subject to feasibility spikes.

**Rationale:** One native stack can own real-time boundaries, GPU rendering, and later desktop portability without bundling a media player or browser runtime.

## D-003 — Player-first modern-retro experience

**Decision:** Prioritize source selection, visual selection, compact controls, and windowed/borderless/fullscreen playback with Winamp-era visual energy and a clean modern overlay.

**Rationale:** A preset editor and media library would delay the primary audio-reactive experience.

## D-004 — System audio and microphone for v0.1

**Decision:** Treat loopback output and microphone capture as the two v0.1 source classes.

**Rationale:** This covers computer playback and live audio without turning TheVisualizer into a media player.

## D-005 — One normalized visual-feature path

**Decision:** Built-ins, presets, and plugins consume the same conceptual time, waveform, spectrum, RMS, peak, and low/mid/high features.

**Rationale:** A shared path prevents each visual type from reimplementing capture and analysis.

## D-006 — Future input adapters remain deferred

**Decision:** Preserve room for MIDI, OSC, sensors, telemetry, and network values without defining or implementing them in v0.1.

**Rationale:** Their schemas and timing requirements should come from concrete devices and use cases.

## D-007 — Dual extension model

**Decision:** Support declarative WGSL presets and a new versioned cross-platform C ABI for native plugins.

**Rationale:** Presets cover portable visual content, while trusted native plugins permit bounded CPU-side behavior without making every visual native code.

## D-008 — Host-owned rendering

**Decision:** The host owns audio capture, windows, GPU resources, and frame lifecycle. v0.1 plugins process normalized features and drive host-rendered shaders without receiving raw GPU or window handles.

**Rationale:** This is the smallest useful native boundary that avoids locking the ABI to one graphics backend or window system.

## D-009 — Explicit trust for native plugins

**Decision:** Unknown native plugins remain disabled until the user approves the exact discovered artifact.

**Rationale:** Native libraries run with the user's privileges. Approval communicates trust but is not a sandbox or security guarantee.

## D-010 — MilkDrop and Winamp compatibility are separate

**Decision:** Do not make `.milk`, projectM, or legacy Winamp DLL compatibility a v0.1 requirement.

**Rationale:** Compatibility, rendering semantics, maintenance cost, and licensing need evidence before they shape the core.

## D-011 — License decision gate

**Decision:** Do not add a project license until MilkDrop, projectM, plugin SDK, and third-party preset licensing are researched.

**Rationale:** A public license should be compatible with the implementation and content strategy rather than guessed first.

**Status:** Superseded by D-015 after the 2026-07-27 licensing review.

## D-012 — Embedded is a separate post-v0.1 runtime

**Decision:** Explore standalone ESP32-class display, M5Tab5, and LED-controller visualizers only after desktop v0.1.

**Rationale:** Embedded targets have different audio, memory, shader, display, and deployment constraints. Shared concepts do not imply shared binaries or WGSL support.

## D-013 — Native approval is session-only and byte-bound

**Decision:** Native-plugin approval in v0.1 lasts only for the current process and is tied to the library's full SHA-256 digest, which is recomputed immediately before loading.

**Rationale:** This proves explicit approval of the reviewed artifact without inventing persistent trust storage or implying that a checksum is signing, sandboxing, or crash isolation.

## D-014 — Prove packaging with a portable local-test archive

**Decision:** Use a checksummed Windows x86_64 ZIP for local packaging and clean-machine validation before deciding whether v0.1 needs an installer.

**Rationale:** Extraction and folder deletion are enough to validate the complete runtime layout without adding installer, signing, or update infrastructure before those requirements are known.

## D-015 — Apache-2.0 for original project work

**Decision:** License TheVisualizer's original code, documentation, bundled WGSL presets, plugin SDK, and example plugin under Apache-2.0. Do not import or bundle MilkDrop/projectM code, community presets, or textures without an artifact-level license and provenance record.

**Rationale:** Apache-2.0 is a permissive open-source license with an explicit patent grant. The current implementation is original and its resolved Rust dependencies declare compatible license choices. projectM's core is LGPL-2.1-or-later, while major community preset collections acknowledge that most authors supplied no explicit license; neither should silently determine the license of this repository.

## D-016 — Explicit frame pacing

**Decision:** Follow the presentation cadence by default and offer optional 60 FPS and 30 FPS host-side limits in the technical disclosure.

**Rationale:** High-refresh displays preserve maximum fluidity by default, while explicit lower limits give users a direct cadence/CPU tradeoff without changing capture, analysis, preset, or plugin contracts. Measured application cadence remains visible; process CPU measurements do not substitute for later GPU board-power, thermal, or battery validation.

## D-017 — Host-owned saved scenes

**Decision:** Persist named creative views in a new bounded `.tvscene` value format rather than modifying `.tvpreset` packages or serializing renderer internals.

**Rationale:** A user should be able to return to tuned sliders, colors, materials, zones, and camera state across launches. Keeping snapshots host-owned preserves immutable preset provenance, works for both built-in and WGSL modes, and prevents a saved view from gaining shader or native-code execution.
