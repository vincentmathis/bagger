#Requires -Version 5.1
<#
.SYNOPSIS
    One-liner install script for bagger (Scoop in Rust).
.DESCRIPTION
    Downloads the latest bagger release binary from GitHub, places it on PATH
    (per-user or system-wide), and optionally initializes a Scoop-compatible
    ~/.local/share/bagger root so it can be used as a drop-in Scoop replacement.
.NOTES
    Windows-only.  Requires PowerShell 5.1+ and an internet connection.
.EXAMPLE
    PS> iwr -useb https://bagger.sh/install.ps1 | iex
#>
param(
    [switch]$System,          # install to Program Files (machine-wide, requires admin)
    [string]$Dir,             # override install directory
    [switch]$NoEnv,           # do not modify user/system PATH
    [switch]$Force,           # reinstall even if already present & newer
    [string]$Tag = "latest"   # GitHub tag / release to fetch
)

$ErrorActionPreference = "Stop"

# — helpers ──────────────────────────────────────────────────────────────
function Write-Step([string]$msg) { Write-Host "[bagger] " -NoNewline -ForegroundColor Cyan; Write-Host $msg }
function Die([string]$msg, [int]$code = 1) { Write-Host "[bagger] ERROR: $msg" -ForegroundColor Red; exit $code }

# — resolve install dir ─────────────────────────────────────────────────
if (-not $Dir) {
    if ($System) {
        if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
            Die "Admin rights are required for a system-wide (-System) installation."
        }
        $Dir = "$Env:ProgramFiles\bagger"
    } else {
        $Dir = "$Env:LOCALAPPDATA\bagger"
    }
}
$Dir = [System.IO.Path]::GetFullPath($Dir)

# — fetch latest release asset URL ──────────────────────────────────────
Write-Step "Resolving latest release ..."
$base = "https://github.com/vincentmathis/bagger/releases"
$api  = if ($Tag -eq "latest") { "$base/latest" } else { "$base/tag/$Tag" }
$r = irm "$api" -Headers @{"Accept"="application/json"} 2>$null
if (-not $r) { Write-Step "falling back to releases list"; $r = (irm -u "$base.atom" 2>$null) }
$asset = $r.assets | Where-Object { $_.name -match '^bagger-windows.*\.zip$' } |
         Sort-Object { [Version]($_.name -replace '.*windows-[^-]+-(?:x64|i686|arm64)-(\d+\.\d+\.\d+).*','$1') } -Desc |
         Select-Object -First 1
if (-not $asset) { Die "Could not find a suitable Windows binary in the release. Check $api manually." }

$zipUrl = $asset.browser_download_url
$zipTmp = "$env:TEMP\bagger-install.zip"
$verCmp = { param($a,$b) [Version]$a -ge [Version]$b }

# — check existing ─────────────────────────────────────────────────────
if (-not $Force -and (Test-Path "$Dir\bagger.exe")) {
    try {
        $local = [Version]((Get-Item "$Dir\bagger.exe").VersionInfo.ProductVersion)
    } catch { $local = [Version]"0.0.0" }
    try { $remote = [Version]($asset.name -replace '.*-(\d+\.\d+\.\d+).*','$1') } catch { $remote = $local }
    if ($local -ge $remote) {
        Write-Step "$Dir\bagger.exe (v$local) is up to date (remote v$remote). Use -Force to reinstall."
        exit 0
    }
}

# — download ───────────────────────────────────────────────────────────
Write-Step "Downloading $($asset.name) ..."
irm -u $zipUrl -OutFile $zipTmp

# — extract ────────────────────────────────────────────────────────────
Write-Step "Extracting to $Dir ..."
New-Item -ItemType Directory -Force -Path $Dir | Out-Null
Expand-Archive -LiteralPath $zipTmp -DestinationPath $Dir -Force

$exe = "$Dir\bagger.exe"
if (-not (Test-Path $exe)) { Die "Extraction did not yield bagger.exe" }
Remove-Item $zipTmp -ErrorAction SilentlyContinue

# — PATH ───────────────────────────────────────────────────────────────
if (-not $NoEnv) {
    $scope = if ($System) { "Machine" } else { "User" }
    $path = [Environment]::GetEnvironmentVariable("Path", $scope)
    if (-not ($path -split ';' | Where-Object { $_ -and (Get-Item $_ -ErrorAction SilentlyContinue) -and (Get-Item $_).FullName -eq $Dir })) {
        Write-Step "Adding $Dir to $scope PATH ..."
        $new = if ([string]::IsNullOrEmpty($path)) { $Dir } else { "$path;$Dir" }
        [Environment]::SetEnvironmentVariable("Path", $new, $scope)
    }
}

# — smoke test ─────────────────────────────────────────────────────────
Write-Step "Verifying install ..."
$env:Path = "$Dir;$env:Path"
if ((bagger --version 2>$null) -match '(\S+)') { Write-Host "bagger $($Matches[1]) installed to $exe" -ForegroundColor Green }
else { Die "bagger.exe failed to start after install." }
Write-Host "Run 'bagger help' to get started." -ForegroundColor Green
