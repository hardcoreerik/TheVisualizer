"""Generate progressive terrain albedo maps for Hyperreal Valley Flight."""
from __future__ import annotations

import os
import random
from pathlib import Path

from PIL import Image

random.seed(42)
ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "assets" / "valley-flight"
OUT.mkdir(parents=True, exist_ok=True)
N = 512


def fade(t: float) -> float:
    return t * t * t * (t * (t * 6 - 15) + 10)


def lerp(a: float, b: float, t: float) -> float:
    return a + (b - a) * t


def value_noise(size: int, period: int) -> list[list[float]]:
    cells = period
    grid = [[random.random() for _ in range(cells + 1)] for _ in range(cells + 1)]
    out = [[0.0] * size for _ in range(size)]
    for y in range(size):
        for x in range(size):
            fx = x / size * cells
            fy = y / size * cells
            x0, y0 = int(fx), int(fy)
            tx, ty = fade(fx - x0), fade(fy - y0)
            out[y][x] = lerp(
                lerp(grid[y0][x0], grid[y0][x0 + 1], tx),
                lerp(grid[y0 + 1][x0], grid[y0 + 1][x0 + 1], tx),
                ty,
            )
    return out


def fbm(size: int, octaves: int = 5) -> list[list[float]]:
    acc = [[0.0] * size for _ in range(size)]
    amp = 0.5
    total = 0.0
    period = 4
    for _ in range(octaves):
        layer = value_noise(size, period)
        for y in range(size):
            for x in range(size):
                acc[y][x] += layer[y][x] * amp
        total += amp
        amp *= 0.5
        period = min(period * 2, size // 2)
    for y in range(size):
        for x in range(size):
            acc[y][x] /= total
    return acc


def clampi(v: float) -> int:
    return max(0, min(255, int(v)))


def save_rgb(path: Path, fn) -> None:
    img = Image.new("RGB", (N, N))
    px = img.load()
    for y in range(N):
        for x in range(N):
            px[x, y] = fn(x, y)
    img.save(path, "PNG", optimize=True)
    print(f"wrote {path} ({img.size[0]}x{img.size[1]})")


def main() -> None:
    grass = fbm(N, 6)
    grass2 = fbm(N, 5)
    rock = fbm(N, 6)
    dirt = fbm(N, 5)
    detail = fbm(N, 7)

    save_rgb(
        OUT / "terrain-grass.png",
        lambda x, y: (
            clampi(40 + grass[y][x] * 55 + grass2[y][x] * 25),
            clampi(70 + grass[y][x] * 90 + grass2[(y * 3) % N][x] * 40),
            clampi(25 + grass[y][x] * 30),
        ),
    )
    save_rgb(
        OUT / "terrain-rock.png",
        lambda x, y: (
            clampi(90 + rock[y][x] * 70 + detail[y][x] * 20),
            clampi(88 + rock[y][x] * 65 + detail[y][x] * 15),
            clampi(85 + rock[y][x] * 60 + detail[y][x] * 12),
        ),
    )
    save_rgb(
        OUT / "terrain-dirt.png",
        lambda x, y: (
            clampi(95 + dirt[y][x] * 55 + detail[y][x] * 15),
            clampi(70 + dirt[y][x] * 40 + detail[y][x] * 10),
            clampi(40 + dirt[y][x] * 25),
        ),
    )
    save_rgb(
        OUT / "terrain-macro.png",
        lambda x, y: (
            clampi(60 + grass[y][x] * 40 + rock[y][x] * 50),
            clampi(75 + grass2[y][x] * 50 + dirt[y][x] * 30),
            clampi(55 + rock[y][x] * 45 + detail[y][x] * 20),
        ),
    )
    (OUT / "README.md").write_text(
        """# Valley Flight terrain textures

High-resolution albedo maps for Hyperreal Valley Flight progressive texturing.

| File | Role |
| --- | --- |
| `terrain-grass.png` | Meadow / low-slope albedo |
| `terrain-rock.png` | Cliff / high-slope albedo |
| `terrain-dirt.png` | Path and dry soil albedo |
| `terrain-macro.png` | Large-scale color variation |

Sampled at runtime with distance-based progressive LOD (near = full detail, far = macro only).
Regenerate with `python scripts/generate-valley-textures.py`.
""",
        encoding="utf-8",
    )
    print("done")


if __name__ == "__main__":
    main()
