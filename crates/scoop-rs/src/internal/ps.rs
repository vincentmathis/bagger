use crate::error::Fallible;
use crate::internal;
use crate::package::Package;
use crate::Session;
use std::path::{Path, PathBuf};

/// Quote a value as a PowerShell single-quoted string literal.
pub(crate) fn ps_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Resolve the executable shipped by a Scoop helper app.
///
/// Mirrors upstream `Get-HelperPath`/`Get-AppFilePath`: the effective
/// (user or global) root wins, then the configured global root. Each root
/// prefers the `current` link and, when junctions are disabled, falls back
/// to the newest version directory containing the executable.
pub(crate) fn helper_exe(session: &Session, app: &str, rel_path: &str) -> Option<PathBuf> {
    let config = session.config();
    for base in [config.root_path(), config.global_path()] {
        let app_dir = base.join("apps").join(app);
        let current = app_dir.join("current").join(rel_path);
        if current.is_file() {
            return Some(current);
        }
        if config.no_junction() {
            let mut best: Option<(PathBuf, String)> = None;
            if let Ok(entries) = std::fs::read_dir(&app_dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.eq_ignore_ascii_case("current") {
                        continue;
                    }
                    let candidate = entry.path().join(rel_path);
                    if !candidate.is_file() {
                        continue;
                    }
                    let newer = match &best {
                        Some((_, best_name)) => {
                            internal::compare_versions(&name, best_name)
                                == std::cmp::Ordering::Greater
                        }
                        None => true,
                    };
                    if newer {
                        best = Some((candidate, name));
                    }
                }
            }
            if let Some((path, _)) = best {
                return Some(path);
            }
        }
    }
    None
}

