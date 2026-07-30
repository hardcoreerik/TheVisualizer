# Roadmap

This document is the authority for milestone status. A milestone is complete only when its exit gate has been run and recorded in [Project Truth](PROJECT_TRUTH.md).

| Milestone | Status | Exit gate |
| --- | --- | --- |
| 1. Documentation and feasibility | Complete | Canonical docs agree; focused WASAPI, GPU, and plugin-loading spikes answer the blocking questions |
| 2. Windows audio foundation | Nearly complete | System loopback and microphone sources feed bounded normalized audio features with measured latency and clear error states |
| 3. Renderer and player | Complete | Core bundled modes and shared interaction controls run in windowed, borderless, and fullscreen modes |
| 4. Presets and native plugins | Complete | Format-2 WGSL presets load safely; one explicitly approved example native plugin passes the versioned host-owned lifecycle |
| 5. Windows desktop v0.1 | In progress | Acceptance checks pass on supported Windows hardware and a clean packaged install |
| 6. macOS and Linux | Not started | Native capture backends and packages pass equivalent platform acceptance checks |
| 7. MilkDrop/projectM compatibility | Research only | Format, rendering, licensing, and maintenance options produce a recorded implement/defer decision |
| 8. Standalone embedded visualizers | Deferred until after v0.1 | Exact target hardware independently captures audio and renders a bounded visual demo |

## Public repository state (2026-07-30)

- Public remote: `https://github.com/hardcoreerik/TheVisualizer` (public).
- `origin/main` remains the empty initialize commit `be7a8e0` (zero files). It is **not** a usable public source tree.
- All implemented code and docs live on `docs/project-foundation` (`origin/docs/project-foundation` at last push, plus uncommitted local work).
- No open or closed pull requests exist. Truth-sync and a review PR into `main` are required before any public-release claim.

## Milestone 1 — Documentation and feasibility — Complete

Exit gate met on the development branch:

- Verified facts stay in [Project Truth](PROJECT_TRUTH.md); proposals stay in design docs and this roadmap.
- WASAPI loopback, microphone enumeration/selection, automatic default following, and pinned explicit selection are proven.
- Application-owned WGSL renders on NVIDIA GeForce RTX 5070 Ti; Microsoft Basic Render Driver is a compatibility probe only (~9–10 FPS windowed / ~3 FPS fullscreen).
- Newest-sample-to-feature age and the separate 42.7 ms FFT window are instrumented; external playback-to-photon latency remains open under Milestone 5.
- Example native plugin discovery, session-only SHA-256 approval, and host-owned lifecycle are proven.
- Apache-2.0 project licensing and target-resolved third-party license packaging are in place; MilkDrop/projectM code and unlabeled community presets remain excluded.

## Milestone 2 — Windows audio foundation — Nearly complete

Proven:

- Select active output loopback or microphone; explicit selections remain pinned.
- Automatic mode follows Windows default-output changes and reports recovery timing after detection.
- Shared features: time, waveform, spectrum, RMS, peak, low/mid/high, spectral centroid/rolloff/flatness/crest, positive flux, and one bounded adaptive onset pulse (not BPM).
- Bounded cross-thread buffering; waiting, live, silent, and capture-error states with source-aware guidance.
- Capture-to-feature age reported separately from analysis-window duration.

Still open (blocks full “measured latency” honesty for v0.1):

- Externally timestamped playback-to-display latency.
- Physical no-default device removal / same-ID recovery validation.
- Integrated-GPU / minimum-hardware capture+render pairing (AMD adapter currently Windows-disabled Code 22 on the development host).

## Milestone 3 — Renderer and player — Complete

Original exit gate met and exceeded on the development host:

- Host-owned render loop and compact progressive-disclosure overlay.
- Neon Scope, Particle Forge, Feedback Tunnel / GPU presets, windowed/borderless/fullscreen, Display/60/30 pacing.
- Capture stays live through normal resize and presentation changes.
- Instrument, Mode Controls, Visual Library, Zone Studio, saved `.tvscene` views, and Performance Studio are implemented beyond the original three-visual scope.

