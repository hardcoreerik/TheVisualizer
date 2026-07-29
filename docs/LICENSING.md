# Licensing

Last reviewed: 2026-07-27

This document records the project license decision and the boundary around future MilkDrop/projectM compatibility. It is an engineering provenance record, not legal advice.

## Project license

TheVisualizer's original source code, documentation, seventeen original bundled WGSL presets, plugin SDK, and example plugin are licensed under [Apache-2.0](../LICENSE). Their manifests use the same SPDX identifier.

Eight modified WGSL adaptations from VIDVOX ISF-Files are bundled under MIT with an exact source revision, file-level credits, modifications, and the upstream license recorded in [ISF Imports](ISF_IMPORTS.md). The repository contains no MilkDrop source, projectM source, `.milk` presets, unlabeled community preset packs, or third-party textures.

## Research findings

- The official projectM repository states that its core library is LGPL-2.1-or-later and warns that upstream and downstream projects may use different licenses. It also states that the core library does not ship preset packs. See [projectM licensing and preset boundaries](https://github.com/projectM-visualizer/projectm#license).
- The official MilkDrop 2.25c source archive linked by Ryan Geiss applies a three-clause BSD-style notice to the core `vis_milk2` files. The archive also contains separately copyrighted support headers and code, so any future source reuse requires a file-level review rather than relying on the headline license. See the [official MilkDrop source page](https://www.geisswerks.com/milkdrop/).
- The projectM Cream of the Crop repository states that almost all collected presets were released without a specific license and then assumes public-domain status from historical sharing. Absence of a license is not permission, so TheVisualizer will not rely on that assumption. See the pack's [license note](https://github.com/projectM-visualizer/presets-cream-of-the-crop/blob/master/LICENSE.md).
- The resolved Rust dependency graph reported license metadata for every external package during the review. Direct dependencies offer Apache-2.0 or other permissive choices.
- The Windows packaging workflow now creates a target-specific bundle from the locked resolved graph. It copies packaged upstream files first, falls back to the exact Cargo-pinned GitHub revision when a crate omits them, and fails closed when evidence is unavailable. The one recorded exception is `hexf-parse`, whose declared CC0-1.0 text is sourced from pinned SPDX license-list-data v3.26.0 because neither its crate nor upstream revision contains a license file.

## Compatibility policy

Future MilkDrop/projectM work must choose and record one of these paths per artifact:

1. Implement behavior independently from published format/behavior documentation.
2. Use a separately distributed projectM library while meeting its LGPL terms.
3. Accept user-supplied presets without redistributing them.
4. Bundle only presets and textures whose authors supplied an explicit compatible license.

Every imported file must record its source URL, author, version or revision, license, required notices, and any modifications. Unlabeled presets remain excluded. Legacy Winamp DLL loading remains outside v0.1.

## Release gate

The source repository is open-source under Apache-2.0, and the portable ZIP includes an automated target-specific third-party license bundle. It remains a local-test artifact until it passes the remaining Windows release validation.
