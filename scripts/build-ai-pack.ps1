[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$Model,
    [string]$Lora,
    [string]$CudaWorker = (Join-Path $PSScriptRoot "..\target\ai-worker\cuda-ninja\thevisualizer-ai-worker.exe"),
    [string]$VulkanWorker = (Join-Path $PSScriptRoot "..\target\ai-worker\vulkan-ninja\thevisualizer-ai-worker.exe"),
    [string]$Output = (Join-Path $PSScriptRoot "..\dist\TheVisualizer-AI-Pack")
)

$ErrorActionPreference = "Stop"
$expectedModel = "832e7bb2302c3cd67c818ca4fe9dcbedf696d3070dab5463127263ec4db9899f"
$modelPath = (Resolve-Path $Model).Path
$actualModel = (Get-FileHash -LiteralPath $modelPath -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actualModel -ne $expectedModel) {
    throw "Model checksum does not match DreamShaper XL Lightning SFW."
}

$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$outputPath = [System.IO.Path]::GetFullPath($Output)
$distRoot = [System.IO.Path]::GetFullPath((Join-Path $repo "dist")).TrimEnd('\') + '\'
if (-not $outputPath.StartsWith($distRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "AI Pack output must remain under dist."
}
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null
Copy-Item -LiteralPath $modelPath -Destination (Join-Path $outputPath "DreamShaperXL_Lightning-SFW.safetensors")

$workers = @{}
foreach ($pair in @(@("cuda", $CudaWorker), @("vulkan", $VulkanWorker))) {
    if (Test-Path -LiteralPath $pair[1] -PathType Leaf) {
        $name = "thevisualizer-ai-worker-$($pair[0]).exe"
        Copy-Item -LiteralPath $pair[1] -Destination (Join-Path $outputPath $name)
        $workers[$pair[0]] = @{
            path = $name
            sha256 = (Get-FileHash (Join-Path $outputPath $name) -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    }
}
if ($workers.Count -eq 0) { throw "Build at least one CUDA or Vulkan worker." }

$runtime = @()
if ($workers.ContainsKey("cuda")) {
    foreach ($name in @("cublas64_13.dll", "cublasLt64_13.dll")) {
        $source = Join-Path $env:CUDA_PATH "bin\x64\$name"
        if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
            throw "CUDA runtime dependency is missing: $name"
        }
        Copy-Item -LiteralPath $source -Destination (Join-Path $outputPath $name)
        $runtime += @{
            path = $name
            sha256 = (Get-FileHash (Join-Path $outputPath $name) -Algorithm SHA256).Hash.ToLowerInvariant()
        }
    }
}

$loraEntry = $null
if ($Lora) {
    $loraPath = (Resolve-Path $Lora).Path
    Copy-Item -LiteralPath $loraPath -Destination (Join-Path $outputPath "tvizfield.safetensors")
    $loraEntry = @{
        path = "tvizfield.safetensors"
        sha256 = (Get-FileHash (Join-Path $outputPath "tvizfield.safetensors") -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}
$manifest = [ordered]@{
    format = 1
    runner_commit = "5ef4a75"
    model = @{ path = "DreamShaperXL_Lightning-SFW.safetensors"; sha256 = $expectedModel }
    lora = $loraEntry
    runtime = $runtime
    workers = $workers
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outputPath "manifest.json")
Copy-Item -LiteralPath (Join-Path $repo "docs\AI_PACK.md") -Destination $outputPath
Copy-Item -LiteralPath (Join-Path $repo "target\ai-worker\stable-diffusion.cpp\LICENSE") `
    -Destination (Join-Path $outputPath "stable-diffusion.cpp-MIT-LICENSE.txt")
Copy-Item -LiteralPath (Join-Path $repo "third-party\dreamshaper-xl-lightning-OpenRAIL++-LICENSE.txt") `
    -Destination $outputPath
Write-Output "Offline AI Pack: $outputPath"
