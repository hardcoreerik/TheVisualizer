[CmdletBinding()]
param(
    [string]$Model = (Join-Path $PSScriptRoot "..\target\ai-pack-source\DreamShaperXL_Lightning-SFW.safetensors"),
    [string]$Trainer = (Join-Path $PSScriptRoot "..\target\lora-training\sd-scripts"),
    [string]$Python = (Join-Path $PSScriptRoot "..\target\lora-training\.venv\Scripts\python.exe")
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
& (Join-Path $repo "scripts\prepare-lora-dataset.ps1") | Write-Output
$dataset = (Resolve-Path (Join-Path $repo "training-output\dataset")).Path.Replace('\', '/')
$output = Join-Path $repo "training-output\checkpoints"
$cache = Join-Path $repo "target\lora-training\cache"
New-Item -ItemType Directory -Path $output, $cache -Force | Out-Null
$runtimeDataset = Join-Path $repo "training-output\dataset.toml"
@"
[general]
caption_extension = ".txt"

[[datasets]]
resolution = 1024
batch_size = 1
enable_bucket = true
min_bucket_reso = 512
max_bucket_reso = 1536
bucket_reso_steps = 64

  [[datasets.subsets]]
  image_dir = "$dataset"
  num_repeats = 1
"@ | Set-Content -LiteralPath $runtimeDataset

$env:HF_HOME = Join-Path $cache "huggingface"
$env:HF_HUB_OFFLINE = "1"
$env:TRANSFORMERS_OFFLINE = "1"
$env:TORCH_HOME = Join-Path $cache "torch"
$env:PYTHONUTF8 = "1"
$env:TOKENIZERS_PARALLELISM = "false"
Push-Location (Resolve-Path $Trainer)
try {
    & (Resolve-Path $Python) sdxl_train_network.py `
        --config_file (Join-Path $repo "training\lora-config.toml") `
        --dataset_config $runtimeDataset `
        --pretrained_model_name_or_path (Resolve-Path $Model) `
        --output_dir $output `
        --logging_dir (Join-Path $repo "training-output\logs")
    if ($LASTEXITCODE -ne 0) { throw "LoRA training failed." }
} finally {
    Pop-Location
}
