<#
.SYNOPSIS
    Builds and packages Tidemark into a portable Windows folder.

.DESCRIPTION
    Builds the Svelte frontend and release binaries (tm-watcher.exe, tm-server.exe,
    tidemark-ui.exe) and copies them into a self-contained portable directory.

.PARAMETER OutDir
    Destination folder for the portable build. Defaults to "dist/tidemark-portable".
#>
param(
    [string]$OutDir = "dist/tidemark-portable"
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $RepoRoot
try {
    Write-Host "[1/4] Building Svelte frontend..."
    Push-Location (Join-Path $RepoRoot "ui")
    try {
        npm run build
        if ($LASTEXITCODE -ne 0) { throw "Frontend build failed with exit code $LASTEXITCODE" }
        Write-Host "[2/4] Building release tidemark-ui binary with embedded frontend..."
        npx @tauri-apps/cli build --no-bundle
        if ($LASTEXITCODE -ne 0) { throw "Tauri build failed with exit code $LASTEXITCODE" }
    }
    finally {
        Pop-Location
    }

    Write-Host "[3/4] Building release Rust binaries (tm-watcher, tm-server)..."
    cargo build --release --bin tm-watcher --bin tm-server
    if ($LASTEXITCODE -ne 0) { throw "Cargo release build failed with exit code $LASTEXITCODE" }

    $TargetRelease = Join-Path $RepoRoot "target\release"
    $ResolvedOut = if ([System.IO.Path]::IsPathRooted($OutDir)) {
        $OutDir
    } else {
        Join-Path $RepoRoot $OutDir
    }

    Write-Host "[3/4] Assembling portable folder at $ResolvedOut ..."
    if (-not (Test-Path $ResolvedOut)) {
        New-Item -ItemType Directory -Path $ResolvedOut -Force | Out-Null
    }

    $Binaries = @("tm-watcher.exe", "tm-server.exe", "tidemark-ui.exe")
    foreach ($Bin in $Binaries) {
        $Src = Join-Path $TargetRelease $Bin
        if (-not (Test-Path $Src)) {
            throw "Expected binary not found: $Src"
        }
        Copy-Item -Path $Src -Destination (Join-Path $ResolvedOut $Bin) -Force
    }

    $ReadmeSrc = Join-Path $RepoRoot "README.md"
    if (Test-Path $ReadmeSrc) {
        Copy-Item -Path $ReadmeSrc -Destination (Join-Path $ResolvedOut "README.md") -Force
    }

    Write-Host "[4/4] Portable package created successfully:"
    Get-ChildItem -Path $ResolvedOut | ForEach-Object {
        Write-Host ("  - {0} ({1:N0} bytes)" -f $_.Name, $_.Length)
    }
    Write-Host ""
    Write-Host "Run tm-watcher.exe from $ResolvedOut to start the tray watcher (or tm-server.exe + tidemark-ui.exe)."
}
finally {
    Pop-Location
}
