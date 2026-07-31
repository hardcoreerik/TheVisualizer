[CmdletBinding()]
param(
    [string]$Target = "x86_64-pc-windows-msvc",
    [Parameter(Mandatory)]
    [string]$Destination
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$destinationPath = [System.IO.Path]::GetFullPath($Destination)
$distRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "dist"))
$distPrefix = $distRoot.TrimEnd('\') + '\'
if (-not $destinationPath.StartsWith($distPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Destination must stay inside $distRoot."
}
if (Test-Path -LiteralPath $destinationPath) {
    Remove-Item -LiteralPath $destinationPath -Recurse -Force
}
New-Item -ItemType Directory -Path $destinationPath -Force | Out-Null

$metadata = cargo metadata --format-version 1 --locked --filter-platform $Target | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw "cargo metadata failed."
}

$packagesById = @{}
foreach ($package in $metadata.packages) {
    $packagesById[$package.id] = $package
}
$packages = @(
    $metadata.resolve.nodes |
        ForEach-Object { $packagesById[$_.id] } |
        Where-Object { $null -ne $_.source } |
        Sort-Object name, version, id -Unique
)
if ($packages.Count -eq 0) {
    throw "No external packages were resolved for $Target."
}

$licenseNamePattern = '^(?i:LICENSE|LICENCE|COPYING|NOTICE|UNLICENSE)(?:[._-].*)?$'
$summary = @(
    "TheVisualizer third-party licenses"
    "Target: $Target"
    "Generated from Cargo.lock and cargo metadata; upstream fallbacks are pinned to Cargo source revisions."
    ""
)

$sourceCache = @{}
$temporaryRoot = Join-Path ([System.IO.Path]::GetTempPath()) "TheVisualizer-licenses-$([System.Guid]::NewGuid())"
New-Item -ItemType Directory -Path $temporaryRoot -Force | Out-Null

try {
foreach ($package in $packages) {
    if ([string]::IsNullOrWhiteSpace($package.license) -and [string]::IsNullOrWhiteSpace($package.license_file)) {
        throw "$($package.name) $($package.version) has no Cargo license metadata."
    }

    $packageRoot = Split-Path $package.manifest_path
    $licenseFiles = @(
        Get-ChildItem -LiteralPath $packageRoot -Recurse -File |
            Where-Object { $_.Name -match $licenseNamePattern }
    )
    $upstreamFiles = @()
    $canonicalLicenseUri = $null

    if ($licenseFiles.Count -eq 0) {
        $vcsInfoPath = Join-Path $packageRoot ".cargo_vcs_info.json"
        if (-not (Test-Path -LiteralPath $vcsInfoPath -PathType Leaf)) {
            throw "$($package.name) $($package.version) has no packaged license file or pinned VCS revision."
        }
        $vcsInfo = Get-Content -LiteralPath $vcsInfoPath -Raw | ConvertFrom-Json
        if ([string]::IsNullOrWhiteSpace($vcsInfo.git.sha1) -or $package.repository -notmatch 'github\.com/([^/]+)/([^/#]+)') {
            throw "$($package.name) $($package.version) has no usable pinned GitHub source."
        }

        $owner = $Matches[1]
        $repository = $Matches[2] -replace '\.git$', ''
        $revision = $vcsInfo.git.sha1
        $pathInVcs = [string]$vcsInfo.path_in_vcs

        $sourceKey = "$owner/$repository@$revision"
        if (-not $sourceCache.ContainsKey($sourceKey)) {
            $cachePath = Join-Path $temporaryRoot ($sourceCache.Count.ToString())
            $archivePath = "$cachePath.zip"
            Invoke-WebRequest `
                -Uri "https://codeload.github.com/$owner/$repository/zip/$revision" `
                -Headers @{ "User-Agent" = "TheVisualizer-license-collector" } `
                -OutFile $archivePath
            Expand-Archive -LiteralPath $archivePath -DestinationPath $cachePath
            $sourceRoot = Get-ChildItem -LiteralPath $cachePath -Directory | Select-Object -First 1
            if (-not $sourceRoot) {
                throw "The pinned source archive for $owner/$repository at $revision was empty."
            }
            $sourceCache[$sourceKey] = $sourceRoot.FullName
        }

        $sourceRoot = $sourceCache[$sourceKey]
        $upstreamFiles = @(
            Get-ChildItem -LiteralPath $sourceRoot -File |
                Where-Object { $_.Name -match $licenseNamePattern } |
                ForEach-Object {
                    [pscustomobject]@{ SourcePath = $_.FullName; RelativePath = $_.Name }
                }
            if (-not [string]::IsNullOrWhiteSpace($pathInVcs)) {
                $packageSourcePath = Join-Path $sourceRoot $pathInVcs
                if (Test-Path -LiteralPath $packageSourcePath -PathType Container) {
                    Get-ChildItem -LiteralPath $packageSourcePath -Recurse -File |
                        Where-Object { $_.Name -match $licenseNamePattern } |
                        ForEach-Object {
                            [pscustomobject]@{
                                SourcePath = $_.FullName
                                RelativePath = [System.IO.Path]::GetRelativePath($sourceRoot, $_.FullName)
                            }
                        }
                }
            }
        )
        $upstreamFiles = @($upstreamFiles | Sort-Object SourcePath -Unique)
        if ($upstreamFiles.Count -eq 0) {
            if ($package.license -eq "CC0-1.0") {
                $canonicalLicenseUri = "https://raw.githubusercontent.com/spdx/license-list-data/v3.26.0/text/CC0-1.0.txt"
            } else {
                throw "$($package.name) $($package.version) has no license files in its pinned upstream root or package path."
            }
        }
    }

    $folderName = "$($package.name)-$($package.version)" -replace '[<>:"/\\|?*]', '_'
    $packageDestination = Join-Path $destinationPath (Join-Path "licenses" $folderName)
    if (Test-Path -LiteralPath $packageDestination) {
        throw "Duplicate third-party license destination: $folderName."
    }
    New-Item -ItemType Directory -Path $packageDestination -Force | Out-Null

    $copied = @()
    foreach ($file in $licenseFiles) {
        $relativePath = [System.IO.Path]::GetRelativePath($packageRoot, $file.FullName)
        $outputPath = Join-Path $packageDestination $relativePath
        New-Item -ItemType Directory -Path (Split-Path $outputPath) -Force | Out-Null
        Copy-Item -LiteralPath $file.FullName -Destination $outputPath
        $copied += $relativePath.Replace('\', '/')
    }
    foreach ($file in $upstreamFiles) {
        $relativePath = "upstream/$($file.RelativePath.Replace('\', '/'))"
        $outputPath = Join-Path $packageDestination $relativePath
        New-Item -ItemType Directory -Path (Split-Path $outputPath) -Force | Out-Null
        Copy-Item -LiteralPath $file.SourcePath -Destination $outputPath
        $copied += $relativePath
    }
    if ($canonicalLicenseUri) {
        $relativePath = "canonical/CC0-1.0.txt"
        $outputPath = Join-Path $packageDestination $relativePath
        New-Item -ItemType Directory -Path (Split-Path $outputPath) -Force | Out-Null
        Invoke-WebRequest -Uri $canonicalLicenseUri -Headers @{ "User-Agent" = "TheVisualizer-license-collector" } -OutFile $outputPath
        $copied += $relativePath
    }

    $emptyFiles = @(Get-ChildItem -LiteralPath $packageDestination -Recurse -File | Where-Object Length -eq 0)
    if ($emptyFiles.Count -ne 0) {
        throw "$($package.name) $($package.version) produced an empty license file."
    }

    $summary += "$($package.name) $($package.version)"
    $summary += "License: $($package.license)"
    $summary += "Source: $($package.source)"
    if ($package.repository) {
        $summary += "Repository: $($package.repository)"
    }
    $summary += "Files: $($copied -join ', ')"
    $summary += ""
}
}
finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}

$summary | Set-Content -LiteralPath (Join-Path $destinationPath "THIRD-PARTY-LICENSES.txt")
Get-ChildItem -LiteralPath $destinationPath -Recurse -File |
    ForEach-Object { $_.LastWriteTimeUtc = [System.DateTime]::new(2000, 1, 1, 0, 0, 0, [System.DateTimeKind]::Utc) }
Write-Output "Collected licenses for $($packages.Count) target-resolved external packages."