/// Scoop-compatibility prelude prepended to every hook script.
///
/// Upstream Scoop executes manifest hook scripts (`pre_install`,
/// `installer.script`, `post_install`, `pre_uninstall`, ...) in a scope
/// populated with hook variables (`$dir`, `$version`, ...) and helper
/// functions (`Expand-7zipArchive`, ...). The function definitions below
/// are faithful ports of `lib/core.ps1`, `lib/decompress.ps1` and
/// `lib/system.ps1` from ScoopInstaller/Scoop, reduced to the subset that
/// is self-contained (no Scoop installation required on the machine).
const HOOK_PRELUDE: &str = r##"
function abort($msg, [int]$exit_code = 1) { Write-Host $msg -ForegroundColor Red; exit $exit_code }
function error($msg) { Write-Host "ERROR $msg" -ForegroundColor DarkRed }
function warn($msg) { Write-Host "WARN  $msg" -ForegroundColor DarkYellow }
function info($msg) { Write-Host "INFO  $msg" -ForegroundColor DarkGray }
function success($msg) { Write-Host $msg -ForegroundColor DarkGreen }
function fname($path) { Split-Path $path -Leaf }
function strip_ext($fname) { $fname -replace '\.[^\.]*$', '' }
function friendly_path($path) {
    $h = (Get-PSProvider 'FileSystem').Home
    if (!$h.EndsWith('\')) { $h += '\' }
    if ($h -eq '\') { return $path }
    return $path -replace ([Regex]::Escape($h)), '~\'
}
function is_admin {
    $admin = [security.principal.windowsbuiltinrole]::administrator
    $id = [security.principal.windowsidentity]::getcurrent()
    ([security.principal.windowsprincipal]($id)).isinrole($admin)
}
function Test-CommandAvailable {
    param([String]$Name)
    return [Boolean](Get-Command $Name -ErrorAction Ignore)
}
function ensure($dir) {
    if (!(Test-Path -Path $dir)) { New-Item -Path $dir -ItemType Directory | Out-Null }
    Convert-Path -Path $dir
}
function get_config($name, $default) {
    $name = $name.ToLowerInvariant()
    if ($null -eq $BaggerConfig[$name] -and $null -ne $default) { return $default }
    return $BaggerConfig[$name]
}
function Get-HelperPath {
    [CmdletBinding()]
    [OutputType([String])]
    param(
        [Parameter(Mandatory = $true, Position = 0, ValueFromPipeline = $true)]
        [ValidateSet('Git', '7zip', 'Lessmsi', 'Innounp', 'Dark', 'Aria2')]
        [String]$Helper
    )
    process {
        $p = $BaggerHelpers[$Helper]
        if ($p -and (Test-Path $p -PathType Leaf)) { return $p }
        if ($Helper -eq 'Git') {
            return (Get-Command git -CommandType Application -TotalCount 1 -ErrorAction Ignore).Source
        }
        return $null
    }
}
function Test-HelperInstalled {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true, Position = 0, ValueFromPipeline = $true)]
        [ValidateSet('7zip', 'Lessmsi', 'Innounp', 'Dark', 'Aria2')]
        [String]$Helper
    )
    return ![String]::IsNullOrWhiteSpace((Get-HelperPath -Helper $Helper))
}
function Publish-EnvVar {
    if (-not ('Win32.NativeMethods' -as [Type])) {
        Add-Type -Namespace Win32 -Name NativeMethods -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam, uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
    }
    $HWND_BROADCAST = [IntPtr] 0xffff
    $WM_SETTINGCHANGE = 0x1a
    $result = [UIntPtr]::Zero
    [Win32.NativeMethods]::SendMessageTimeout($HWND_BROADCAST, $WM_SETTINGCHANGE, [UIntPtr]::Zero, 'Environment', 2, 5000, [ref] $result) | Out-Null
}
function Get-EnvVar {
    param([string]$Name, [switch]$Global)
    $registerKey = if ($Global) { Get-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager' } else { Get-Item -Path 'HKCU:' }
    $envRegisterKey = $registerKey.OpenSubKey('Environment')
    $registryValueOption = [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames
    $envRegisterKey.GetValue($Name, $null, $registryValueOption)
}
function Set-EnvVar {
    param([string]$Name, [string]$Value, [switch]$Global)
    $registerKey = if ($Global) { Get-Item -Path 'HKLM:\SYSTEM\CurrentControlSet\Control\Session Manager' } else { Get-Item -Path 'HKCU:' }
    $envRegisterKey = $registerKey.OpenSubKey('Environment', $true)
    if ($null -eq $Value -or $Value -eq '') {
        if ($envRegisterKey.GetValue($Name)) { $envRegisterKey.DeleteValue($Name) }
    } else {
        $registryValueKind = if ($Value.Contains('%')) { [Microsoft.Win32.RegistryValueKind]::ExpandString }
        elseif ($envRegisterKey.GetValue($Name)) { $envRegisterKey.GetValueKind($Name) }
        else { [Microsoft.Win32.RegistryValueKind]::String }
        $envRegisterKey.SetValue($Name, $Value, $registryValueKind)
    }
    Publish-EnvVar
}
function Split-PathLikeEnvVar {
    param([string[]]$Pattern, [string]$Path)
    if ($null -eq $Path -and $Path -eq '') { return $null, $null }
    $splitPattern = $Pattern.Split(';', [System.StringSplitOptions]::RemoveEmptyEntries)
    $splitPath = $Path.Split(';', [System.StringSplitOptions]::RemoveEmptyEntries)
    $inPath = @()
    foreach ($p in $splitPattern) {
        $inPath += $splitPath.Where({ $_ -like $p })
        $splitPath = $splitPath.Where({ $_ -notlike $p })
    }
    return ($inPath -join ';'), ($splitPath -join ';')
}
function Add-Path {
    param([string[]]$Path, [string]$TargetEnvVar = 'PATH', [switch]$Global, [switch]$Force, [switch]$Quiet)
    $inPath, $strippedPath = Split-PathLikeEnvVar $Path (Get-EnvVar -Name $TargetEnvVar -Global:$Global)
    if (!$inPath -or $Force) {
        if (!$Quiet) { $Path | ForEach-Object { Write-Host "Adding $(friendly_path $_) to $(if ($Global) {'global'} else {'your'}) path." } }
        Set-EnvVar -Name $TargetEnvVar -Value ((@($Path) + $strippedPath) -join ';') -Global:$Global
    }
    $inPath, $strippedPath = Split-PathLikeEnvVar $Path $env:PATH
    if (!$inPath -or $Force) { $env:PATH = (@($Path) + $strippedPath) -join ';' }
}
function Remove-Path {
    param([string[]]$Path, [string]$TargetEnvVar = 'PATH', [switch]$Global, [switch]$Quiet, [switch]$PassThru)
    $inPath, $strippedPath = Split-PathLikeEnvVar $Path (Get-EnvVar -Name $TargetEnvVar -Global:$Global)
    if ($inPath) {
        if (!$Quiet) { $Path | ForEach-Object { Write-Host "Removing $(friendly_path $_) from $(if ($Global) {'global'} else {'your'}) path." } }
        Set-EnvVar -Name $TargetEnvVar -Value $strippedPath -Global:$Global
    }
    $inSessionPath, $strippedPath = Split-PathLikeEnvVar $Path $env:PATH
    if ($inSessionPath) { $env:PATH = $strippedPath }
    if ($PassThru) { return $inPath }
}
function Show-DeprecatedWarning {
    param($Invocation, [String]$New)
    warn ('"{0}" will be deprecated. Please change your code/manifest to use "{1}"' -f $Invocation.MyCommand.Name, $New)
}
function Invoke-ExternalCommand {
    [CmdletBinding()]
    [OutputType([Boolean])]
    param(
        [Parameter(Mandatory = $true, Position = 0)]
        [String]$FilePath,
        [Parameter(Position = 1)]
        [String[]]$ArgumentList,
        [Alias('Msg')]
        [String]$Activity,
        [Alias('cec')]
        [Hashtable]$ContinueExitCodes,
        [Alias('Log')]
        [String]$LogPath
    )
    if ($Activity) { Write-Host "$Activity " -NoNewline }
    try {
        if ($LogPath -and ($FilePath -match '^msiexec(.exe)?$')) {
            $ArgumentList += "/lwe `"$LogPath`""
            & $FilePath @ArgumentList
        } elseif ($LogPath) {
            & $FilePath @ArgumentList 2>&1 | Out-File -FilePath $LogPath -Encoding utf8
        } else {
            & $FilePath @ArgumentList
        }
    } catch {
        if ($Activity) { Write-Host 'Error.' -ForegroundColor DarkRed }
        error $_.Exception.Message
        return $false
    }
    if ($LASTEXITCODE -ne 0) {
        if ($ContinueExitCodes -and ($ContinueExitCodes.ContainsKey($LASTEXITCODE))) {
            if ($Activity) { Write-Host 'Done.' -ForegroundColor DarkYellow }
            warn $ContinueExitCodes[$LASTEXITCODE]
            return $true
        }
        if ($Activity) { Write-Host 'Error.' -ForegroundColor DarkRed }
        error "Exit code was $LASTEXITCODE!"
        return $false
    }
    if ($Activity) { Write-Host 'Done.' -ForegroundColor Green }
    return $true
}
function movedir($from, $to) {
    $from = $from.TrimEnd('\')
    $to = $to.TrimEnd('\')
    $proc = Start-Process -FilePath 'robocopy.exe' -ArgumentList "`"$from`"", "`"$to`"", '/e', '/move' -NoNewWindow -Wait -PassThru
    if ($proc.ExitCode -ge 8) { throw "Could not move '$(fname $from)'! (robocopy error $($proc.ExitCode))" }
    1..10 | ForEach-Object { if (Test-Path $from) { Start-Sleep -Milliseconds 100 } }
}
function Expand-7zipArchive {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true, Position = 0, ValueFromPipeline = $true)]
        [String]$Path,
        [Parameter(Position = 1)]
        [String]$DestinationPath = (Split-Path $Path),
        [String]$ExtractDir,
        [Parameter(ValueFromRemainingArguments = $true)]
        [String]$Switches,
        [ValidateSet('All', 'Skip', 'Rename')]
        [String]$Overwrite,
        [Switch]$Removal
    )
    if ((get_config USE_EXTERNAL_7ZIP)) {
        try { $7zPath = (Get-Command '7z' -CommandType Application -ErrorAction Stop | Select-Object -First 1).Source }
        catch { abort "`nCannot find external 7-Zip (7z.exe) while 'use_external_7zip' is 'true'!`nRun 'bagger config set use_external_7zip false' or install 7-Zip manually and try again." }
    } else {
        $7zPath = Get-HelperPath -Helper 7zip
    }
    if (!$7zPath) { abort "Cannot find 7-Zip executable! Install the '7zip' app or set 'use_external_7zip' and try again." }
    $LogPath = "$(Split-Path $Path)\7zip.log"
    $DestinationPath = $DestinationPath.TrimEnd('\')
    $ArgList = @('x', $Path, "-o$DestinationPath", '-xr!*.nsis', '-y')
    $IsTar = ((strip_ext $Path) -match '\.tar$') -or ($Path -match '\.t[abgpx]z2?$')
    if (!$IsTar -and $ExtractDir) { $ArgList += "-ir!$ExtractDir\*" }
    if ($Switches) { $ArgList += (-split $Switches) }
    switch ($Overwrite) {
        'All' { $ArgList += '-aoa' }
        'Skip' { $ArgList += '-aos' }
        'Rename' { $ArgList += '-aou' }
    }
    $Status = Invoke-ExternalCommand $7zPath $ArgList -LogPath $LogPath
    if (!$Status) { abort "Failed to extract files from $Path.`nLog file:`n  $(friendly_path $LogPath)" }
    if ($IsTar) {
        $Status = Invoke-ExternalCommand $7zPath @('l', $Path) -LogPath $LogPath
        if ($Status) {
            $TarFile = (Select-String -Path $LogPath -Pattern '[^ ]*tar$').Matches.Value
            Expand-7zipArchive -Path "$DestinationPath\$TarFile" -DestinationPath $DestinationPath -ExtractDir $ExtractDir -Removal
        } else {
            abort "Failed to list files in $Path.`nNot a 7-Zip supported archive file."
        }
    }
    if (!$IsTar -and $ExtractDir) {
        movedir "$DestinationPath\$ExtractDir" $DestinationPath | Out-Null
        $ExtractDirTopPath = [string] "$DestinationPath\$($ExtractDir -replace '[\\/].*')"
        if ((Get-ChildItem -Path $ExtractDirTopPath -Force -ErrorAction Ignore).Count -eq 0) {
            Remove-Item -Path $ExtractDirTopPath -Recurse -Force -ErrorAction Ignore
        }
    }
    if (Test-Path $LogPath) { Remove-Item $LogPath -Force }
    if ($Removal) {
        if (($Path -replace '.*\.([^\.]*)$', '$1') -eq '001') {
            Get-ChildItem "$($Path -replace '\.[^\.]*$', '').???" | Remove-Item -Force
        } elseif (($Path -replace '.*\.part(\d+)\.rar$', '$1')[-1] -eq '1') {
            Get-ChildItem "$($Path -replace '\.part(\d+)\.rar$', '').part*.rar" | Remove-Item -Force
        } else {
            Remove-Item $Path -Force
        }
    }
}
function Expand-ZstdArchive {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true, Position = 0, ValueFromPipeline = $true)]
        [String]$Path,
        [Parameter(Position = 1)]
        [String]$DestinationPath = (Split-Path $Path),
        [String]$ExtractDir,
        [Parameter(ValueFromRemainingArguments = $true)]
        [String]$Switches,
        [Switch]$Removal
    )
    Show-DeprecatedWarning $MyInvocation 'Expand-7zipArchive'
    Expand-7zipArchive -Path $Path -DestinationPath $DestinationPath -ExtractDir $ExtractDir -Switches $Switches -Removal:$Removal
}
function Expand-MsiArchive {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true, Position = 0, ValueFromPipeline = $true)]
        [String]$Path,
        [Parameter(Position = 1)]
        [String]$DestinationPath = (Split-Path $Path),
        [String]$ExtractDir,
        [Parameter(ValueFromRemainingArguments = $true)]
        [String]$Switches,
        [Switch]$Removal
    )
    $DestinationPath = $DestinationPath.TrimEnd('\')
    if ($ExtractDir) {
        $OriDestinationPath = $DestinationPath
        $DestinationPath = "$DestinationPath\_tmp"
    }
    if ((get_config USE_LESSMSI)) {
        $MsiPath = Get-HelperPath -Helper Lessmsi
        if (!$MsiPath) { abort "Cannot find Lessmsi executable! Install the 'lessmsi' app or unset 'use_lessmsi' and try again." }
        $ArgList = @('x', $Path, "$DestinationPath\")
    } else {
        $MsiPath = 'msiexec.exe'
        $ArgList = @('/a', $Path, '/qn', "TARGETDIR=$DestinationPath\SourceDir")
    }
    $LogPath = "$(Split-Path $Path)\msi.log"
    if ($Switches) { $ArgList += (-split $Switches) }
    $Status = Invoke-ExternalCommand $MsiPath $ArgList -LogPath $LogPath
    if (!$Status) { abort "Failed to extract files from $Path.`nLog file:`n  $(friendly_path $LogPath)" }
    if ($ExtractDir -and (Test-Path "$DestinationPath\SourceDir")) {
        movedir "$DestinationPath\SourceDir\$ExtractDir" $OriDestinationPath | Out-Null
        Remove-Item $DestinationPath -Recurse -Force
    } elseif ($ExtractDir) {
        movedir "$DestinationPath\$ExtractDir" $OriDestinationPath | Out-Null
        Remove-Item $DestinationPath -Recurse -Force
    } elseif (Test-Path "$DestinationPath\SourceDir") {
        movedir "$DestinationPath\SourceDir" $DestinationPath | Out-Null
    }
    if (($DestinationPath -ne (Split-Path $Path)) -and (Test-Path "$DestinationPath\$(fname $Path)")) {
        Remove-Item "$DestinationPath\$(fname $Path)" -Force
    }
    if (Test-Path $LogPath) { Remove-Item $LogPath -Force }
    if ($Removal) { Remove-Item $Path -Force }
}
function Expand-InnoArchive {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true, Position = 0, ValueFromPipeline = $true)]
        [String]$Path,
        [Parameter(Position = 1)]
        [String]$DestinationPath = (Split-Path $Path),
        [String]$ExtractDir,
        [Parameter(ValueFromRemainingArguments = $true)]
        [String]$Switches,
        [Switch]$Removal
    )
    $LogPath = "$(Split-Path $Path)\innounp.log"
    $ArgList = @('-x', "-d$DestinationPath", $Path, '-y')
    switch -Regex ($ExtractDir) {
        '^[^{].*' { $ArgList += "-c{app}\$ExtractDir" }
        '^{.*' { $ArgList += "-c$ExtractDir" }
        Default { $ArgList += '-c{app}' }
    }
    if ($Switches) { $ArgList += (-split $Switches) }
    $InnounpPath = Get-HelperPath -Helper Innounp
    if (!$InnounpPath) { abort "Cannot find Innounp executable! Install the 'innounp' app and try again." }
    $Status = Invoke-ExternalCommand $InnounpPath $ArgList -LogPath $LogPath
    if (!$Status) { abort "Failed to extract files from $Path.`nLog file:`n  $(friendly_path $LogPath)" }
    if (Test-Path $LogPath) { Remove-Item $LogPath -Force }
    if ($Removal) { Remove-Item $Path -Force }
}
function Expand-ZipArchive {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true, Position = 0, ValueFromPipeline = $true)]
        [String]$Path,
        [Parameter(Position = 1)]
        [String]$DestinationPath = (Split-Path $Path),
        [String]$ExtractDir,
        [Switch]$Removal
    )
    if ($ExtractDir) {
        $OriDestinationPath = $DestinationPath
        $DestinationPath = "$DestinationPath\_tmp"
    }
    $oldProgressPreference = $ProgressPreference
    $global:ProgressPreference = 'SilentlyContinue'
    Microsoft.PowerShell.Archive\Expand-Archive -Path $Path -DestinationPath $DestinationPath -Force
    $global:ProgressPreference = $oldProgressPreference
    if ($ExtractDir) {
        movedir "$DestinationPath\$ExtractDir" $OriDestinationPath | Out-Null
        Remove-Item $DestinationPath -Recurse -Force
    }
    if ($Removal) { Remove-Item $Path -Force }
}
function Expand-DarkArchive {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory = $true, Position = 0, ValueFromPipeline = $true)]
        [String]$Path,
        [Parameter(Position = 1)]
        [String]$DestinationPath = (Split-Path $Path),
        [Parameter(ValueFromRemainingArguments = $true)]
        [String]$Switches,
        [Switch]$Removal
    )
    $LogPath = "$(Split-Path $Path)\dark.log"
    $DarkPath = Get-HelperPath -Helper Dark
    if (!$DarkPath) { abort "Cannot find Dark (WiX) executable! Install the 'dark' app and try again." }
    if ((Split-Path $DarkPath -Leaf) -eq 'wix.exe') {
        $ArgList = @('burn', 'extract', $Path, '-out', $DestinationPath, '-outba', "$DestinationPath\UX")
    } else {
        $ArgList = @('-nologo', '-x', $DestinationPath, $Path)
    }
    if ($Switches) { $ArgList += (-split $Switches) }
    $Status = Invoke-ExternalCommand $DarkPath $ArgList -LogPath $LogPath
    if (!$Status) { abort "Failed to extract files from $Path.`nLog file:`n  $(friendly_path $LogPath)" }
    if (Test-Path "$DestinationPath\WixAttachedContainer") {
        Rename-Item "$DestinationPath\WixAttachedContainer" 'AttachedContainer' -ErrorAction Ignore
    } else {
        if (Test-Path "$DestinationPath\AttachedContainer\a0") {
            $Xml = [xml](Get-Content -Raw "$DestinationPath\UX\manifest.xml" -Encoding utf8)
            $Xml.BurnManifest.UX.Payload | ForEach-Object {
                Rename-Item "$DestinationPath\UX\$($_.SourcePath)" $_.FilePath -ErrorAction Ignore
            }
            $Xml.BurnManifest.Payload | ForEach-Object {
                Rename-Item "$DestinationPath\AttachedContainer\$($_.SourcePath)" $_.FilePath -ErrorAction Ignore
            }
        }
    }
    if (Test-Path $LogPath) { Remove-Item $LogPath -Force }
    if ($Removal) { Remove-Item $Path -Force }
}
function uninstall_rm($item) {
    if (Test-Path $item) { Remove-Item $item -Recurse -Force }
}
"##;

