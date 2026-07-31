# Presets and Plugins

This document defines the implemented v0.1 extension boundary for presets and the repository-owned native-plugin proof.

## Two extension tiers

### Declarative WGSL presets

The implemented preset package is one UTF-8 `.tvpreset` file, limited to 128 KiB. A strict `//! key: value` header ends at `//! ---` and contains:

- stable preset identifier
- display name and version
- author and content-license metadata
- required preset-format version (`1` or `2`)
- WGSL shader source
- format 1: one `response|min|max|default` adjustable parameter
- format 2: 1–40 `group|id|label|min|max|default` parameters, including `response`

The host rejects missing, duplicate, unknown, malformed, or unsupported metadata. It compiles the shader inside a wgpu validation scope, owns all GPU resources, binds normalized audio features, and reports a concise error without terminating the player or replacing the last working pipeline. Presets do not execute native CPU code.

The host exposes five fixed-size bind-group entries:

- binding 0: the original 32-byte uniform contract containing resolution, time, response gain, low/mid/high energy, and RMS
- binding 1: a fixed read-only storage buffer containing 256 waveform values followed by 64 spectrum values
- binding 2: a 32-byte uniform whose original frame delta, peak, waveform length, and spectrum length fields are followed by onset, transient intensity, and reserved padding
- binding 3: 160 read-only scene floats containing camera state, up to eight normalized Zone Studio layers, routed colors, material values, and bounded zone visual controls
- binding 4: 40 read-only finite mode-parameter floats in metadata order

Bindings 0–2 remain layout-compatible with format 1. Format-2 presets use the scene and parameter buffers without receiving GPU/window handles or allocating host buffers. The fixed bounds prevent preset-controlled buffer allocation.

The player discovers `.tvpreset` extensions case-insensitively beside the executable, through a Cargo build's `target\<profile>` ancestry, or from the working-directory `presets` folder; `THEVISUALIZER_PRESETS` overrides that location for focused testing. Startup skips rejected shaders until one compiles, and refresh can initialize the GPU renderer after an initially invalid or empty directory. Optional live reload polls the bounded preset directory every 500 ms and retains the last working pipeline when a replacement fails validation. Seventeen bundled format-2 presets expose 12–35 grouped controls. A visual editor, remote downloads, and a marketplace remain outside v0.1.

### Trusted native plugins

The implemented `.tvplugin` manifest contains:

- stable plugin identifier
- display name and plugin version
- author and license metadata
- required host ABI version
- platform and architecture
- relative native-library path; the entry point is fixed as `thevisualizer_plugin_v1`

The plugin ABI is a size-tagged C ABI rather than a Rust ABI. Rust definitions and a matching C/C++ header are in the [Plugin SDK](../plugin-sdk/README.md). ABI v1 remains loadable with its single bounded response multiplier. ABI v2 adds profile-aware initialization, onset/transient and bounded controller inputs, up to 24 fixed modulation commands, beat/take event flags, and shutdown.

The host supplies a versioned, size-tagged read-only snapshot containing time, frame delta, waveform and spectrum slices, RMS, peak, low/mid/high energy, onset, transient intensity, ten Performance macro values, and eight external input lanes. MIDI CC 20–27 or localhost OSC `/thevisualizer/plugin/input/1` through `/8` update those lanes. Feature pointers are valid only for the synchronous process call.

Each ABI-v2 command contains a target, index, Set/Add/Multiply operation, and finite value. The host validates the 24-command limit and clamps every target to its existing safe range before applying response, active-mode parameters, color motion/materials, camera, Zone Studio, Particle Forge, Performance Studio, or macros. Beat and deck-take events use a separate bounded bit field. The plugin still receives no direct `wgpu`, Direct3D, Vulkan, Metal, native-window, capture-device, audio-buffer, or frame-lifecycle ownership.

The bundled Creative Suite uses one shared DLL and eighteen strict manifests:

| Plugin | Host-owned behavior |
| --- | --- |
| Beat Choreographer | response, glow, Forge twist, and beat events |
| Spectral Colorist | hue, phase, and saturation motion |
| Zone Dancer | Zone Studio position, strength, type, and rotation |
| Camera Pilot | yaw, pitch, and zoom choreography |
| Particle Conductor | Particle Forge spin, twist, topology, and gradient |
| Transition DJ | Performance crossfader, transition, softness, and deck takes |
| MIDI Performance Mapper | eight external lanes to Performance macros 1–8 |
| Ambient Auto-Director | coordinated color, camera, zone, glow, and Forge motion |
| Analog Fractal Pilot | Fractal Reef dive/path/waveform-formation parameters and response |
| Spectrum Sculptor | color motion plus palette band/speed mode parameters |
| Onset Architect | zone placement, Forge topology kicks, and glow on onsets |
| Silence Gardener | softens response/glow and travel when quiet persists |
| Macro Weaver | maps bands, RMS, peak, transient, and beat into macros 1–8 |
| Stereo Navigator | camera yaw/pitch/zoom from band imbalance |
| Pulse Drummer | zone strength and response punches on beats |
| Color Storm | aggressive hue/phase/saturation storms |
| Deck Juggler | Performance crossfader, transition, and deck takes |
| Ambient Orbit | slow camera/zone/color orbit with gentle travel speed |

One native plugin is active at a time. This avoids ordering conflicts and preserves a clear approval identity; simultaneous plugin chains remain deferred.

## Trust and safety

Native plugins execute in the TheVisualizer process with the user's operating-system privileges. They can crash the process and may access anything the user account can access. A manifest, checksum, warning, or approval record does not sandbox native code.

Therefore:

- Unknown native plugins remain disabled.
- The `DETAILS & EXTENSIONS` disclosure shows plugin identity, path, version, author metadata, and the native-code warning before approval without occupying the default visual experience.
- Approval is explicit, session-only, and tied to the discovered library's full SHA-256 digest. The host recomputes the digest immediately before loading and refuses a changed artifact.
- A failed or incompatible load is reported and disabled rather than retried in a loop.
- Signed-plugin infrastructure is deferred and must not be implied by the approval flow.

Cross-process isolation may be considered if real plugins justify its latency and interprocess-rendering complexity.

## Compatibility

The v0.1 SDK is a new cross-platform TheVisualizer interface. It does not load legacy Winamp visualization DLLs. The repository-owned SDK, example plugin, and bundled presets are Apache-2.0. MilkDrop and projectM compatibility remains a separate engineering decision, and third-party presets may be bundled only with an explicit artifact-level license and provenance record; see [Licensing](LICENSING.md).

Native libraries are built separately for each supported operating system and architecture. Cross-platform ABI design does not make one compiled binary portable.

Desktop native plugins and WGSL presets are not promised to run on microcontrollers. Embedded renderers may reuse normalized feature concepts through independently defined, resource-bounded formats.

## v0.1 proof

The repository-owned ABI-v1 example plugin:

- remains disabled until approved and after restart
- passes ABI-version and structure-size checks
- receives normalized features without owning capture or rendering
- drives a visible bounded response multiplier in a host-owned WGSL visual
- shuts down on unload and rejects processing outside its initialized lifecycle

The ABI-v2 Creative Suite proves bounded multi-target routing without giving plugins rendering ownership. Neither ABI establishes a public compatibility guarantee beyond the documented versions.
