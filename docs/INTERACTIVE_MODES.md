# Interactive Modes

Status: implemented prototype; broader runtime and package validation remains open.

TheVisualizer now treats each visual as an interactive instrument. The host owns interaction state so built-ins and WGSL presets share bounded controls without receiving window or GPU handles.

## Direct manipulation

- Drag a sound zone to move its influence point.
- Drag empty space to rotate modes that use camera yaw and pitch.
- Use the mouse wheel over the visual to zoom.
- Double-click empty space to add a zone, up to eight.
- Select a zone from the right-click menu to change its band, radius, strength, or pin state.
- `R` resets camera and zones; `Delete` removes the selected unpinned zone.

A sound zone contains normalized position, radius, strength, audio band (`Full`, `Low`, `Mid`, or `High`), and pin state. Pinning protects the zone from deletion; deliberate dragging remains available. On-canvas zones use compact diamond anchors and four sparse range ticks instead of enclosing radius circles. Only the selected zone carries a text label.

## Instrument controls

`Instrument [I]` opens a persistent, scrollable panel beside the visual. The right-click menu provides the same control model when a temporary menu is preferable.

1. **Mode** — choose either host visual or any discovered GPU preset.
2. **Saved Scenes** — name, save, discover, and restore the current mode, relevant sliders, routed colors, material treatment, camera, and sound zones.
3. **Live Audio Routing** — inspect Full, Bass, Mid, and Treble meters using their routed colors.
4. **Sound Zones** — add/reset zones and edit the selected zone.
5. **Mode Controls** — show only the sliders declared by the active format-2 preset, grouped by that mode's metadata. Each group and the complete mode can be reset independently.
6. **Colors & Materials** — choose palettes, per-band colors, finish, and clickable/draggable gradient bars for glow, gloss, and saturation. Numeric fields remain available for precise and keyboard-accessible entry.
7. **Camera & Scene** — adjust yaw, pitch, and zoom, show/hide handles, or reset interaction.

The implemented palettes are Cyber Neon, Earth Tones, Arctic Glass, Inferno, Acid, Vaporwave, Monochrome, and Custom. Finishes are Neon, Glossy, Matte, Metallic, and Glass. Every frame supplies separate Full Range, Bass, Mid, Treble, and Background colors; format-2 presets decide how those routed colors affect their scene.

## Initial ten modes

| Mode | Implementation | Identity | Mode controls |
| --- | --- | --- | ---: |
| Neon Scope | host | layered oscilloscope and waveform trails | shared response |
| Particle Forge | host | frequency-ordered particle spiral | shared response |
| Feedback Tunnel | WGSL | traveling spectral tunnel with zone warping | 12 |
| Solar Bloom | WGSL | waveform flower, rays, and zone blooms | 12 |
| Neon Horizon | WGSL | synthwave sun, skyline, stars, and grid | 12 |
| Gravity Wells | WGSL | lensed particle field and orbiting wells | 12 |
| Ripple Garden | WGSL | zone interference, caustics, and audio flowers | 12 |
| Kaleido Reactor | WGSL | folded spectral reactor and movable warps | 12 |
| Aurora Flow | WGSL | audio ribbons, stars, shimmer, and zone flow | 12 |
| TheVisualCityScape | WGSL | rotatable party block with an impossible sky | 35 |

All ten modes also receive the shared zone, camera, palette, material, glow, gloss, and saturation controls. The two host visuals are not `.tvpreset` packages, so the panel intentionally gives them only the response control instead of irrelevant preset options.

## TheVisualCityScape

The tenth flagship mode combines the physical **Neverending Party Block** with the impossible geometry and evolving sky of **Pocket Metropolis**. It starts with four pinned routes:

- Bass drives the hotel and architecture.
- Mids drive the street party and crowd.
- Treble drives the rooftop and sky detail.
- Full-range energy drives the movable sky portal.

The 35 sliders are grouped under Audio, Camera and navigation, Buildings and streets, Hotel and apartments, Party and crowd, Traffic and particles, Sky worlds, Physics, and Generated scene content. The user can rotate the cylindrical city, zoom, reposition any of the four routes, change their audio bands, recolor each band independently, and turn the sky route into a moving hyperspace focus.

The scene is procedural. It does not currently use downloaded city photography, recognizable landmarks, generated image assets, or true polygonal 3D geometry. Its host-owned state can be saved in the same `.tvscene` format as every other mode.

## Saved scene format

Each `.tvscene` file is a bounded, versioned UTF-8 snapshot. It records identity for the required host mode or preset plus exactly the state the host already owns: response, forty bounded parameter slots, five routed colors, palette/material values, camera state, and up to eight sound zones. Restoring a scene first selects its mode, rejects a missing preset, clamps values to the current preset metadata, and only then replaces the active interaction state.

The player discovers scenes from `%LOCALAPPDATA%\TheVisualizer\scenes`; `THEVISUALIZER_SCENES` overrides that folder for focused testing. Invalid, oversized, duplicate-key, non-finite, or out-of-range files are rejected and reported without interrupting visualization. Scene files contain no shader code, native library, audio, API credential, or generated image asset.

## Runtime boundary

- The host stores at most eight sound zones and forty finite parameters per mode.
- Parameters, camera state, colors, materials, and zones use fixed-size host-owned GPU buffers.
- Presets receive values only; they do not receive pointer APIs, native window handles, or direct GPU ownership.
- Format 1 remains loadable. All eight bundled GPU presets now use format 2.
- The current shader effects are bounded per frame; no persistent GPU particle simulation or collision solver exists yet.