/// Build the hook-script prelude: variables plus [`HOOK_PRELUDE`].
///
/// The variables mirror the locals visible to hook scripts in upstream
/// Scoop (`install_app`/`uninstall_app` scope): `$dir` is the version
/// directory being committed, `$version`/`$architecture` describe the
/// package, `$cmd` mirrors the operation in progress, and `$global` is a
/// real boolean (unlike the legacy stringly-typed environment fallback).
pub(crate) fn build_prelude_for(
    session: &Session,
    package: &Package,
    cmd: &str,
    working_dir: &Path,
) -> String {
    let config = session.config();
    let root = config.root_path();
    let version = package.version().to_string();
    let app_dir = root.join("apps").join(package.name());
    let buckets_dir = root.join("buckets");
    let bucket_dir = buckets_dir.join(package.bucket());

    let mut out = String::new();
    out.push_str(&format!("$cmd = {}\n", ps_quote(cmd)));
    out.push_str(&format!(
        "$dir = {}\n",
        ps_quote(&working_dir.to_string_lossy())
    ));
    out.push_str(&format!("$version = {}\n", ps_quote(&version)));
    out.push_str(&format!(
        "$architecture = {}\n",
        ps_quote(&crate::operation::resolved_arch(package))
    ));
    out.push_str(&format!("$app = {}\n", ps_quote(package.name())));
    out.push_str(&format!("$bucket = {}\n", ps_quote(package.bucket())));
    out.push_str(&format!(
        "$bucketsdir = {}\n",
        ps_quote(&buckets_dir.to_string_lossy())
    ));
    out.push_str(&format!(
        "$bucketdir = {}\n",
        ps_quote(&bucket_dir.to_string_lossy())
    ));
    let fnames: Vec<String> = package
        .download_filenames()
        .iter()
        .map(|f| ps_quote(f))
        .collect();
    out.push_str(&format!("$fname = @({})\n", fnames.join(", ")));
    out.push_str(&format!(
        "$global = {}\n",
        if config.is_global_scope() {
            "$true"
        } else {
            "$false"
        }
    ));
    out.push_str(&format!(
        "$original_dir = {}\n",
        ps_quote(&app_dir.join(&version).to_string_lossy())
    ));
    out.push_str(&format!(
        "$persist_dir = {}\n",
        ps_quote(&root.join("persist").join(package.name()).to_string_lossy())
    ));
    let scoop_path_var = match config.use_isolated_path() {
        None => "PATH".to_string(),
        Some(crate::config::IsolatedPath::Boolean(true)) => "SCOOP_PATH".to_string(),
        Some(crate::config::IsolatedPath::Boolean(false)) => "PATH".to_string(),
        Some(crate::config::IsolatedPath::Named(name)) => name.to_uppercase(),
    };
    out.push_str(&format!(
        "$scoopPathEnvVar = {}\n",
        ps_quote(&scoop_path_var)
    ));

    let helper = |app: &str, rel: &str| {
        helper_exe(session, app, rel)
            .map(|p| ps_quote(&p.to_string_lossy()))
            .unwrap_or_else(|| "''".to_string())
    };
    let innounp = helper("innounp-unicode", "innounp.exe");
    let innounp = if innounp == "''" {
        helper("innounp", "innounp.exe")
    } else {
        innounp
    };
    let dark = helper("dark", "dark.exe");
    let dark = if dark == "''" {
        helper("wixtoolset", "wix.exe")
    } else {
        dark
    };
    let git = helper("git", "mingw64/bin/git.exe");
    let git = if git == "''" {
        helper("git", "mingw32/bin/git.exe")
    } else {
        git
    };
    out.push_str(&format!(
        "$BaggerHelpers = @{{ 'Git' = {git}; '7zip' = {seven}; 'Lessmsi' = {less}; 'Innounp' = {innounp}; 'Dark' = {dark}; 'Aria2' = {aria} }}\n",
        seven = helper("7zip", "7z.exe"),
        less = helper("lessmsi", "lessmsi.exe"),
        aria = helper("aria2", "aria2c.exe"),
    ));
    out.push_str(&format!(
        "$BaggerConfig = @{{ 'use_external_7zip' = {ext}; 'use_lessmsi' = {less}; 'no_junction' = {noj} }}\n",
        ext = if config.use_external_7zip() {
            "$true"
        } else {
            "$false"
        },
        less = if config.use_lessmsi() {
            "$true"
        } else {
            "$false"
        },
        noj = if config.no_junction() {
            "$true"
        } else {
            "$false"
        },
    ));

    out.push_str(HOOK_PRELUDE);
    out
}

