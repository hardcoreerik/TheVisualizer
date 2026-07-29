[CmdletBinding()]
param(
    [ValidateSet("Debug", "Release")]
    [string]$Configuration = "Release",
    [string]$OutputRoot = (Join-Path $PSScriptRoot "..\dist")
)

$ErrorActionPreference = "Stop"

if (-not $IsWindows -or [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne "X64") {
    throw "The v0.1 package proof currently supports only Windows x86_64."
}

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$outputRootPath = [System.IO.Path]::GetFullPath($OutputRoot)
$repoPrefix = $repoRoot.TrimEnd('\') + '\'
if (-not $outputRootPath.StartsWith($repoPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "OutputRoot must stay inside $repoRoot."
}

$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed."
}
$package = $metadata.packages | Where-Object name -eq "thevisualizer" | Select-Object -First 1
if (-not $package) {
    throw "Could not find thevisualizer package metadata."
}

$configurationName = $Configuration.ToLowerInvariant()
$artifactName = "TheVisualizer-$($package.version)-windows-x86_64-local-test"
$stagePath = Join-Path $outputRootPath $artifactName
$zipPath = "$stagePath.zip"

foreach ($target in @($stagePath, $zipPath)) {
    $resolvedTarget = [System.IO.Path]::GetFullPath($target)
    $outputPrefix = $outputRootPath.TrimEnd('\') + '\'
    if (-not $resolvedTarget.StartsWith($outputPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to replace a target outside $outputRootPath."
    }
    if (Test-Path -LiteralPath $resolvedTarget) {
        Remove-Item -LiteralPath $resolvedTarget -Recurse -Force
    }
}

$buildArguments = @("build", "--workspace", "--locked")
if ($Configuration -eq "Release") {
    $buildArguments += "--release"
}
cargo @buildArguments
if ($LASTEXITCODE -ne 0) {
    throw "Workspace build failed."
}

$presetPath = Join-Path $stagePath "presets"
$assetPath = Join-Path $stagePath "assets"
$pluginPath = Join-Path $stagePath "plugins"
$sdkPath = Join-Path $stagePath "plugin-sdk"
$docsPath = Join-Path $stagePath "docs"
New-Item -ItemType Directory -Path $presetPath, $assetPath, $pluginPath, (Join-Path $sdkPath "include"), $docsPath -Force | Out-Null

Copy-Item -LiteralPath (Join-Path $repoRoot "target\$configurationName\thevisualizer.exe") -Destination $stagePath
Copy-Item -Path (Join-Path $repoRoot "presets\*.tvpreset") -Destination $presetPath
Copy-Item -LiteralPath (Join-Path $repoRoot "assets\living-photograph-greenhouse.png") -Destination $assetPath
Copy-Item -LiteralPath (Join-Path $repoRoot "assets\living-photograph-greenhouse-motion.webp") -Destination $assetPath
Copy-Item -LiteralPath (Join-Path $repoRoot "assets\README.md") -Destination $assetPath
Copy-Item -LiteralPath (Join-Path $repoRoot "target\$configurationName\thevisualizer_example_plugin.dll") -Destination $pluginPath
Copy-Item -LiteralPath (Join-Path $repoRoot "plugin-sdk\README.md") -Destination $sdkPath
Copy-Item -LiteralPath (Join-Path $repoRoot "plugin-sdk\include\thevisualizer_plugin.h") -Destination (Join-Path $sdkPath "include")
Copy-Item -LiteralPath (Join-Path $repoRoot "LICENSE") -Destination $stagePath
Copy-Item -LiteralPath (Join-Path $repoRoot "NOTICE") -Destination $stagePath
Copy-Item -LiteralPath (Join-Path $repoRoot "docs\LICENSING.md") -Destination $docsPath
& (Join-Path $repoRoot "scripts\collect-third-party-licenses.ps1") `
    -Target "x86_64-pc-windows-msvc" `
    -Destination (Join-Path $stagePath "third-party")

$pluginManifest = Get-Content -LiteralPath (Join-Path $repoRoot "plugins\example.tvplugin") -Raw
$pluginManifest = $pluginManifest -replace '(?m)^library=.*$', 'library=thevisualizer_example_plugin.dll'
if ($pluginManifest -notmatch '(?m)^library=thevisualizer_example_plugin\.dll$') {
    throw "Could not stage the package-relative plugin library path."
}
Set-Content -LiteralPath (Join-Path $pluginPath "example.tvplugin") -Value $pluginManifest -NoNewline

@"
TheVisualizer $($package.version) - Windows x86_64 local test

Run thevisualizer.exe. The player starts on the default Windows system-output loopback source.
Use S/M for automatic system/microphone input, L for the 20-mode Visual Library, H for labels,
up/down for GPU presets, I/O/F/P for instrument panels, B for borderless, F11 for fullscreen,
Tab for the overlay, and Escape to return or exit. Canvas clicks and drags directly edit the
active visual; right-click never opens a duplicate context menu.

The bundled native plugin remains disabled until you review its path and SHA-256 identity and
select Approve & Load. Native code runs with your user privileges and is not sandboxed.
Approval lasts only for the current run.

This is a portable package. Delete the extracted directory to uninstall it.
TheVisualizer's original work is Apache-2.0; see LICENSE, NOTICE, and docs\LICENSING.md.
Target-resolved dependency notices and upstream license files are under third-party.
"@ | Set-Content -LiteralPath (Join-Path $stagePath "README.txt")

@"
LOCAL TEST BUILD - NOT FOR REDISTRIBUTION

TheVisualizer's original work is licensed under Apache-2.0. This archive exists only for local
packaging and smoke testing because remaining Windows release validation is incomplete. It is not
a signed installer or a public release.
"@ | Set-Content -LiteralPath (Join-Path $stagePath "LOCAL-TEST-NOTICE.txt")

$required = @(
    "thevisualizer.exe",
    "assets\living-photograph-greenhouse.png",
    "assets\living-photograph-greenhouse-motion.webp",
    "assets\README.md",
    "plugins\example.tvplugin",
    "plugins\thevisualizer_example_plugin.dll",
    "plugin-sdk\README.md",
    "plugin-sdk\include\thevisualizer_plugin.h",
    "LICENSE",
    "NOTICE",
    "docs\LICENSING.md",
    "third-party\THIRD-PARTY-LICENSES.txt",
    "README.txt",
    "LOCAL-TEST-NOTICE.txt"
)
$required += Get-ChildItem -LiteralPath (Join-Path $repoRoot "presets") -Filter "*.tvpreset" -File |
    ForEach-Object { "presets\$($_.Name)" }
foreach ($relativePath in $required) {
    if (-not (Test-Path -LiteralPath (Join-Path $stagePath $relativePath) -PathType Leaf)) {
        throw "Package is missing $relativePath."
    }
}

$hashLines = Get-ChildItem -LiteralPath $stagePath -Recurse -File |
    Sort-Object FullName |
    ForEach-Object {
        $relativePath = [System.IO.Path]::GetRelativePath($stagePath, $_.FullName).Replace('\', '/')
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $relativePath"
    }
$hashLines | Set-Content -LiteralPath (Join-Path $stagePath "SHA256SUMS.txt")

Compress-Archive -Path (Join-Path $stagePath "*") -DestinationPath $zipPath -CompressionLevel Optimal

Write-Output "Package directory: $stagePath"
Write-Output "Package archive:   $zipPath"
