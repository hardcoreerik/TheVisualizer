# Project Truth

Last updated: 2026-07-27

This document records verified project state. Planned behavior belongs in the [Roadmap](ROADMAP.md) and design documents, not in this file.

## Verified locally

- `F:\Ai\TheVisualizer` began as an empty directory.
- A local Git repository was initialized with `main` as its source-of-truth branch.
- The documentation foundation is being authored on `docs/project-foundation`.
- Local `main` and `docs/project-foundation` share the empty baseline commit `be7a8e0`; no merge or remote repository exists.
- The development host runs Windows.
- Rust `1.96.0`, Cargo `1.96.0`, Git `2.51.1.windows.1`, CMake, and Ninja were present on `PATH` during the initial scout.
- Windows reported an AMD Radeon integrated GPU and an NVIDIA GeForce RTX 5070 Ti. Windows PnP currently marks the AMD adapter `CM_PROB_DISABLED`; NVIDIA reports `CM_PROB_NONE`.
- One HP X34 monitor is active at 3440×1440, 165 Hz, and 96 DPI. No second active monitor or mixed-DPI configuration is available on this host.

## Research-verified

- The official projectM repository identifies the core library as LGPL-2.1-or-later, states that its core does not ship presets, and warns that related projects may use different licenses.
- The official MilkDrop 2.25c source archive applies a three-clause BSD-style notice to core `vis_milk2` files but also contains separately copyrighted support code.
- The projectM Cream of the Crop repository states that almost all collected presets lacked an explicit license and relies on a public-domain assumption. TheVisualizer does not treat that assumption as redistribution permission.
- Microsoft documents `CM_PROB_DISABLED` as Device Manager Code 22, meaning that the device is disabled; see [CM_PROB_DISABLED](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/cm-prob-disabled).
- `cargo metadata --locked --filter-platform x86_64-pc-windows-msvc` reported license metadata for every resolved external package on 2026-07-27.

## Implemented

