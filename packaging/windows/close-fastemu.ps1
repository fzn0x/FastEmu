# Closes the FastEmu programs that run from a folder, for the setup and the
# uninstaller (setup.nsi), which cannot replace or remove their files
# while they run:
#
#   close-fastemu.ps1 'C:\Users\me\AppData\Local\Programs\FastEmu'
#
# It asks each one's window to close, as its close button does, waits up
# to 10 seconds, then ends whatever is still running. The setup is a
# 32-bit program, so this runs in 32-bit PowerShell, which cannot read the
# paths of 64-bit processes; CIM gives every process's path.

param(
    [Parameter(Mandatory = $true)][string]$Folder
)

$prefix = $Folder.TrimEnd('\') + '\'
$running = @(
    Get-CimInstance Win32_Process -Filter "Name = 'fastemu.exe' OR Name = 'fastemu.com'" |
        Where-Object { $_.ExecutablePath -and $_.ExecutablePath.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase) } |
        ForEach-Object { Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue }
)
foreach ($process in $running) {
    [void]$process.CloseMainWindow()
}
$deadline = [DateTime]::UtcNow.AddSeconds(10)
foreach ($process in $running) {
    $left = [Math]::Max(0, [int]($deadline - [DateTime]::UtcNow).TotalMilliseconds)
    if (-not $process.WaitForExit($left)) {
        try {
            $process.Kill()
        } catch {
            # It ended meanwhile, or it is not ours to end: the setup
            # finds it still running and says so.
        }
    }
}
