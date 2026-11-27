# Prints the path of makensis.exe from NSIS 3.13, which builds the Windows
# setup (setup.nsi). The first time, it downloads NSIS's own zip, checks it
# against the SHA-256 below and unpacks it into Dir:
#
#   $makensis = packaging/windows/nsis.ps1 [-Dir <folder>]
#
# NSIS publishes the zip on SourceForge. The second address is a copy of
# the same file that MyGo's releases keep, for when SourceForge is down;
# the checksum is the same, so either gives the same bytes.

param(
    [string]$Dir = (Join-Path ([System.IO.Path]::GetTempPath()) 'fastemu-nsis')
)

$ErrorActionPreference = 'Stop'

$version = '3.13'
$sha256 = 'ba63dffc4410ee89193e1cb5a41989991bd77c61068da17e3156d136b7b0b3d8'
$urls = @(
    "https://downloads.sourceforge.net/project/nsis/NSIS%203/$version/nsis-$version.zip",
    "https://github.com/egoist/mygo/releases/download/nsis-$version/nsis-$version.zip"
)

$nsis = Join-Path $Dir "nsis-$version"
$makensis = Join-Path $nsis 'Bin\makensis.exe'
if (Test-Path $makensis) {
    return $makensis
}

New-Item -ItemType Directory -Force $Dir | Out-Null
$zip = Join-Path $Dir "nsis-$version.zip"
$good = $false
foreach ($url in $urls) {
    for ($try = 1; $try -le 3; $try++) {
        & "$env:SystemRoot\System32\curl.exe" --fail --silent --show-error --location --output $zip $url
        if ($LASTEXITCODE -eq 0 -and (Get-FileHash -Algorithm SHA256 $zip).Hash -eq $sha256) {
            $good = $true
            break
        }
        Write-Warning "nsis.ps1: try $try of $url failed"
        Start-Sleep -Seconds (5 * $try)
    }
    if ($good) {
        break
    }
}
if (-not $good) {
    throw "nsis.ps1: could not download NSIS $version with SHA-256 $sha256"
}

# Unpacked beside its place, then moved there, so a folder at $nsis is
# always complete.
$unpack = Join-Path $Dir "unpack-$([System.Guid]::NewGuid())"
Expand-Archive -LiteralPath $zip -DestinationPath $unpack
Move-Item (Join-Path $unpack "nsis-$version") $nsis
Remove-Item -Recurse -Force $unpack, $zip
if (-not (Test-Path $makensis)) {
    throw "nsis.ps1: the NSIS $version zip has no Bin\makensis.exe"
}
$makensis
