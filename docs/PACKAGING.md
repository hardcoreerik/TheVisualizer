# Windows Packaging

The current packaging proof creates a portable, local-test Windows x86_64 archive. It is not an installer, signed artifact, updater, or public release.

Run from the repository root:

```powershell
.\scripts\package-windows.ps1
```

The script:

1. requires Windows x86_64 and keeps generated output inside `dist`
2. builds the locked release workspace
3. stages the player, all bundled WGSL presets, every repository plugin manifest, the ABI-v1 example DLL, the ABI-v2 Creative Suite DLL, C/Rust SDK documentation, and project license/provenance files
4. collects license metadata and files for the locked Windows target dependency graph, failing if required evidence is missing
5. rewrites only the staged plugin manifest to use its package-relative DLL
6. adds an explicit local-test/public-release-gate notice and `SHA256SUMS.txt`
7. creates `dist/TheVisualizer-0.1.0-windows-x86_64-local-test.zip`

The collector copies license and notice files from each resolved crate. When a crate omits them, it downloads that package's exact Cargo-pinned GitHub revision and copies only repository-root and package-path license files. `hexf-parse` declares CC0-1.0 but supplies no text in either location, so the collector uses the canonical CC0-1.0 text pinned to SPDX license-list-data v3.26.0. Network access is therefore required when these upstream fallbacks are not already present in the crate.

The package is intentionally portable: extraction is installation, and deleting the extracted directory is uninstallation. The original project work is Apache-2.0. The runtime embeds the repository-owned application/window mark; a future public release still requires broader hardware validation, signed executable metadata/resource-icon work, and a decision on installer/signing/update infrastructure.
