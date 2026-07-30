[CmdletBinding()]
param(
    [ValidateSet("CUDA", "Vulkan")]
    [string]$Backend = "CUDA",
    [string]$Source = (Join-Path $PSScriptRoot "..\target\ai-worker\stable-diffusion.cpp")
)

$ErrorActionPreference = "Stop"
$vsDev = "C:\Program Files\Microsoft Visual Studio\18\Community\Common7\Tools\VsDevCmd.bat"
if (-not (Test-Path -LiteralPath $vsDev)) {
    throw "Visual Studio C++ build tools are required."
}
& $env:ComSpec /s /c "`"$vsDev`" -arch=x64 -host_arch=x64 >nul && set" |
    ForEach-Object {
        $name, $value = $_ -split "=", 2
        if ($name -ceq "PATH") {
            $env:Path = $value
        } elseif ($name -ine "Path" -and $name -and $value) {
            Set-Item -Path "Env:$name" -Value $value
        }
    }
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$sourcePath = (Resolve-Path $Source).Path
$commit = git -c "safe.directory=$($sourcePath.Replace('\', '/'))" -C $sourcePath rev-parse HEAD
if ($commit -ne "5ef4a7557df3ba7d1a0edc3fba4df0ffc99d07d2") {
    throw "stable-diffusion.cpp must be pinned to commit 5ef4a75."
}
$build = Join-Path $repo "target\ai-worker\$($Backend.ToLowerInvariant())-ninja"
$flags = if ($Backend -eq "CUDA") {
    @("-DSD_CUDA=ON", "-DSD_VULKAN=OFF")
} else {
    @("-DSD_CUDA=OFF", "-DSD_VULKAN=ON")
}
cmake -S (Join-Path $repo "ai-worker") -B $build -G Ninja `
    "-DSDCPP_SOURCE=$sourcePath" -DCMAKE_BUILD_TYPE=Release @flags
if ($LASTEXITCODE -ne 0) { throw "CMake configuration failed." }
cmake --build $build --config Release --target thevisualizer-ai-worker
if ($LASTEXITCODE -ne 0) { throw "AI worker build failed." }
Write-Output (Join-Path $build "thevisualizer-ai-worker.exe")
