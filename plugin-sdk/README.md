# TheVisualizer Plugin SDK v1

Licensed under Apache-2.0 as part of TheVisualizer.

This directory defines the size-tagged C ABI shared by TheVisualizer and explicitly trusted native plugins.

- C/C++ consumers use [`include/thevisualizer_plugin.h`](include/thevisualizer_plugin.h).
- Rust consumers may depend on the `thevisualizer-plugin-sdk` crate.
- `thevisualizer_plugin_v1` returns a static descriptor with `initialize`, `process`, and `shutdown`.
- Feature arrays are read-only and valid only during the synchronous `process` call.
- The only v1 output is a host-clamped response multiplier. The host retains audio capture, rendering, GPU, window, and frame ownership.

Native plugins run in-process with the user's privileges. Approval is not sandboxing, signing, or crash isolation.
