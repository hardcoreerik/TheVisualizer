# TheVisualizer Agent Instructions

Operate scout-first and keep changes narrow.

## Source of truth

- `main` is the protected source-of-truth branch.
- `PROJECT_TRUTH.md` records verified implementation and runtime evidence.
- `ROADMAP.md` is the milestone-status authority.
- Proposed behavior must not be described as shipped or tested.

## Change discipline

- Inspect the current branch, diff, and relevant documents before editing.
- Do not merge, force-push, rebase a shared branch, delete branches, or create a remote without explicit approval.
- Prefer the smallest implementation that proves the current milestone.
- Do not add frameworks, abstractions, plugin features, or input types for later milestones.
- Run the narrowest meaningful check first and report the exact command and result.
- Never claim a build, test, device, platform, or package works unless it was run and observed.

## Audio and rendering

- Treat capture callbacks as real-time boundaries: do not block, allocate without bounds, or perform rendering work inside them.
- Preserve one shared normalized feature path for built-in visuals, presets, and plugins.
- Measure latency and device behavior before fixing buffer sizes or thresholds.
- Keep platform capture code outside shared analysis and rendering behavior.

## Native plugin safety

- Native plugins are trusted code running with the user's process privileges.
- A trust prompt is approval, not sandboxing.
- Unknown native plugins remain disabled until explicitly approved.
- The host owns audio capture, windows, GPU resources, and frame lifecycle.
- Do not expose raw GPU or window handles in the v0.1 ABI.
- Never market plugins as safe merely because a manifest, checksum, or approval record exists.

## Embedded boundaries

- Embedded support begins after desktop v0.1.
- Do not claim desktop ABI, WGSL, preset, or binary compatibility on microcontrollers.
- Record exact boards, audio inputs, displays, LEDs, memory, and performance as measured evidence before committing a target-specific implementation.