/// Build the Scoop PowerShell execution context variables.
///
/// Returns a list of `key= value` pairs that are set as environment variables
/// when invoking PowerShell, so that scripts can reference them.
fn build_context_variables(
    session: &Session,
    package: &Package,
    cmd: &str,
    working_dir: &Path,
) -> Vec<(&'static str, String)> {
    let config = session.config();
    let root_path = config.root_path().to_string_lossy().to_string();
    let bucket = package.bucket();
    let bucket_path = config
        .root_path()
        .join("buckets")
        .join(bucket)
        .to_string_lossy()
        .to_string();

    let version = package.version().to_string();
    let arch = crate::operation::resolved_arch(package);

    let dir = working_dir.to_string_lossy().to_string();

    let global = if config.is_global_scope() { "1" } else { "0" }.to_string();

    vec![
        ("SCOOP", root_path),
        (
            "SCOOP_GLOBAL",
            config.global_path().to_string_lossy().to_string(),
        ),
        ("SCOOP_BUCKET", bucket.to_string()),
        ("SCOOP_BUCKET_DIR", bucket_path.clone()),
        ("SCOOP_PACKAGE", package.name().to_string()),
        ("SCOOP_VERSION", version.clone()),
        ("SCOOP_ARCH", arch.clone()),
        ("SCOOP_CMD", cmd.to_string()),
        ("dir", dir),
        ("bucketdir", bucket_path),
        ("bucket", bucket.to_string()),
        ("version", version),
        ("architecture", arch),
        ("fname", package.name().to_string()),
        ("global", global),
    ]
}