- Native Rust desktop application using `eframe` 0.35.0 with its `wgpu` renderer.
- CPAL 0.18.1 capture from the default Windows output device as WASAPI loopback.
- CPAL capture from the default Windows microphone.
- Enumeration and explicit selection of available output-loopback and microphone devices by CPAL device ID.
- A device refresh action; the System and Microphone shortcuts enter automatic mode and follow their current Windows defaults.
- One-second default-device identity checks on the UI thread. Explicit device selections remain pinned instead of being silently replaced.
- Automatic default changes report detection-to-stream-open and detection-to-first-packet timing, while explicitly identifying the unmeasured 0–1000 ms polling interval and quiet-endpoint delay.
- Automatic mode retries capture after a backend stream error even when Windows restores the same default device ID. If no default exists or reopening fails, the overlay keeps an actionable error and retries on the next poll.
- A bounded 16,384-sample shared buffer; the capture callback uses `try_lock` and drops work rather than waiting on UI contention.
- CPAL capture timestamps are used to estimate the newest sample's backend age. The packet span is removed before adding callback-to-feature handoff time.
- Analysis copies one bounded snapshot while holding the sample lock, then releases the lock before running the FFT. A callback-sequence gate publishes features at most once per new captured packet; UI frames without new audio reuse the latest immutable feature set.
- 2,048-sample Hann-windowed FFT analysis using RustFFT 6.4.1.
- Waveform, 64 spectrum bands, RMS, peak, and low/mid/high energy features.
- Compact current, average, and peak newest-sample-to-feature age telemetry. The separate FFT window duration is shown beside it.
- Compact smoothed application-frame interval and UI cadence telemetry, explicitly labeled as distinct from monitor refresh, display scanout, and playback-to-photon latency.
- Oscilloscope/neon-trail and spectrum/particle visuals drawn through egui, plus an application-owned WGSL feedback visual.
- A bounded ten-snapshot visual history supplies fading waveform trails and attack/release-smoothed spectrum trails. Capture-source changes clear this history.
- The particle visual maps low-to-high spectrum order into a radial spiral with a reactive core, frequency-color progression, connecting contours, and bounded motion trails.
- A host-owned wgpu render pipeline, uniform buffer, and fullscreen-triangle callback for the WGSL visual.
- A bounded host-owned GPU feature buffer supplies every WGSL preset with the same 256-point waveform and 64-band spectrum used by built-ins, while supplemental uniforms supply frame delta and peak alongside the original time, gain, RMS, and low/mid/high values.
- Discovery of bounded single-file `.tvpreset` packages containing strict format, identity, author, license, response-parameter, and WGSL metadata.
- Manual preset refresh and keyboard/dropdown selection. The host compiles replacements inside a wgpu validation scope and retains the last working pipeline when validation fails.
- Two repository-owned WGSL presets: Feedback Tunnel and Solar Bloom.
- An optional `THEVISUALIZER_GPU` adapter-name override for focused compatibility checks.
- An optional `THEVISUALIZER_PRESETS` directory override for focused discovery and packaging checks.
- A shared size-tagged native-plugin ABI v1 with matching Rust definitions and a C header.
- Strict `.tvplugin` manifest discovery for the current platform without loading native code.
- SHA-256 artifact identity displayed before approval and recomputed immediately before library loading. Approval is explicit and lasts only for the current run.
- One repository-owned Windows example plugin implementing initialize, synchronous feature processing, bounded response output, and shutdown without receiving audio ownership or GPU/window handles.
- An optional `THEVISUALIZER_PLUGINS` directory override for focused discovery and packaging checks.
- A guarded Windows x86_64 packaging script that builds the locked release workspace and stages a portable local-test archive under `dist`.
- The portable package includes the player, both WGSL presets, the example-plugin manifest and DLL, C/Rust SDK documentation, project and target-resolved third-party license/provenance files, a package-specific readme, an explicit local-test notice, and SHA-256 checksums.
- The staged plugin manifest uses a package-relative DLL path; no development-machine path is included in packaged text or manifests.
- Apache-2.0 project licensing with a canonical `LICENSE`, contributor `NOTICE`, manifest metadata, and a recorded compatibility/provenance policy.
- A fail-closed Windows license collector inventories the locked target graph, copies crate-provided files, and uses exact Cargo-pinned upstream sources when crates omit them. Its sole canonical-text fallback is CC0-1.0 from SPDX license-list-data v3.26.0 for `hexf-parse`.
- The repository and portable package contain only repository-owned presets and plugin content; no MilkDrop/projectM code, community presets, or third-party textures are included.
- Named Scope, Particles, and Preset visual selection, response gain, source switching, overlay hiding, and explicit windowed, borderless, and fullscreen presentation modes.
- Progressive overlay disclosure: source, preset, response, visual, levels, presentation, and status remain immediate, while capture format, GPU identity, native-plugin review, latency, and frame telemetry begin collapsed under `DETAILS` or `DETAILS & EXTENSIONS`. Errors remain immediate.
- `WAITING`, `LIVE`, `SILENT`, and `ERROR` status presentation.
- Fifteen focused automated tests across the workspace covering analysis, callback-sequence gating, devices, same-ID capture-failure recovery, timestamp adjustment, latency aggregation, frame-cadence smoothing, default-switch timing, bounded visual history, presentation, presets, the WGSL scalar and waveform/spectrum buffer contract, strict plugin manifests, changed-artifact rejection, ABI sizes, and the example-plugin lifecycle.

## Runtime-observed results

