[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$Source
)

$ErrorActionPreference = "Stop"
$expectedRevision = "395072d48b3ce7351ccb20a5fda54470591324df"
$expected = @(
    "Color Bars",
    "Color Schemes",
    "Heart",
    "Linear Gradient",
    "Noise",
    "Ridgelines",
    "Simplex Noise",
    "Worley Cells"
)
$sourcePath = (Resolve-Path -LiteralPath $Source).Path
$revisionOutput = git -c "safe.directory=$sourcePath" -C $sourcePath rev-parse HEAD
if ($LASTEXITCODE -ne 0) {
    throw "Could not read the ISF-Files revision."
}
$revision = $revisionOutput.Trim()
if ($revision -ne $expectedRevision) {
    throw "Expected ISF-Files revision $expectedRevision; found $revision."
}

$sourceFiles = @(Get-ChildItem -LiteralPath (Join-Path $sourcePath "ISF") -File -Filter "*.fs")
if ($sourceFiles.Count -ne 327) {
    throw "Expected 327 ISF sources; found $($sourceFiles.Count)."
}
$eligible = foreach ($file in $sourceFiles) {
    $sourceText = Get-Content -LiteralPath $file.FullName -Raw
    $start = $sourceText.IndexOf("/*{")
    $end = $sourceText.IndexOf("}*/")
    if ($start -lt 0 -or $end -le $start) {
        continue
    }
    try {
        $metadata = $sourceText.Substring($start + 2, $end - $start - 1) | ConvertFrom-Json
    }
    catch {
        continue
    }
    $textureInputs = @(
        $metadata.INPUTS |
            Where-Object { $_.TYPE -in @("image", "audio", "audioFFT") }
    )
    if ($null -eq $metadata.PASSES -and $textureInputs.Count -eq 0) {
        $file.BaseName
    }
}
$eligible = @($eligible | Sort-Object)
if (Compare-Object $expected $eligible) {
    throw "Eligible ISF source set changed: $($eligible -join ', ')."
}

Write-Output "ISF import audit passed: 327 inspected, 8 eligible, revision $revision."
