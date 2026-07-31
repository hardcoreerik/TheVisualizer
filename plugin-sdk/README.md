# TheVisualizer Plugin SDK

Licensed under Apache-2.0 as part of TheVisualizer.

This directory defines the size-tagged C ABI shared by TheVisualizer and explicitly trusted native plugins.

- C/C++ consumers use [`include/thevisualizer_plugin.h`](include/thevisualizer_plugin.h).
- Rust consumers may depend on the `thevisualizer-plugin-sdk` crate.
- ABI v1 remains supported through `thevisualizer_plugin_v1` and one response multiplier.
- ABI v2 uses `thevisualizer_plugin_v2`, profile-aware initialization, onset/transient input, ten Performance macro inputs, eight MIDI/OSC telemetry lanes, up to 24 fixed modulation commands, and beat/take event flags.
- Feature arrays are read-only and valid only during the synchronous `process` call.
- ABI v2 targets response, active-mode parameters, colors, camera, Zone Studio, Particle Forge, Performance Studio, and macros. The host validates indices, operations, finiteness, counts, and target-specific ranges before applying anything.
- MIDI CC 20–27 and localhost OSC `/thevisualizer/plugin/input/1` through `/8` feed the eight external lanes.
- Only one native plugin is active at a time. The host retains audio capture, rendering, GPU, window, and frame ownership.

Native plugins run in-process with the user's privileges. Approval is not sandboxing, signing, or crash isolation.