/// Execute a PowerShell script with the Scoop execution context.
///
/// This function builds the appropriate environment variables and invokes
/// `powershell.exe` to run the given script lines.
///
/// # Arguments
///
/// * `session` - The Scoop session
/// * `package` - The package being installed
/// * `cmd` - The command being run (e.g., "install", "uninstall")
/// * `script` - The script lines to execute
/// * `working_dir` - The working directory for the script (typically the app dir)
pub fn invoke_script(
    session: &Session,
    package: &Package,
    cmd: &str,
    script: &[&str],
    working_dir: &Path,
) -> Fallible<()> {
    // Prepend the Scoop-compatibility prelude (hook variables + helpers)
    // so manifest scripts observe the same scope as under upstream Scoop.
    let ps_script = format!(
        "{}{}",
        build_prelude_for(session, package, cmd, working_dir),
        script.join("\n")
    );

    // Build environment variables for the PowerShell context
    let env_vars = build_context_variables(session, package, cmd, working_dir);

    let mut command = std::process::Command::new("powershell.exe");
    command
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(&ps_script)
        .current_dir(working_dir);

    // Set environment variables
    for (key, value) in &env_vars {
        command.env(key, value);
    }

    if let Some(tx) = session.emitter() {
        let _ = tx.send(crate::Event::PackageCommitStart(format!(
            "running powershell script for {}",
            package.name()
        )));
    }

    let output = command.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(crate::Error::Custom(format!(
            "PowerShell script execution failed for '{}':\nstdout: {}\nstderr: {}",
            package.name(),
            stdout,
            stderr
        )));
    }

    if let Some(tx) = session.emitter() {
        let _ = tx.send(crate::Event::PackageCommitDone(package.name().to_owned()));
    }

    Ok(())
}

