# Valley Flight terrain textures

High-resolution albedo maps for Hyperreal Valley Flight progressive texturing.

| File | Role |
| --- | --- |
| `terrain-grass.png` | Meadow / low-slope albedo |
| `terrain-rock.png` | Cliff / high-slope albedo |
| `terrain-dirt.png` | Path and dry soil albedo |
| `terrain-macro.png` | Large-scale color variation |

Sampled at runtime with distance-based progressive LOD (near = full detail, far = macro only).
Regenerate with `python scripts/generate-valley-textures.py`.
