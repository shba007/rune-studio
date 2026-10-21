# Runs the M1.5 grounding probe against a local rune-infer server.
#
#   pwsh scripts/m15/run-probe.ps1 [-Modes <list>] [-PerScene <n>] [-Scenes <list>] [-Fresh]
#
# Modes are "model:convention" pairs, comma separated. Conventions:
#   norm    point as 0..1000 across the screenshot, through a click tool
#   px      point in pixels of the downscaled screenshot, through a click tool
#   ground  rune-infer's /v1/images/grounding endpoint
#   box     the bounding box of the element, 0..1000, through a locate tool (R4: how well can the
#           chat model draw boxes?). The report adds an IoU table. Example:
#           pwsh scripts/m15/run-probe.ps1 -Modes "gemma-4-12b-it:box" -Fresh
#
# Everything stays on this machine: scenes are rendered locally and sent only to rune-infer.
# It makes no mouse or keyboard input and does not touch your desktop.
param(
    [string]$Modes = 'qwen3.8-27b:norm',
    [int]$PerScene = 4,
    [string]$Scenes = '',
    [string]$Out = '',
    [string]$Base = 'http://127.0.0.1:8080',
    [switch]$Fresh,
    [switch]$Profile,
    [string]$Model = ''
)
$ErrorActionPreference = 'Stop'
$root = Split-Path (Split-Path $PSScriptRoot -Parent) -Parent
Set-Location $root

try { $health = Invoke-RestMethod "$Base/health" -TimeoutSec 5 } catch { throw "rune-infer is not reachable at $Base. Start it first." }
"rune-infer $($health.version) ($($health.backend))"

$sceneDir = Join-Path $root 'temp/m15/scenes'
if (-not (Test-Path (Join-Path $sceneDir 'index.json'))) { node scripts/m15/make-scenes.mjs $sceneDir; if ($LASTEXITCODE) { throw 'rendering the scenes failed' } }

if ($Profile) {
    $targetModel = if ($Model) { $Model } else { $Modes.Split(':')[0] }
    if (-not $targetModel) { throw 'Please specify a model using -Model <name> for profile calibration.' }
    $profileOut = if ($Out) { $Out } else { "temp/m15/$targetModel-profile.json" }
    $outPath = Join-Path $root $profileOut
    New-Item -ItemType Directory -Force -Path (Split-Path $outPath) | Out-Null
    if ($Fresh -and (Test-Path $outPath)) { Remove-Item $outPath }

    $env:RUNE_PROBE_DIR = $sceneDir
    $env:RUNE_PROBE_BASE = $Base
    $env:RUNE_PROBE_MODEL = $targetModel
    $env:RUNE_PROBE_PROFILE_OUT = $outPath

    Push-Location (Join-Path $root 'src-tauri')
    cargo test --lib m15_calibrate_profile -- --ignored --nocapture --test-threads=1
    $code = $LASTEXITCODE
    Pop-Location
    if ($code -ne 0) { throw 'Profile calibration failed' }
    "Suggested profile saved to: $outPath"
    return
}

$defaultOut = if ($Out) { $Out } else { 'temp/m15/results.jsonl' }
$outPath = Join-Path $root $defaultOut
New-Item -ItemType Directory -Force -Path (Split-Path $outPath) | Out-Null
if ($Fresh -and (Test-Path $outPath)) { Remove-Item $outPath }

$env:RUNE_PROBE_DIR = $sceneDir
$env:RUNE_PROBE_OUT = $outPath
$env:RUNE_PROBE_MODES = $Modes
$env:RUNE_PROBE_PER_SCENE = $PerScene
$env:RUNE_PROBE_BASE = $Base
if ($Scenes) { $env:RUNE_PROBE_SCENES = $Scenes } else { Remove-Item Env:RUNE_PROBE_SCENES -ErrorAction SilentlyContinue }

Push-Location (Join-Path $root 'src-tauri')
cargo test --lib m15_probe -- --ignored --nocapture --test-threads=1
$code = $LASTEXITCODE
Pop-Location
if ($code -ne 0) { throw 'the probe failed' }

node scripts/m15/report.mjs $outPath
