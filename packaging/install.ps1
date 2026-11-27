# Installs FastEmu from a release, on Windows, in PowerShell:
#
#   irm https://github.com/fzn0x/FastEmu/releases/latest/download/install.ps1 | iex
#   & ([scriptblock]::Create((irm https://github.com/fzn0x/FastEmu/releases/latest/download/install.ps1))) -Version 0.2.0
#   powershell -File install.ps1 [-Version X.Y.Z] [-From <folder or address>] [-Uninstall]
#
# It downloads the release's setup, checks it against the release's
# SHA256SUMS, and runs it silently: FastEmu installs for the current user,
# with no administrator rights, into %LOCALAPPDATA%\Programs\FastEmu, with
# a Start menu shortcut and Open with for .jar and .jad files. -Version
# picks a release, by default the one this script came with. -From takes
# the files from a folder or another address instead, as CI does with the
# files a release has just built. -Uninstall runs FastEmu's uninstaller
# silently; games, saves and settings stay. It runs in Windows PowerShell
# 5.1 and PowerShell 7.

param(
    [string]$Version = '',
    [string]$From = '',
    [switch]$Uninstall
)

# Everything runs in a function, so that `irm | iex` leaves no variables
# or preferences behind in the shell it runs in.
function Install-FastEmu([string]$Version, [string]$From, [switch]$Uninstall) {
    $ErrorActionPreference = 'Stop'
    # Windows PowerShell 5.1 shows a progress bar that slows downloads down.
    $ProgressPreference = 'SilentlyContinue'

    $repo = 'fzn0x/FastEmu'
    $id = 'io.github.fzn0x.fastemu'
    # The publish job of release.yml writes the release's version here.
    $releaseVersion = '@VERSION@'
    $uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$id"

    function Say([string]$Message) {
        Write-Host "install.ps1: $Message"
    }

    function Fail([string]$Message) {
        throw "install.ps1: $Message"
    }

    if ($Uninstall) {
        $key = Get-ItemProperty -LiteralPath $uninstallKey -ErrorAction SilentlyContinue
        if (-not $key) {
            Say 'FastEmu is not installed for this user'
            return
        }
        $uninstaller = Join-Path $key.InstallLocation 'Uninstall.exe'
        if (-not (Test-Path -LiteralPath $uninstaller)) {
            Fail "$uninstaller is missing: uninstall FastEmu from Settings > Apps"
        }
        $process = Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            Fail "the uninstaller exited with $($process.ExitCode)"
        }
        Say "uninstalled FastEmu; your games and saves stay in $env:APPDATA\FastEmu"
        return
    }

    $Version = $Version.TrimStart('v')
    if (-not $Version -and -not $releaseVersion.StartsWith('@')) {
        $Version = $releaseVersion
    }
    if ($Version -and $Version -notmatch '^\d+\.\d+\.\d+$') {
        Fail "-Version takes a version such as 0.2.0, not $Version"
    }
    if ($From) {
        $base = $From.TrimEnd('/', '\')
    } elseif ($Version) {
        $base = "https://github.com/$repo/releases/download/v$Version"
    } else {
        $base = "https://github.com/$repo/releases/latest/download"
    }
    $remote = $base -match '^https?://'

    $work = Join-Path ([System.IO.Path]::GetTempPath()) "fastemu-install-$([System.Guid]::NewGuid())"
    New-Item -ItemType Directory -Path $work | Out-Null
    try {
        # Copies the release's file NAME into the work folder.
        function Get-ReleaseFile([string]$Name) {
            $dest = Join-Path $work $Name
            if ($remote) {
                try {
                    Invoke-WebRequest -UseBasicParsing -Uri "$base/$Name" -OutFile $dest
                } catch {
                    Fail "cannot download $base/$Name`: $($_.Exception.Message)"
                }
            } else {
                $source = Join-Path $base $Name
                if (-not (Test-Path -LiteralPath $source)) {
                    Fail "$base has no $Name"
                }
                Copy-Item -LiteralPath $source $dest
            }
            $dest
        }

        $sums = Get-Content -LiteralPath (Get-ReleaseFile 'SHA256SUMS')
        if ($Version) {
            $pattern = '^([0-9a-f]{64})  (fastemu-' + [regex]::Escape($Version) + '-windows-x64-setup\.exe)$'
        } else {
            $pattern = '^([0-9a-f]{64})  (fastemu-\d+\.\d+\.\d+-windows-x64-setup\.exe)$'
        }
        $line = $sums | Where-Object { $_ -match $pattern } | Select-Object -First 1
        if (-not $line) {
            Fail 'the release has no Windows setup'
        }
        $found = [regex]::Match($line, $pattern)
        $want = $found.Groups[1].Value
        $name = $found.Groups[2].Value
        $v = ($name -replace '^fastemu-', '') -replace '-windows-x64-setup\.exe$', ''

        $setup = Get-ReleaseFile $name
        $got = (Get-FileHash -Algorithm SHA256 -LiteralPath $setup).Hash
        if ($got -ne $want) {
            Fail "$name does not match the release's SHA256SUMS: it was damaged or changed on the way"
        }
        $info = (Get-Item -LiteralPath $setup).VersionInfo
        if ($info.ProductName -ne 'FastEmu' -or $info.ProductVersion -ne $v) {
            Fail "$name is not the setup of FastEmu $v"
        }

        Say "installing FastEmu $v"
        $process = Start-Process -FilePath $setup -ArgumentList '/S' -Wait -PassThru
        if ($process.ExitCode -ne 0) {
            Fail "the setup exited with $($process.ExitCode)"
        }
        $key = Get-ItemProperty -LiteralPath $uninstallKey -ErrorAction SilentlyContinue
        if (-not $key -or $key.DisplayVersion -ne $v) {
            Fail 'the setup ended, but Settings > Apps does not list FastEmu'
        }
        Say "installed FastEmu $v in $($key.InstallLocation): open it from the Start menu"
    } finally {
        Remove-Item -Recurse -Force -LiteralPath $work -ErrorAction SilentlyContinue
    }
}

Install-FastEmu -Version $Version -From $From -Uninstall:$Uninstall
