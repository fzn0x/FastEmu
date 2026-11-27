# Tries the Windows setup that package.ps1 builds, as CI does:
#
#   packaging/windows/check-setup.ps1 -Setup out/fastemu-0.2.0-windows-x64-setup.exe -Version 0.2.0
#
# It installs FastEmu silently for the current user and checks the files,
# the Start menu shortcut, Open with for .jar and .jad, the entry in
# Settings > Apps, and that both programs start. It installs again over a
# running FastEmu, as an update does, then uninstalls silently and checks
# that everything the setup added is gone and that .jar files still open
# with what opened them before. It stops when FastEmu is installed
# already, so as not to touch a real installation; on your own machine it
# opens FastEmu's window for a moment.

param(
    [Parameter(Mandatory = $true)][string]$Setup,
    [Parameter(Mandatory = $true)][string]$Version
)

$ErrorActionPreference = 'Stop'

$id = 'io.github.fzn0x.fastemu'
$dir = Join-Path $env:LOCALAPPDATA 'Programs\FastEmu'
$exe = Join-Path $dir 'fastemu.exe'
$classes = 'HKCU:\Software\Classes'
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$id"
$startMenu = Join-Path ([Environment]::GetFolderPath('Programs')) 'FastEmu.lnk'
$desktop = Join-Path ([Environment]::GetFolderPath('Desktop')) 'FastEmu.lnk'
$files = 'fastemu.exe', 'fastemu.com', 'README.md', 'LICENSE', 'THIRD_PARTY_NOTICES.md', 'Uninstall.exe'
$command = "`"$exe`" `"%1`""

function Fail([string]$Message) {
    throw "check-setup: $Message"
}

# A registry key, or $null.
function Get-Key([string]$Path) {
    Get-Item -LiteralPath $Path -ErrorAction SilentlyContinue
}

# The (default) value of a key, or $null.
function Get-Default([string]$Path) {
    $key = Get-Key $Path
    if ($key) { $key.GetValue('') } else { $null }
}

function Test-Value([string]$Path, [string]$Name) {
    $key = Get-Key $Path
    [bool]($key -and ($key.GetValueNames() -contains $Name))
}

# Runs a setup or an uninstaller and returns its exit code. -Wait also
# waits for the copy of itself that the uninstaller starts.
function Invoke-Silently([string]$Program) {
    (Start-Process -FilePath $Program -ArgumentList '/S' -Wait -PassThru).ExitCode
}

# What `PROGRAM --version` prints, through a file, as fastemu.exe is a GUI
# program that only writes where it is pointed.
function Get-Version([string]$Program) {
    $out = Join-Path ([System.IO.Path]::GetTempPath()) "fastemu-version-$([System.Guid]::NewGuid()).txt"
    try {
        $process = Start-Process -FilePath $Program -ArgumentList '--version' -RedirectStandardOutput $out -NoNewWindow -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            Fail "$Program --version exited with $($process.ExitCode)"
        }
        "$(Get-Content -Raw $out)".Trim()
    } finally {
        Remove-Item -Force $out -ErrorAction SilentlyContinue
    }
}