- `cargo check --workspace` completed successfully on 2026-07-27.
- `cargo test --workspace` passed 15 tests with 0 failures on 2026-07-27.
- `cargo clippy --workspace -- -D warnings` completed successfully on 2026-07-27.
- After the same-ID recovery change, the rebuilt debug application opened automatic system capture and local Windows system sounds produced `LIVE`, nonzero RMS/peak values, and visible neon trails.
- The debug application launched and rendered at 1280×720 on the Windows development host.
- The default speaker endpoint opened as 48,000 Hz stereo loopback capture.
- Repeated local Windows system sounds changed status from `WAITING` to `LIVE`, produced nonzero RMS/peak values, and visibly drove all three bundled visuals in the extracted package without reopening capture.
- After output became idle, status changed from `LIVE` to `SILENT`; the neon scope settled to a stable zero line, and the next system sound returned the same stream to `LIVE`.
- The default HD Pro Webcam C920 microphone opened at 48,000 Hz stereo, reported `LIVE`, produced nonzero RMS/peak values, and visibly drove all three bundled visuals without reopening capture.
- All three visuals were selected and visually inspected.
- The compact overlay and expanded details state were visually inspected at 1280×720 and the 800×500 minimum content size. The compact GPU state remained focused on player controls; the expanded minimum-size state retained capture/GPU details, the complete plugin warning and approval action, and telemetry without clipping.
- Named Scope, Particles, and Preset controls remained on one row at 1280×720 and the 800×500 minimum. Mouse selection changed to Particle Array, and repeated Windows system sounds visibly drove its bounded frequency trails without reopening capture.
- With callback-sequence gating active, the release build produced live waveform motion from repeated Windows system sounds, switched to the default microphone with fresh nonzero features, then returned to system capture and again reported `LIVE` at approximately 0.051 RMS / 0.092 peak. This verifies that source replacement resets the gate.
- Three quiet ten-second samples averaged 8.8% of one CPU core for both the committed package and the callback-gated release on this host. The current WASAPI callback cadence therefore produced no measurable local CPU reduction; the gate prevents stale re-analysis when callback cadence is lower than UI cadence but is not claimed as a performance win here.
- Live system audio visibly produced multiple fading scope traces and expanded the frequency-ordered particle spiral. The particle field remained stable after silence and scaled through 1280×720 windowed and fullscreen presentation.
- `F11` entered fullscreen, `Escape` returned to windowed mode, `Tab` hid the overlay, and `Escape` closed the window.
- The application-owned WGSL visual rendered on the NVIDIA GeForce RTX 5070 Ti in windowed and 2880×1620 fullscreen modes.
- Local Windows system sounds changed the WGSL visual's shape and color while the overlay reported `LIVE` with nonzero RMS and peak values.
- A forced AMD adapter check did not launch: wgpu enumerated NVIDIA adapters and Microsoft Basic Render Driver, but no AMD adapter. Windows PnP then identified the AMD adapter as disabled with `CM_PROB_DISABLED`, so AMD rendering cannot be validated without a deliberate system-level enablement outside TheVisualizer.
- With `THEVISUALIZER_GPU=Microsoft Basic Render Driver`, the application launched and reported that software adapter in the overlay. Feedback Tunnel rendered in windowed and 3440×1440 fullscreen modes; local Windows system sounds produced `LIVE` with nonzero RMS/peak values. A transition sample showed feature-age peak telemetry of 61.2 ms. Smoothed application cadence measured about 9–10 FPS at roughly 102–109 ms per frame windowed and about 3 FPS at roughly 396 ms per frame fullscreen, so this proves compatibility rather than a usable performance floor.
- On the NVIDIA GeForce RTX 5070 Ti and active 3440×1440 165 Hz HP X34, the debug player reported about 162–172 application UI frames per second windowed. The WGSL Feedback Tunnel held about 168–169 FPS windowed and about 168–170 FPS fullscreen, at roughly 5.9–6.0 ms per application frame. These are application-cadence observations, not proof of monitor scanout rate or playback-to-photon latency.
- The device menu enumerated four output endpoints and two microphone endpoints on the development host.
- The non-default HP X34 output opened at 48,000 Hz stereo, and the non-default Steam Streaming Microphone opened at 44,100 Hz mono.
- Refreshing the device inventory preserved the active capture. The System shortcut then returned to the default speakers, where local system sounds produced `LIVE` state and nonzero RMS/peak values.
- The overlay displayed `AUTO · follows Windows default` for default-managed capture and `PINNED · manual device` after explicit HP X34 selection.
- While automatic system capture was active, changing the Windows default output from Speakers to HP X34 caused TheVisualizer to reopen HP X34. Restoring the Windows default caused it to reopen Speakers, where system sounds again produced `LIVE` state and nonzero RMS/peak values.
- During an active bounded-sound probe, automatic recovery to HP X34 took 12.3 ms from default-change detection to stream open and 25.4 ms to the first capture packet. Returning to Speakers took 14.7 ms to open and 23.2 ms to the first packet; capture remained `LIVE` and visual trails continued.
- A quiet HP X34 switch separately showed why first-packet time is not pure recovery latency: the stream opened in 10.4 ms but reported its first packet 9.3 seconds later only after sound was produced.
- The Windows default output was restored to its original Speakers endpoint after the recovery test.
- The overlay and keyboard controls entered borderless presentation at 3440×1440, entered fullscreen, and returned to the original decorated 1280×720 window.
- The application-owned WGSL visual continued rendering through borderless, fullscreen, restore, and maximize-resize transitions.
- System-loopback capture remained `LIVE` with nonzero RMS and peak values while the WGSL visual ran fullscreen.
- Feedback Tunnel and Solar Bloom were discovered from `.tvpreset` files, selected with the preset shortcut, and visually inspected in the running application.
- After the shared GPU feature path was completed, Feedback Tunnel compiled and used angle-mapped spectrum values while Solar Bloom compiled and used angle-mapped waveform and spectrum values. Repeated Windows system sounds produced `LIVE`, nonzero levels, and visibly asymmetric frequency/waveform-driven contours in both presets. Solar Bloom also rendered fullscreen at 3440×1440; its quiet steady-state application cadence returned to about 175 FPS windowed and measured about 173 FPS fullscreen on the NVIDIA adapter.
- Windows system sounds drove Solar Bloom while the overlay reported `LIVE` with nonzero RMS and peak values.
- A deliberately invalid WGSL preset produced a concise validation error while Solar Bloom remained rendered and usable. Removing the temporary file and refreshing cleared the error; the invalid probe was not retained in the repository.
- The example plugin was discovered with `DISABLED` status before any approval, with its identity, library path, version, author, license, truncated SHA-256 digest, and explicit no-sandbox warning visible.
- Selecting `Approve & Load` initialized the example plugin. Live system audio changed its bounded response multiplier while the host-owned Feedback Tunnel continued rendering.
- Selecting `Unload` returned the plugin to `DISABLED`; the example lifecycle test independently verified that processing fails before initialization and after shutdown.
- Restarting TheVisualizer returned the example plugin to `DISABLED`, confirming that approval is session-only.
- `scripts\package-windows.ps1` completed successfully with the target-specific license collector and `cargo build --workspace --locked --release`. After gating analysis by capture callback sequence, it created a 7,320,110-byte local-test ZIP with SHA-256 `4B8C6A1B5CB7BE0466633D1078CF3C65E8D46F35F761AC28AD92DF28760F415C`.
- The rebuilt package launched from its staged directory, discovered the package-relative presets and example plugin, compiled the richer Feedback Tunnel shader, and rendered it at about 165 FPS in a quiet windowed observation on the NVIDIA adapter.
- The package's third-party bundle contained 205 resolved package entries and 384 non-empty license or notice files. Every resolved package had one summary entry.
- A second independent collector run produced the same aggregate path-and-content digest as the staged bundle.
- A fresh extraction under `dist` contained 398 staged files and passed every entry in `SHA256SUMS.txt` with zero failures.
- A fresh temporary extraction of the same-ID recovery package passed all 397 payload entries in `SHA256SUMS.txt` with zero failures.
- The fresh extracted package launched the updated executable; bounded system sounds produced `LIVE` state and visibly drove the radial particle field.
- Every staged file and every file extracted into a fresh directory matched its entry in `SHA256SUMS.txt`.
- The extracted release executable launched from the fresh directory on the development host, opened the default 48,000 Hz stereo loopback source, and discovered both packaged WGSL presets and the packaged example plugin.
- Feedback Tunnel and Solar Bloom rendered from the extracted package. Local Windows system sounds produced `LIVE` state and nonzero RMS/peak values while Solar Bloom ran.
- The packaged example plugin remained `DISABLED` until explicit approval, then reported `ACTIVE` while driving the host-owned visual. `Escape` closed the extracted application normally.
- During a short system-loopback validation run, the debug overlay showed newest-sample-to-feature estimates between approximately 1 and 10 ms, with an observed average near 3.6 ms and a separate 42.7 ms FFT window at 48,000 Hz.
- Switching to the default microphone produced fresh source-specific telemetry and live nonzero levels; one short observation showed approximately 1.0 ms current, 7.1 ms average, and 18.0 ms peak newest-sample-to-feature age.
- The rebuilt extracted release package showed `LIVE`, reactive waveform motion, approximately 0.6 ms current, 3.6 ms average, and 9.8 ms peak feature age, plus the 42.7 ms FFT window.
- These estimates begin at the audio backend's capture timestamp and end at feature publication. They do not measure application playback buffering, compositor/display scanout, or human-perceived playback-to-photon latency.
- The rebuilt package and a fresh extraction both passed every `SHA256SUMS.txt` entry after the license change.
- The extracted package contained `LICENSE`, `NOTICE`, and `docs\LICENSING.md`; its bundled preset and example-plugin manifests reported `Apache-2.0`.
- The extracted release launched, rendered Feedback Tunnel, displayed the example plugin as `DISABLED`, and visibly reported `Apache-2.0` while retaining the explicit native-code/no-sandbox warning.
- The rebuilt same-ID recovery package launched in automatic system mode; local Windows system sounds produced `LIVE`, nonzero RMS/peak values, and visible neon trails.
- The official MDN T-Rex audio sample reported active playback in a controlled browser transport, but that transport did not route audio to the captured Windows endpoint; an immediate local Windows system-sound control confirmed loopback remained healthy.
- A separate installed Chrome 150 instance used an isolated disposable profile and a local Web Audio page whose analyzer reported a running 48,000 Hz signal at approximately 0.050 RMS. Before Chrome started, the packaged player reported `WAITING` with zero levels. It then reported `LIVE` at approximately 0.049 RMS / 0.095 peak while the browser signal visibly drove Neon Scope, Particle Array, and Feedback Tunnel without reopening capture. Stopping only that Chrome instance returned the same stream to `SILENT` and zero levels.

