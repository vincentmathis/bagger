#Requires -Version 5.1
<#
.SYNOPSIS
    One-liner install script for bagger (Scoop in Rust).
.DESCRIPTION
    Downloads a bagger release binary from GitHub and places it on PATH
    (per-user or system-wide).
.NOTES
    Windows-only.  Requires PowerShell 5.1+ and an internet connection.
.EXAMPLE
    PS> iwr -useb https://raw.githubusercontent.com/vincentmathis/bagger/main/scripts/install.ps1 | iex
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

# — fetch release asset URL ─────────────────────────────────────────
Write-Step "Resolving release ..."
$apiBase = "https://api.github.com/repos/vincentmathis/bagger/releases"
$release = if ($Tag -eq "latest") {
    irm "$apiBase/latest"
} else {
    $t = $Tag.TrimStart('v')
    try { irm "$apiBase/tags/v$t" } catch { irm "$apiBase/tags/$t" }
}
if (-not $release) { Die "Could not find release '$Tag'." }
try { $remote = [Version]($release.tag_name.TrimStart('v')) } catch { $remote = $null }

# Pick the asset matching this machine's architecture.
$arch = if ([System.Environment]::Is64BitOperatingSystem) {
    if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') { 'aarch64' } else { 'x86_64' }
} else { 'i686' }
$want = "bagger-$arch-pc-windows-msvc.zip"
$asset = @($release.assets | Where-Object { $_.name -eq $want })[0]
if (-not $asset) { Die "Release $($release.tag_name) has no asset '$want'." }

$zipUrl = $asset.browser_download_url
$zipTmp = "$env:TEMP\bagger-install.zip"

# — check existing ─────────────────────────────────────────────────────
if (-not $Force -and (Test-Path "$Dir\bagger.exe")) {
    try {
        $local = [Version]((Get-Item "$Dir\bagger.exe").VersionInfo.ProductVersion)
    } catch { $local = $null }
    if ($local -and $remote -and ($local -ge $remote)) {
        Write-Step "$Dir\bagger.exe (v$local) is up to date (remote v$remote). Use -Force to reinstall."
        exit 0
    }
}

# — download ───────────────────────────────────────────────────────────
Write-Step "Downloading $($asset.name) ..."
irm -Uri $zipUrl -OutFile $zipTmp

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
$verOut = (bagger --version 2>$null)
if ($verOut -match 'bagger (\S+)') { Write-Host "bagger $($Matches[1]) installed to $exe" -ForegroundColor Green }
else { Die "bagger.exe failed to start after install." }
Write-Host "Run 'bagger help' to get started." -ForegroundColor Green
