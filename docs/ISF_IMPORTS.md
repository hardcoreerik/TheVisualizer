# ISF imports

TheVisualizer includes eight modified, audio-reactive WGSL adaptations from the
official [VIDVOX ISF-Files](https://github.com/Vidvox/ISF-Files) collection.
The source was pinned at commit
`395072d48b3ce7351ccb20a5fda54470591324df` on 2026-07-29. The upstream
collection and these modified adaptations are MIT-licensed.

## Compatibility result

All 327 upstream `.fs` files were inspected. Eight are single-pass generators
with no image, audio-texture, FFT-texture, persistent-buffer, or transition
input. Those eight were adapted. The other 319 remain excluded because the
current `.tvpreset` host deliberately owns a single texture-free frame pass.
No foreign DLL or OpenGL host was added.

The adaptation is intentionally offline and reviewed: ISF metadata and GLSL are
not executed or translated inside the application. Each selected algorithm was
rebuilt as bounded WGSL against TheVisualizer's existing audio buffers, then
given at least 25% more live parameters or audio mappings than its ISF source.

## Included artifacts

| Bundled preset | Upstream file | Upstream credit | Modifications |
| --- | --- | --- | --- |
| ISF · Broadcast Reactor | `ISF/Color Bars.fs` | VIDVOX | WGSL port, variable bar count, scanlines, bloom, bass roll, mid warp, treble glitch |
| ISF · Chromatic Atlas | `ISF/Color Schemes.fs` | VIDVOX | WGSL harmony field, radial mapping, continuous count, bass turn, mid hue, treble edges |
| ISF · Pulse Hearts | `ISF/Heart.fs` | VIDVOX | WGSL heart field, multiple orbiting hearts, echoes, bass pulse, mid orbit, treble sparks |
| ISF · Gradient Engine | `ISF/Linear Gradient.fs` | Carter Rosenberg | WGSL gradient curves, arbitrary angle, motion, audio warp, shimmer |
| ISF · Signal Noise | `ISF/Noise.fs` | VIDVOX | WGSL noise modes, temporal persistence, scanlines, audio scale/color/static |
| ISF · Ridgeline Terrain | `ISF/Ridgelines.fs` | VIDVOX; simplex credit to Ashima Arts / Stefan Gustavson | Bounded WGSL multifractal ridges, contours, snow, fog, audio terrain motion |
| ISF · Fractal Noise | `ISF/Simplex Noise.fs` | VIDVOX; simplex credit to Ashima Arts / Stefan Gustavson | Bounded WGSL fractal field, palette mapping, audio fold/flow/grain |
| ISF · Worley Cells | `ISF/Worley Cells.fs` | VIDVOX | Bounded WGSL cellular field, three metrics, borders, audio scale/flow/glow |

The exact upstream MIT text is preserved at
`third-party/isf-files/LICENSE`. The upstream source remains available from the
repository and revision above; it is not vendored into the application package.

## Re-audit

Run:

```powershell
.\scripts\audit-isf-imports.ps1 -Source C:\path\to\ISF-Files
```

The audit fails if the checkout revision or eligible source set differs from
the reviewed pin. New imports require a new revision, artifact-level license
review, explicit adaptation, and WGSL validation.
