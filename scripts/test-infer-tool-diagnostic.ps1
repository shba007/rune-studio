param([switch]$PrepareOnly)

$ErrorActionPreference = 'Stop'
$studioRoot = Split-Path -Parent $PSScriptRoot
$inferRoot = [System.IO.Path]::GetFullPath((Join-Path $studioRoot '..\code-infer'))
$cargoDir = Join-Path $studioRoot '.cargo'
$cargoConfig = Join-Path $cargoDir 'config.toml'
$lockFile = Join-Path $studioRoot 'src-tauri\Cargo.lock'
$tempDir = Join-Path $env:TEMP ('rune-tool-diagnostic-' + [guid]::NewGuid().ToString())
$backupLock = Join-Path $tempDir 'Cargo.lock.original'
$diagnosticFile = Join-Path $tempDir 'tool-diagnostic.jsonl'

if (-not (Test-Path -LiteralPath (Join-Path $inferRoot 'Cargo.toml'))) {
    throw "Rune Infer checkout not found at $inferRoot"
}
if (Test-Path -LiteralPath $cargoConfig) {
    throw "Refusing to overwrite existing Cargo configuration: $cargoConfig"
}

New-Item -ItemType Directory -Path $tempDir | Out-Null
New-Item -ItemType Directory -Path $cargoDir -Force | Out-Null
Copy-Item -LiteralPath $lockFile -Destination $backupLock
$originalDiagnostic = $env:RUNE_INFER_TOOL_DIAGNOSTICS
$generatedLockHash = $null
try {
    $tomlPath = $inferRoot.Replace('\', '/').Replace('"', '\"')
    $config = "[patch.'https://github.com/shba007/rune-infer.git']`nrune-infer = { path = `"$tomlPath`" }`n"
    $file = [System.IO.File]::Open($cargoConfig, [System.IO.FileMode]::CreateNew)
    try {
        $bytes = [System.Text.UTF8Encoding]::new($false).GetBytes($config)
        $file.Write($bytes, 0, $bytes.Length)
    } finally { $file.Dispose() }
    Push-Location $studioRoot
    try {
        cargo update --offline --manifest-path src-tauri/Cargo.toml -p rune-infer
        if ($LASTEXITCODE -ne 0) { throw 'Could not prepare the temporary local Infer dependency.' }
        $generatedLockHash = (Get-FileHash -LiteralPath $lockFile -Algorithm SHA256).Hash
        $env:RUNE_INFER_TOOL_DIAGNOSTICS = $diagnosticFile
        Write-Host "Diagnostic metadata will be saved to: $diagnosticFile"
        Write-Host 'Run the Notepad task, then close Studio to finish.'
        if (-not $PrepareOnly) { bun run tauri:dev }
    } finally {
        Pop-Location
    }
} finally {
    $env:RUNE_INFER_TOOL_DIAGNOSTICS = $originalDiagnostic
    if (Test-Path -LiteralPath $cargoConfig) {
        $currentConfig = [System.IO.File]::ReadAllText($cargoConfig)
        if ($currentConfig -eq $config) { Remove-Item -LiteralPath $cargoConfig }
        else { Write-Warning "Cargo configuration changed during the test; left it at $cargoConfig" }
    }
    if ($generatedLockHash -and (Test-Path -LiteralPath $lockFile)) {
        $currentHash = (Get-FileHash -LiteralPath $lockFile -Algorithm SHA256).Hash
        if ($currentHash -eq $generatedLockHash) { Copy-Item -LiteralPath $backupLock -Destination $lockFile -Force }
        else { Write-Warning "Cargo.lock changed during the test; original backup is at $backupLock" }
    } elseif (Test-Path -LiteralPath $backupLock) {
        Write-Warning "Cargo.lock was not restored automatically; original backup is at $backupLock"
    }
    Write-Host "Diagnostic metadata: $diagnosticFile"
    if (Test-Path -LiteralPath $diagnosticFile) {
        Write-Host 'Copy the following diagnostic lines:'
        Get-Content -LiteralPath $diagnosticFile
    } else {
        Write-Host 'No diagnostic lines were recorded.'
    }
}
