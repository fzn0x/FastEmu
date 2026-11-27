# Tries install.ps1 on Windows, as release.yml does:
#
#   packaging/check-installers.ps1 -Version 0.2.0 -From <folder>
#
# The folder holds the release's Windows files and their SHA256SUMS.
# install.ps1 installs FastEmu from it, the installed program prints its
# version, and -Uninstall removes it. It stops when FastEmu is installed
# already, so as not to touch a real installation.

param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$From
)

$ErrorActionPreference = 'Stop'

$dir = Join-Path $env:LOCALAPPDATA 'Programs\FastEmu'
$key = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\io.github.fzn0x.fastemu'
if (Test-Path $dir) {
    throw "check-installers: $dir exists: FastEmu is installed already"
}

& (Join-Path $PSScriptRoot 'install.ps1') -Version $Version -From $From

$out = Join-Path ([System.IO.Path]::GetTempPath()) "fastemu-version-$([System.Guid]::NewGuid()).txt"
$process = Start-Process -FilePath (Join-Path $dir 'fastemu.com') -ArgumentList '--version' -RedirectStandardOutput $out -NoNewWindow -Wait -PassThru
$said = "$(Get-Content -Raw $out)".Trim()
Remove-Item -Force $out
if ($process.ExitCode -ne 0 -or $said -ne "fastemu $Version") {
    throw "check-installers: the installed fastemu says '$said', exit $($process.ExitCode)"
}

& (Join-Path $PSScriptRoot 'install.ps1') -Uninstall
# The uninstaller runs from a copy of itself, which may still be at work.
$deadline = [DateTime]::UtcNow.AddSeconds(60)
while ((Test-Path $dir) -and [DateTime]::UtcNow -lt $deadline) {
    Start-Sleep -Milliseconds 500
}
if ((Test-Path $dir) -or (Test-Path $key)) {
    throw 'check-installers: -Uninstall left FastEmu behind'
}
Write-Output "check-installers: install.ps1 installed and uninstalled FastEmu $Version"
