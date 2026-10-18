# A throwaway target window for the M1 system tests (see scripts/m1-system-test.ps1).
#
# It is a magenta rectangle with a text box, always on top. It records, for the tests to read:
#   events.log   one line per mouse-down: down,<screen x>,<screen y>,<button>,<click count>
#   textbox.txt  the text box content, refreshed every 100 ms
# and it quits when a file named `quit` appears in the directory.
param(
    [Parameter(Mandatory)][string]$Dir,
    [Parameter(Mandatory)][int]$Left,
    [Parameter(Mandatory)][int]$Top,
    [Parameter(Mandatory)][int]$Width,
    [Parameter(Mandatory)][int]$Height
)

Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @"
using System.Runtime.InteropServices;
public static class Dpi { [DllImport("user32.dll")] public static extern bool SetProcessDPIAware(); }
"@
# Report physical pixels, like the real app does.
[void][Dpi]::SetProcessDPIAware()

New-Item -ItemType Directory -Force -Path $Dir | Out-Null
$events = Join-Path $Dir 'events.log'
$textFile = Join-Path $Dir 'textbox.txt'
$quit = Join-Path $Dir 'quit'
Remove-Item $events, $textFile, $quit -ErrorAction SilentlyContinue

$form = New-Object System.Windows.Forms.Form
$form.FormBorderStyle = 'None'
$form.StartPosition = 'Manual'
$form.Bounds = New-Object System.Drawing.Rectangle($Left, $Top, $Width, $Height)
$form.BackColor = [System.Drawing.Color]::FromArgb(255, 0, 255)
$form.TopMost = $true
$form.ShowInTaskbar = $false

$box = New-Object System.Windows.Forms.TextBox
$box.Multiline = $false
$box.Width = $Width - 40
$box.Left = 20
$box.Top = $Height - 50
$box.Font = New-Object System.Drawing.Font('Segoe UI', 14)
$form.Controls.Add($box)

$record = {
    param($sender, $e)
    $screen = [System.Windows.Forms.Cursor]::Position
    Add-Content -Path $events -Value ("down,{0},{1},{2},{3}" -f $screen.X, $screen.Y, $e.Button, $e.Clicks)
}
$form.add_MouseDown($record)

$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 100
$timer.add_Tick({
    Set-Content -Path $textFile -Value $box.Text -NoNewline -Encoding UTF8
    if (Test-Path $quit) { $form.Close() }
})
$timer.Start()

$form.Add_Shown({ $form.Activate(); $box.Focus() })
[System.Windows.Forms.Application]::Run($form)
