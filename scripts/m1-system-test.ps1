# End-to-end check for M1: real mouse, keyboard and screen capture against a real window.
#
#   pwsh scripts/m1-system-test.ps1
#
# It opens a magenta target window (scripts/m1-target.ps1), runs the #[ignore]d tests in
# src-tauri/src/system_tests.rs against it, then closes the target. It moves the real mouse and
# types real keys for a few seconds, so leave the machine alone while it runs.

$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
$dir = Join-Path $env:TEMP 'rune-m1-target'

Add-Type -AssemblyName System.Windows.Forms
Add-Type -TypeDefinition @"
using System.Runtime.InteropServices;
public static class DpiAware { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }
"@
[void][DpiAware]::SetProcessDPIAware()   # physical pixels, like the app and the tests

$screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
$width = 520; $height = 320
$left = $screen.X + 360; $top = $screen.Y + 220

# A model-space coordinate (0..1000) at the centre of a physical pixel, matching the app's mapping.
function ToModel($pixel, $origin, $extent) { (($pixel - $origin) + 0.5) / $extent * 1000 }
$centerX = $left + [int]($width / 2);  $centerY = $top + [int]($height / 3)
$textX = $left + [int]($width / 2);    $textY = $top + $height - 50 + 16

$env:RUNE_TARGET_DIR = $dir
$env:RUNE_CLICK_X = ToModel $centerX $screen.X $screen.Width
$env:RUNE_CLICK_Y = ToModel $centerY $screen.Y $screen.Height
$env:RUNE_TEXT_X = ToModel $textX $screen.X $screen.Width
$env:RUNE_TEXT_Y = ToModel $textY $screen.Y $screen.Height
$env:RUNE_RECT_L = $left; $env:RUNE_RECT_T = $top; $env:RUNE_RECT_W = $width; $env:RUNE_RECT_H = $height

"Display: $($screen.Width)x$($screen.Height) at ($($screen.X),$($screen.Y)); target window at ($left,$top) ${width}x$height"

$target = Start-Process pwsh -PassThru -WindowStyle Hidden -ArgumentList @(
    '-NoProfile', '-STA', '-File', (Join-Path $PSScriptRoot 'm1-target.ps1'),
    '-Dir', $dir, '-Left', $left, '-Top', $top, '-Width', $width, '-Height', $height
)

try {
    $deadline = (Get-Date).AddSeconds(15)
    while (-not (Test-Path (Join-Path $dir 'textbox.txt'))) {
        if ((Get-Date) -gt $deadline) { throw 'The target window did not start.' }
        Start-Sleep -Milliseconds 200
    }
    Start-Sleep -Milliseconds 500   # let it come to the front
    $env:RUNE_TARGET_PID = $target.Id   # the tests refuse to type unless this process has focus

    Push-Location (Join-Path $root 'src-tauri')
    cargo test --lib system_tests -- --ignored --nocapture --test-threads=1
    $code = $LASTEXITCODE
    Pop-Location
}
finally {
    New-Item -ItemType File -Force -Path (Join-Path $dir 'quit') | Out-Null
    if (-not $target.WaitForExit(3000)) { Stop-Process -Id $target.Id -Force -ErrorAction SilentlyContinue }
}

if ($code -ne 0) { Write-Host "`nM1 system tests FAILED." -ForegroundColor Red; exit $code }
Write-Host "`nM1 system tests passed." -ForegroundColor Green