function Assert-Installed {
    foreach ($file in $files) {
        if (-not (Test-Path (Join-Path $dir $file))) {
            Fail "$file is not in $dir"
        }
    }
    foreach ($program in 'fastemu.exe', 'fastemu.com') {
        $said = Get-Version (Join-Path $dir $program)
        if ($said -ne "fastemu $Version") {
            Fail "$program --version says '$said', not 'fastemu $Version'"
        }
    }

    $key = Get-Key $uninstallKey
    if (-not $key) {
        Fail "Settings > Apps has no FastEmu: $uninstallKey is missing"
    }
    $want = @{
        DisplayName = 'FastEmu'
        DisplayVersion = $Version
        InstallLocation = $dir
        UninstallString = "`"$dir\Uninstall.exe`""
        QuietUninstallString = "`"$dir\Uninstall.exe`" /S"
    }
    foreach ($name in $want.Keys) {
        if ($key.GetValue($name) -ne $want[$name]) {
            Fail "$name in $uninstallKey is '$($key.GetValue($name))', not '$($want[$name])'"
        }
    }
    if ($key.GetValue('EstimatedSize') -lt 1024) {
        Fail "EstimatedSize in $uninstallKey is $($key.GetValue('EstimatedSize')) KB"
    }

    if (-not (Test-Path $startMenu)) {
        Fail "the Start menu has no FastEmu: $startMenu is missing"
    }
    $target = (New-Object -ComObject WScript.Shell).CreateShortcut($startMenu).TargetPath
    if ($target -ne $exe) {
        Fail "the Start menu shortcut opens $target, not $exe"
    }
    if (Test-Path $desktop) {
        Fail "a silent install made a desktop shortcut: $desktop"
    }

    foreach ($ext in 'jar', 'jad') {
        if (-not (Test-Value "$classes\.$ext\OpenWithProgids" "$id.$ext")) {
            Fail ".$ext files do not list FastEmu under Open with"
        }
        if ((Get-Default "$classes\$id.$ext\shell\open\command") -ne $command) {
            Fail "FastEmu opens .$ext files with '$(Get-Default "$classes\$id.$ext\shell\open\command")', not '$command'"
        }
        if (-not (Test-Value "$classes\Applications\fastemu.exe\SupportedTypes" ".$ext")) {
            Fail "Applications\fastemu.exe does not list .$ext"
        }
    }
    if ((Get-Default "$classes\.jar") -ne $jarBefore) {
        Fail "the setup changed what opens .jar files: '$(Get-Default "$classes\.jar")', was '$jarBefore'"
    }
}

if (Test-Path $dir) {
    Fail "$dir exists: FastEmu is installed already, and this check would remove it"
}
$setupPath = (Resolve-Path $Setup).Path
# Java's association, which the setup must leave alone.
$jarBefore = Get-Default "$classes\.jar"

$code = Invoke-Silently $setupPath
if ($code -ne 0) {
    Fail "the setup exited with $code"
}
Assert-Installed
Write-Output "check-setup: installed FastEmu $Version into $dir"

# An update while FastEmu is open: the setup closes it, then replaces it.
$open = Start-Process -FilePath $exe -ArgumentList '--mock' -PassThru
# Holding the handle keeps the exit code readable once it ends.
$null = $open.Handle
Start-Sleep -Seconds 5
if ($open.HasExited) {
    Write-Output "::warning::check-setup: FastEmu --mock ended by itself with $($open.ExitCode), so the update ran over a closed FastEmu"
}
$code = Invoke-Silently $setupPath
if ($code -ne 0) {
    Fail "the update exited with $code"
}
if (-not $open.WaitForExit(5000)) {
    Fail "the update left FastEmu open"
}
Assert-Installed
Write-Output "check-setup: installed FastEmu again over an open one"

$code = Invoke-Silently (Join-Path $dir 'Uninstall.exe')
if ($code -ne 0) {
    Fail "the uninstaller exited with $code"
}
# The uninstaller runs from a copy in the temporary folder, which may
# still be removing files.
$deadline = [DateTime]::UtcNow.AddSeconds(60)
while ((Test-Path $dir) -and [DateTime]::UtcNow -lt $deadline) {
    Start-Sleep -Milliseconds 500
}
if (Test-Path $dir) {
    Fail "$dir is still there after uninstalling: $((Get-ChildItem $dir).Name -join ', ')"
}
foreach ($path in $uninstallKey, "$classes\$id.jar", "$classes\$id.jad", "$classes\Applications\fastemu.exe") {
    if (Get-Key $path) {
        Fail "the uninstaller left $path"
    }
}
foreach ($ext in 'jar', 'jad') {
    if (Test-Value "$classes\.$ext\OpenWithProgids" "$id.$ext") {
        Fail "the uninstaller left FastEmu under Open with for .$ext"
    }
}
if (Test-Path $startMenu) {
    Fail "the uninstaller left $startMenu"
}
if ((Get-Default "$classes\.jar") -ne $jarBefore) {
    Fail "uninstalling changed what opens .jar files"
}
Write-Output "check-setup: uninstalled FastEmu, and nothing of it is left"