/// Execute a PowerShell script and capture its standard output.
///
/// Same Scoop execution context as [`invoke_script`], but returns the
/// trimmed stdout instead of discarding it. Used by `checkver.script`
/// manifests, where the script is expected to print the latest version
/// (optionally post-processed with `checkver.regex`).
pub fn invoke_script_capture(
    session: &Session,
    package: &Package,
    cmd: &str,
    script: &[&str],
    working_dir: &Path,
) -> Fallible<String> {
    // Same prelude as `invoke_script`; definitions and assignments are
    // silent, so captured stdout still carries only the script's output.
    let ps_script = format!(
        "{}{}",
        build_prelude_for(session, package, cmd, working_dir),
        script.join("\n")
    );
    let env_vars = build_context_variables(session, package, cmd, working_dir);

    let mut command = std::process::Command::new("powershell.exe");
    command
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(&ps_script)
        .current_dir(working_dir);

    for (key, value) in &env_vars {
        command.env(key, value);
    }

    let output = command.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(crate::Error::Custom(format!(
            "PowerShell script execution failed for '{}':\nstdout: {}\nstderr: {}",
            package.name(),
            stdout,
            stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::manifest::Manifest;

    /// Build a minimal package for script tests.
    fn test_package() -> Package {
        let manifest = Manifest::parse_bytes(
            br#"{
                "version": "1.0",
                "homepage": "https://example.com",
                "license": "MIT"
            }"#,
            Path::new("test-pkg.json"),
        )
        .expect("fixture manifest should parse");
        Package::from("test-pkg", "main", manifest)
    }
    #[test]
    #[cfg(windows)]
    fn capture_returns_trimmed_stdout() {
        let session = Session::new();
        let pkg = test_package();
        let out = invoke_script_capture(
            &session,
            &pkg,
            "checkver",
            &["\"7.8.9\""],
            &std::env::temp_dir(),
        )
        .expect("powershell should run");
        assert_eq!(out, "7.8.9");
    }

    #[test]
    #[cfg(windows)]
    fn capture_reports_failure() {
        let session = Session::new();
        let pkg = test_package();
        let err = invoke_script_capture(
            &session,
            &pkg,
            "checkver",
            &["exit 3"],
            &std::env::temp_dir(),
        )
        .expect_err("failing script should error");
        assert!(err.to_string().contains("failed"));
    }

    #[test]
    fn ps_quote_escapes_single_quotes() {
        assert_eq!(ps_quote("plain"), "'plain'");
        assert_eq!(ps_quote("a'b"), "'a''b'");
        assert_eq!(ps_quote("C:\\a'b\\c"), "'C:\\a''b\\c'");
    }

    #[test]
    fn helper_exe_prefers_current_then_global() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-helper-exe");
        let _ = std::fs::remove_dir_all(&base);
        let user_root = base.join("user");
        let global_root = base.join("global");
        let seven = user_root.join("apps").join("7zip").join("current");
        let less = global_root.join("apps").join("lessmsi").join("current");
        std::fs::create_dir_all(&seven).unwrap();
        std::fs::create_dir_all(&less).unwrap();
        std::fs::write(seven.join("7z.exe"), b"").unwrap();
        std::fs::write(less.join("lessmsi.exe"), b"").unwrap();
        std::env::set_var("SCOOP", &user_root);
        std::env::set_var("SCOOP_GLOBAL", &global_root);
        std::env::set_var("SCOOP_CACHE", base.join("cache"));

        let session = Session::new();
        assert_eq!(
            helper_exe(&session, "7zip", "7z.exe"),
            Some(seven.join("7z.exe"))
        );
        assert_eq!(
            helper_exe(&session, "lessmsi", "lessmsi.exe"),
            Some(less.join("lessmsi.exe"))
        );
        assert_eq!(helper_exe(&session, "nope", "nope.exe"), None);

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    #[cfg(windows)]
    fn prelude_exposes_hook_variables() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-prelude-vars");
        let _ = std::fs::remove_dir_all(&base);
        std::env::set_var("SCOOP", base.join("root"));
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));

        let session = Session::new();
        let pkg = test_package();
        let dir = base.join("apps").join("test-pkg").join("1.0");
        std::fs::create_dir_all(&dir).unwrap();
        let out = invoke_script_capture(
            &session,
            &pkg,
            "install",
            &["\"$cmd|$dir|$version|$architecture|$app|$bucket|$bucketsdir|$global|$original_dir|$persist_dir\""],
            &dir,
        )
        .expect("powershell should run");
        let root = base.join("root").to_string_lossy().into_owned();
        let backslashed = root.replace('/', "\\");
        let expected = format!(
            "install|{d}|1.0|64bit|test-pkg|main|{r}\\buckets|False|{r}\\apps\\test-pkg\\1.0|{r}\\persist\\test-pkg",
            d = dir.to_string_lossy(),
            r = backslashed,
        );
        assert_eq!(out, expected);

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    #[cfg(windows)]
    fn prelude_defines_scoop_helpers() {
        let session = Session::new();
        let pkg = test_package();
        let out = invoke_script_capture(
            &session,
            &pkg,
            "install",
            &["Get-Command Expand-7zipArchive, Expand-MsiArchive, Expand-InnoArchive, Expand-DarkArchive, Expand-ZipArchive, Expand-ZstdArchive, Get-HelperPath, Invoke-ExternalCommand, uninstall_rm, abort, warn, error, info, success, movedir, ensure, fname, Add-Path, Remove-Path, Get-EnvVar, Set-EnvVar, Test-HelperInstalled -ErrorAction Stop | ForEach-Object { $_.Name }"],
            &std::env::temp_dir(),
        )
        .expect("helpers should be defined");
        for name in [
            "Expand-7zipArchive",
            "Expand-MsiArchive",
            "Expand-InnoArchive",
            "Expand-DarkArchive",
            "Expand-ZipArchive",
            "Expand-ZstdArchive",
            "Get-HelperPath",
            "Invoke-ExternalCommand",
            "uninstall_rm",
            "abort",
            "warn",
            "error",
            "info",
            "success",
            "movedir",
            "ensure",
            "fname",
            "Add-Path",
            "Remove-Path",
            "Get-EnvVar",
            "Set-EnvVar",
            "Test-HelperInstalled",
        ] {
            assert!(out.contains(name), "missing helper {name}");
        }
    }

    #[test]
    #[cfg(windows)]
    fn expand_7ziparchive_roundtrip() {
        let session = Session::new();
        let pkg = test_package();
        // Override the baked helper table with PATH 7z when the Scoop 7zip
        // app is absent (e.g. CI runners), so the roundtrip still executes.
        let script = r#"
$z = Get-HelperPath 7zip
if (!$z) { $z = (Get-Command 7z.exe -CommandType Application -ErrorAction Ignore).Source }
if (!$z) { 'NO7Z' } else {
  $BaggerHelpers['7zip'] = $z
  $t = Join-Path $env:TEMP 'bagger-test-7zrt'
  if (Test-Path $t) { Remove-Item $t -Recurse -Force }
  New-Item -ItemType Directory $t | Out-Null
  'hello-ore' | Out-File "$t\a.txt" -Encoding ascii -NoNewline
  Compress-Archive -Path "$t\a.txt" -DestinationPath "$t\a.zip"
  Expand-7zipArchive "$t\a.zip" "$t\out"
  if ((Get-Content "$t\out\a.txt" -Raw) -eq 'hello-ore') { 'OK' } else { 'MISMATCH' }
  Remove-Item $t -Recurse -Force
}"#;
        let out =
            invoke_script_capture(&session, &pkg, "install", &[script], &std::env::temp_dir())
                .expect("powershell should run");
        assert!(out == "OK" || out == "NO7Z", "unexpected: {out}");
    }

    #[test]
    #[cfg(windows)]
    fn uninstall_rm_removes_files_and_trees() {
        let session = Session::new();
        let pkg = test_package();
        let script = r#"
$t = Join-Path $env:TEMP 'bagger-test-uninstallrm'
if (Test-Path $t) { Remove-Item $t -Recurse -Force }
New-Item -ItemType Directory "$t\d" | Out-Null
'f' | Out-File "$t\d\f.txt" -Encoding ascii -NoNewline
'f' | Out-File "$t\top.txt" -Encoding ascii -NoNewline
uninstall_rm "$t\d"
uninstall_rm "$t\top.txt"
uninstall_rm "$t\does-not-exist"
if ((Test-Path "$t\d") -or (Test-Path "$t\top.txt")) { 'LEFT' } else { 'OK' }
Remove-Item $t -Recurse -Force -ErrorAction Ignore"#;
        let out = invoke_script_capture(
            &session,
            &pkg,
            "uninstall",
            &[script],
            &std::env::temp_dir(),
        )
        .expect("powershell should run");
        assert_eq!(out, "OK");
    }

    #[test]
    #[cfg(windows)]
    fn missing_helpers_resolve_to_null() {
        let session = Session::new();
        let pkg = test_package();
        let out = invoke_script_capture(
            &session,
            &pkg,
            "install",
            &["$x = Get-HelperPath Lessmsi; if ($null -eq $x) { 'NULL' } elseif (Test-Path $x) { 'FOUND' } else { 'STALE' }"],
            &std::env::temp_dir(),
        )
        .expect("powershell should run");
        assert!(out == "NULL" || out == "FOUND", "unexpected: {out}");
    }
}
