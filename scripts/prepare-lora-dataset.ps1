[CmdletBinding()]
param(
    [string]$Output = (Join-Path $PSScriptRoot "..\training-output\dataset")
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$manifest = Get-Content (Join-Path $repo "training\rights-manifest.json") -Raw | ConvertFrom-Json
$captions = Get-Content (Join-Path $repo "training\captions.json") -Raw | ConvertFrom-Json -AsHashtable
$approved = @($manifest.candidates | Where-Object approved)
if ($approved.Count -lt 10) {
    throw "Rights review is incomplete: approve at least ten owned/project-generated stills."
}

$outputPath = [System.IO.Path]::GetFullPath($Output)
$allowedRoot = [System.IO.Path]::GetFullPath((Join-Path $repo "training-output")).TrimEnd('\') + '\'
if (-not $outputPath.StartsWith($allowedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Dataset output must remain under training-output."
}
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null

$index = 0
foreach ($entry in $approved) {
    $source = [System.IO.Path]::GetFullPath((Join-Path $repo $entry.path))
    if (-not $source.StartsWith($repo.TrimEnd('\') + '\', [System.StringComparison]::OrdinalIgnoreCase) -or
        -not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Unsafe or missing training source: $($entry.path)"
    }
    if (-not $captions.ContainsKey($entry.path)) {
        throw "Missing caption: $($entry.path)"
    }
    $index++
    $stem = "tvizfield-{0:D3}" -f $index
    Copy-Item -LiteralPath $source -Destination (Join-Path $outputPath "$stem.png")
    Set-Content -LiteralPath (Join-Path $outputPath "$stem.txt") -Value $captions[$entry.path] -NoNewline
}
Write-Output "Prepared $index rights-approved image/caption pairs in $outputPath"