## Not implemented or runtime-verified

- Runtime validation of no-default device removal and later same-ID recovery
- Externally timestamped playback-to-display latency
- Beat or onset detection
- Explicit multi-monitor selection
- Rendering on the currently disabled AMD integrated GPU and minimum hardware/driver validation
- Automatic preset-directory watching, an editor, additional parameter types, or remote preset delivery
- Persistent approval records, signed-plugin infrastructure, or cross-process crash isolation
- A clean-machine package run, installer, updater, signing, release metadata/icon, public distribution, or formal installed-program uninstall
- macOS or Linux builds and capture backends
- MilkDrop, projectM, or legacy Winamp compatibility
- MIDI, OSC, sensor, telemetry, or network input adapters
- ESP32-class, M5Tab5, or LED-controller support

## Unverified assumptions

- The current application-owned WGSL path will meet later preset validation and performance requirements.
- The observed capture-to-feature age and current 42.7 ms FFT window will produce acceptable playback-to-display response across supported Windows systems.
- In-process native-plugin risk and synchronous processing cost will remain acceptable beyond the repository-owned proof.
- The selected visual feature set can be represented consistently across different sample rates and devices.
- Desktop audio-feature concepts will be useful to embedded renderers without requiring binary or shader compatibility.

## Current gates

1. Run an externally timestamped playback-to-display latency test and decide whether the default-device poll's unmeasured 0–1000 ms detection interval is acceptable.
2. Enable the AMD display adapter outside TheVisualizer only when display disruption is acceptable, then rerun the forced WGSL probe; it is currently disabled by Windows with Code 22.
3. Validate resize and presentation continuity on another system with multiple active monitors and mixed DPI; this host exposes one active 96-DPI HP X34.
4. Validate the portable package on a separate clean Windows machine and decide whether v0.1 needs an installer.
