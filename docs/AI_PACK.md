# Offline AI Pack

AI Studio is optional and runs locally. It does not use an API key, cloud
inference, an HTTP server, or an open network port.

The separate pack must contain:

- `manifest.json`
- DreamShaper XL Lightning SFW (`DreamShaperXL_Lightning-SFW.safetensors`)
- a CUDA or Vulkan `thevisualizer-ai-worker.exe`
- the approved `tvizfield` LoRA, when available
- upstream MIT and OpenRAIL++ license notices

Place the pack in an `ai-pack` folder beside `thevisualizer.exe`, or set
`THEVISUALIZER_AI_PACK` to its folder. AI Studio verifies canonical paths and
every SHA-256 value before launching native code. The approved model hash is:

`832e7bb2302c3cd67c818ca4fe9dcbedf696d3070dab5463127263ec4db9899f`

Build the workers and pack with `scripts/build-ai-pack.ps1`. Model weights,
LoRA checkpoints, and generated scenes are intentionally excluded from Git.
Generated PNGs and JSON metadata remain under
`%LOCALAPPDATA%\TheVisualizer\generated` until the user removes them.
