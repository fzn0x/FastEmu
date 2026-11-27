# Packs a Windows build of FastEmu for a release:
#
#   packaging/windows/package.ps1 -Version 0.2.0 -Exe target/release/fastemu.exe -OutDir out
#
# It writes OutDir/fastemu-VERSION-windows-x64.zip: one folder of the same
# name holding fastemu.exe, fastemu.com, the README and the licenses.
# fastemu.com is fastemu-console.exe from beside fastemu.exe, renamed: the
# console program a terminal runs for `fastemu` (crates/fastemu/src/console.rs).
# First it checks, without running either program, that fastemu.exe
# carries the icon and the version that crates/fastemu/build.rs embeds,
# and that fastemu.exe is a GUI program, which opens no console window,
# and fastemu.com a console one. Run it from the repository's root, in
# Windows PowerShell 5.1 or PowerShell 7.

param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$Exe,
    [Parameter(Mandatory = $true)][string]$OutDir
)

$ErrorActionPreference = 'Stop'

$exePath = (Resolve-Path $Exe).Path
$consolePath = Join-Path (Split-Path $exePath) 'fastemu-console.exe'
if (-not (Test-Path $consolePath)) {
    throw "$consolePath is missing: cargo build -p fastemu builds it beside fastemu.exe"
}

# The subsystem in a program's PE header: 2 for a GUI program, 3 for a
# console program. It sits 92 bytes after the PE signature.
function Get-Subsystem([string]$Path) {
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $pe = [System.BitConverter]::ToInt32($bytes, 0x3C)
    return [System.BitConverter]::ToUInt16($bytes, $pe + 92)
}
if ((Get-Subsystem $exePath) -ne 2) {
    throw "fastemu.exe is not a GUI program: it would open a console window"
}
if ((Get-Subsystem $consolePath) -ne 3) {
    throw "fastemu-console.exe is not a console program: a terminal would not wait for it"
}

# The version Explorer's Properties show.
$info = (Get-Item $exePath).VersionInfo
if ($info.ProductVersion -ne $Version -or $info.FileVersion -ne $Version) {
    throw "fastemu.exe says version $($info.ProductVersion), not ${Version}: was it built from this commit?"
}
if ($info.ProductName -ne 'FastEmu') {
    throw "fastemu.exe has no version information"
}

# The icon: a group icon resource numbered 1, which Explorer and GPUI's
# windows show. The program is loaded as data, so none of it runs.
Add-Type -Namespace FastEmu -Name Resources -MemberDefinition @'
[DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
public static extern IntPtr LoadLibraryEx(string path, IntPtr file, uint flags);
[DllImport("kernel32.dll", SetLastError = true)]
public static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
[DllImport("kernel32.dll", SetLastError = true)]
public static extern uint SizeofResource(IntPtr module, IntPtr resource);
[DllImport("kernel32.dll")]
public static extern bool FreeLibrary(IntPtr module);
'@
$LOAD_LIBRARY_AS_DATAFILE = 0x2
$RT_GROUP_ICON = [IntPtr]14
$module = [FastEmu.Resources]::LoadLibraryEx($exePath, [IntPtr]::Zero, $LOAD_LIBRARY_AS_DATAFILE)
if ($module -eq [IntPtr]::Zero) {
    throw "cannot read the resources of $exePath"
}
try {
    $group = [FastEmu.Resources]::FindResource($module, [IntPtr]1, $RT_GROUP_ICON)
    if ($group -eq [IntPtr]::Zero) {
        throw "fastemu.exe has no icon: crates/fastemu/build.rs did not embed packaging/icons/fastemu.ico"
    }
    # A group of n icons is a 6-byte header and 14 bytes per icon.
    $icons = ([FastEmu.Resources]::SizeofResource($module, $group) - 6) / 14
    Write-Output "fastemu.exe: version $($info.ProductVersion), an icon of $icons sizes"
} finally {
    [void][FastEmu.Resources]::FreeLibrary($module)
}

$name = "fastemu-$Version-windows-x64"
New-Item -ItemType Directory -Force $OutDir | Out-Null
$out = (Resolve-Path $OutDir).Path
$stage = Join-Path $out "stage-$name"
if (Test-Path $stage) {
    Remove-Item -Recurse -Force $stage
}
$folder = Join-Path $stage $name
New-Item -ItemType Directory -Force $folder | Out-Null
Copy-Item $exePath (Join-Path $folder 'fastemu.exe')
Copy-Item $consolePath (Join-Path $folder 'fastemu.com')
foreach ($file in 'README.md', 'LICENSE', 'THIRD_PARTY_NOTICES.md') {
    Copy-Item $file (Join-Path $folder $file)
}

# Windows' own tar writes the zip with forward slashes, which every unzip
# reads; Compress-Archive in PowerShell 5.1 does not.
$zip = Join-Path $out "$name.zip"
if (Test-Path $zip) {
    Remove-Item -Force $zip
}
& "$env:SystemRoot\System32\tar.exe" -a -c -f $zip -C $stage $name
if ($LASTEXITCODE -ne 0) {
    throw "tar could not write $zip"
}
Remove-Item -Recurse -Force $stage
Write-Output $zip