Development inventory (not all package/acceptance-complete):

- Thirty-nine visuals: Neon Scope, Particle Forge, Studio, Visual Canvas, twenty-seven original format-2 WGSL presets, eight provenance-pinned MIT ISF adaptations.
- Zone Studio: up to eight independent GPU overlay types above any base visual.
- Studio Living Photograph with real motion WebP and optional 2.5D foliage rig (same-host package smokes recorded; some packaged motion visual checks still open).
- Performance Studio: dual WGSL decks, A/B mix, transitions, macros, MIDI + localhost OSC.
- Local Visual Director (prompt/brief laboratory only; no paid image API).

Remaining visual/performance acceptance for expanded modes is tracked under Milestone 5, not as a reopen of this core gate.

## Milestone 4 — Presets and native plugins — Complete

- Declarative format-2 WGSL presets with safe rejection and last-working retention.
- ABI v1 response-only example plus ABI v2 Creative Suite (eight profiles, 24 host-owned routes, beat/take, MIDI/OSC lanes).
- Explicit session approval; no GPU/window handles in the v0.1 ABI.
- Automated ABI, lifecycle, package, and checksum validation recorded in Project Truth. Live musical acceptance of every plugin profile remains a Milestone 5 polish item.

## Milestone 5 — Windows desktop v0.1 — In progress

Required for exit (see also [v0.1 Scope](docs/V0.1_SCOPE.md)):

1. **Public source truth:** land `docs/project-foundation` into protected `main` via reviewed PR so the public default branch is not empty.
2. **Expanded package smoke:** one current `scripts/package-windows.ps1` archive that includes the full preset/plugin/asset set; staged + fresh checksum; launch; capture; representative visual pass on the development host.
3. **Clean-machine package validation:** first run, source selection, device loss/recovery, fullscreen exit, folder uninstall on a separate Windows machine.
4. **Hardware breadth:** integrated or second GPU where available; mixed-DPI / multi-monitor when hardware exists.
5. **Latency honesty:** external playback-to-display measurement; decide whether the 0–1000 ms default-device poll gap is acceptable for release notes.
6. **Truth-sync:** README, Roadmap, Project Truth, and scope docs agree; no proposed AI/Studio modes claimed as shipped.
7. **Public-release gate:** signed metadata/icon decision, installer decision, and explicit “local-test vs public release” labeling.

### Priority order while closing Milestone 5

1. Documentation agreement (this pass) and keep truth current as evidence lands.
2. Restore a real public `main` via PR (approval required to open/merge).
3. Expanded portable package smoke on the development host (high usage; ask before large cycles).
4. Live acceptance of Visual Canvas, newest original scenes, Zone Studio quality, Event Horizon musical calibration, and Creative Suite profiles only where it unblocks packaging confidence.
5. Clean-machine and multi-hardware validation when a second machine/config is available.
6. External latency measurement before publishing performance claims.

### Explicitly out of v0.1 exit

- Offline AI Pack / AI Studio / LoRA training (experimental local lane; see [AI Pack](docs/AI_PACK.md)); not a v0.1 acceptance requirement.
- Ten further Studio modes listed in [Interactive Modes](docs/INTERACTIVE_MODES.md) (planned only).
- MP4/video/webcam input, spatial transition masks, external video output.
- Paid image-provider generation.
- macOS/Linux packages, MilkDrop/projectM, embedded targets.

## Later milestones

macOS and Linux use platform-native capture boundaries while sharing analysis, rendering concepts, and preset behavior where measurements permit. MilkDrop/projectM compatibility is a research decision, not an assumed feature. Embedded work is a separate standalone-runtime lane described in [Embedded Vision](docs/EMBEDDED_VISION.md).

## Explicitly deferred

- Media file playback and playlists
- Preset editor and automatic preset rotation
- Marketplace or online account
- Signed-plugin infrastructure
- Legacy Winamp DLL loading
- Sensor, telemetry, or broader non-localhost network adapters
- Mobile releases
- Desktop-to-embedded feature streaming
