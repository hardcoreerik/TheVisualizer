[CmdletBinding()]
param(
    [string]$Model = (Join-Path $PSScriptRoot "..\target\ai-pack-source\DreamShaperXL_Lightning-SFW.safetensors"),
    [string]$Lora = (Join-Path $PSScriptRoot "..\training-output\checkpoints\tvizfield-sdxl-r16-step00001200.safetensors"),
    [string]$Worker = (Join-Path $PSScriptRoot "..\target\ai-worker\cuda-ninja\thevisualizer-ai-worker.exe")
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$output = Join-Path $repo "training-output\evaluation"
$prompts = @(
    @{ seed = 29072001; text = "tvizfield, photorealistic rain garden patio inside a modern glass house, lush wet foliage moving in wind, one clearly visible tree frog, cinematic natural light" },
    @{ seed = 29072002; text = "tvizfield, photorealistic black hole crossing, real star field, luminous accretion disk, gravitational lensing, cinematic deep space" },
    @{ seed = 29072003; text = "tvizfield, realistic comet over a layered nebula, long waveform tail made of cosmic dust and ionized light, deep parallax" },
    @{ seed = 29072004; text = "tvizfield, luminous green waveform filament, layered oscilloscope traces, black void, crisp emissive digital material" },
    @{ seed = 29072005; text = "tvizfield, cyan blue magenta spectrum analyzer city, glossy reflections, floating particles, futuristic concert visual" },
    @{ seed = 29072006; text = "tvizfield, continuously unfolding Mandelbrot coral reef, iridescent fractal branches, impossible mathematical detail, deep ocean light" },
    @{ seed = 29072007; text = "tvizfield, three dimensional particle forge, orbital plasma threads, toroidal flow, electric filaments, cinematic volumetric depth" },
    @{ seed = 29072008; text = "tvizfield, translucent frequency terrain, luminous cyan ridges, low camera, layered waveform topography, black horizon" },
    @{ seed = 29072009; text = "tvizfield, photorealistic DJ performing inside a holographic particle field, natural face and hands, dramatic club lighting" },
    @{ seed = 29072010; text = "tvizfield, full body contemporary dancer shaping a ribbon of spectral light, natural anatomy and hands, dark stage" },
    @{ seed = 29072011; text = "tvizfield, photorealistic raven flying through a field of stars and gravity waves, detailed feathers, cinematic motion" },
    @{ seed = 29072012; text = "tvizfield, monumental brutalist observatory surrounded by aurora waveforms, realistic materials, atmospheric night landscape" }
)
$negative = "text, watermark, logo, low resolution, blurry, malformed anatomy, extra fingers, duplicate subject"

foreach ($path in @($Model, $Lora, $Worker)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "Required file is missing: $path" }
}
New-Item -ItemType Directory -Path $output -Force | Out-Null

function Invoke-Set {
    param([string]$Name, [string]$LoraPath, [float]$Strength)

    $setPath = Join-Path $output $Name
    New-Item -ItemType Directory -Path $setPath -Force | Out-Null
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = (Resolve-Path $Worker).Path
    $start.WorkingDirectory = Split-Path (Resolve-Path $Worker).Path
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    if (-not $process.Start()) { throw "Could not start the AI worker." }

    try {
        $load = @{ type = "load"; model = (Resolve-Path $Model).Path }
        if ($LoraPath) { $load.lora = (Resolve-Path $LoraPath).Path }
        $process.StandardInput.WriteLine(($load | ConvertTo-Json -Compress))
        while ($true) {
            $message = $process.StandardOutput.ReadLine() | ConvertFrom-Json
            if ($message.type -eq "ready") { break }
            if ($message.type -eq "error") { throw $message.message }
        }

        $results = @()
        for ($index = 0; $index -lt $prompts.Count; $index++) {
            $id = $index + 1
            $image = Join-Path $setPath ("{0:D2}.png" -f $id)
            $request = @{
                type = "generate"
                request = @{
                    id = $id; prompt = $prompts[$index].text; negative_prompt = $negative
                    width = 768; height = 768; steps = 4; cfg_scale = 2.0
                    seed = $prompts[$index].seed; lora_strength = $Strength
                    output_path = [IO.Path]::GetFullPath($image)
                }
            }
            $process.StandardInput.WriteLine(($request | ConvertTo-Json -Depth 4 -Compress))
            while ($true) {
                $message = $process.StandardOutput.ReadLine() | ConvertFrom-Json
                if ($message.type -eq "result") {
                    $results += $message.result
                    Write-Host ("{0}: {1}/12 ({2} ms)" -f $Name, $id, $message.result.elapsed_ms)
                    break
                }
                if ($message.type -eq "error") { throw $message.message }
            }
        }
        return $results
    } finally {
        if (-not $process.HasExited) {
            $process.StandardInput.WriteLine('{"type":"unload"}')
            $process.StandardInput.Close()
            $process.WaitForExit(30000) | Out-Null
        }
        if (-not $process.HasExited) { $process.Kill($true) }
        $process.Dispose()
    }
}

function New-ContactSheet {
    param([int]$Start, [int]$Number)

    Add-Type -AssemblyName System.Drawing
    $sheet = [Drawing.Bitmap]::new(960, 2880)
    $graphics = [Drawing.Graphics]::FromImage($sheet)
    $graphics.Clear([Drawing.Color]::FromArgb(10, 12, 18))
    $titleFont = [Drawing.Font]::new("Arial", 17, [Drawing.FontStyle]::Bold)
    $promptFont = [Drawing.Font]::new("Arial", 11)
    $white = [Drawing.Brushes]::White
    try {
        for ($row = 0; $row -lt 6; $row++) {
            $index = $Start + $row
            foreach ($column in 0..1) {
                $name = if ($column -eq 0) { "baseline" } else { "lora-1200" }
                $path = Join-Path $output ("{0}\{1:D2}.png" -f $name, ($index + 1))
                $source = [Drawing.Image]::FromFile($path)
                try { $graphics.DrawImage($source, $column * 480, $row * 480 + 30, 480, 420) }
                finally { $source.Dispose() }
                $label = if ($column -eq 0) { "BASELINE" } else { "LORA 1200 @ 0.65" }
                $graphics.DrawString($label, $titleFont, $white, $column * 480 + 8, $row * 480 + 4)
            }
            $text = "{0:D2} · {1}" -f ($index + 1), $prompts[$index].text
            $graphics.DrawString($text, $promptFont, $white, [Drawing.RectangleF]::new(8, $row * 480 + 452, 944, 27))
        }
        $sheet.Save((Join-Path $output ("contact-sheet-{0}.png" -f $Number)), [Drawing.Imaging.ImageFormat]::Png)
    } finally {
        $promptFont.Dispose()
        $titleFont.Dispose()
        $graphics.Dispose()
        $sheet.Dispose()
    }
}

$baseline = Invoke-Set -Name "baseline" -LoraPath "" -Strength 0
$loraResults = Invoke-Set -Name "lora-1200" -LoraPath $Lora -Strength 0.65
New-ContactSheet -Start 0 -Number 1
New-ContactSheet -Start 6 -Number 2
@{
    checkpoint = (Resolve-Path $Lora).Path
    strength = 0.65
    prompts = $prompts
    baseline = $baseline
    lora = $loraResults
} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $output "evaluation-manifest.json")
Write-Host "Evaluation complete: $output"
