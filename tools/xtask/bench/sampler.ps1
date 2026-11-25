# The process sampler of `cargo xtask bench`, on Windows. The bench writes
# this file to its output folder and runs it once, with
# powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -File.
#
# It first prints "rate R": the cycles a second a busy thread runs, the
# largest of five 200 ms tries, since being preempted only lowers it.
# Then, for each line it reads, a list of process ids (or the word
# "audiodg", the Windows audio engine), it prints one line:
#
#   UNIXMS sys IDLE KERNEL USER | PID CYCLES PEAK_WS WS PRIVATE | PID gone | ...
#
# IDLE, KERNEL and USER are GetSystemTimes, in 100 ns units over every
# processor. CYCLES is QueryProcessCycleTime, which counts every burst a
# thread runs, where Windows' tick based CPU times miss short ones. PEAK_WS,
# WS and PRIVATE are the peak working set, the working set and the private
# bytes; "-" when Windows refuses to tell. A process that has ended, or
# that cannot be opened, is "gone". It waits on its input between lines,
# so it costs nothing between samples.
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

public static class FastEmuBench {
    [StructLayout(LayoutKind.Sequential)]
    public struct Counters {
        public uint cb;
        public uint PageFaultCount;
        public UIntPtr PeakWorkingSetSize;
        public UIntPtr WorkingSetSize;
        public UIntPtr QuotaPeakPagedPoolUsage;
        public UIntPtr QuotaPagedPoolUsage;
        public UIntPtr QuotaPeakNonPagedPoolUsage;
        public UIntPtr QuotaNonPagedPoolUsage;
        public UIntPtr PagefileUsage;
        public UIntPtr PeakPagefileUsage;
        public UIntPtr PrivateUsage;
    }

    [DllImport("kernel32.dll", SetLastError = true)]
    public static extern IntPtr OpenProcess(uint access, bool inherit, uint pid);
    [DllImport("kernel32.dll")]
    public static extern bool GetExitCodeProcess(IntPtr process, out uint code);
    [DllImport("kernel32.dll")]
    public static extern bool QueryProcessCycleTime(IntPtr process, out ulong cycles);
    [DllImport("kernel32.dll")]
    public static extern bool QueryThreadCycleTime(IntPtr thread, out ulong cycles);
    [DllImport("kernel32.dll")]
    public static extern IntPtr GetCurrentThread();
    [DllImport("kernel32.dll")]
    public static extern bool GetSystemTimes(out long idle, out long kernel, out long user);
    [DllImport("kernel32.dll", EntryPoint = "K32GetProcessMemoryInfo")]
    public static extern bool GetProcessMemoryInfo(IntPtr process, out Counters counters, uint size);
}
'@

$thread = [FastEmuBench]::GetCurrentThread()
$best = 0.0
for ($try = 0; $try -lt 5; $try++) {
    $a = [uint64]0
    [void][FastEmuBench]::QueryThreadCycleTime($thread, [ref]$a)
    $watch = [Diagnostics.Stopwatch]::StartNew()
    while ($watch.ElapsedMilliseconds -lt 200) {}
    $b = [uint64]0
    [void][FastEmuBench]::QueryThreadCycleTime($thread, [ref]$b)
    $rate = ($b - $a) / $watch.Elapsed.TotalSeconds
    if ($rate -gt $best) { $best = $rate }
}
[Console]::Out.WriteLine(('rate {0}' -f [uint64]$best))
[Console]::Out.Flush()

# Handles stay open while the sampler runs, so a process id is never
# taken for a new process that reused it.
$handles = @{}
$size = [Runtime.InteropServices.Marshal]::SizeOf([type][FastEmuBench+Counters])
while ($null -ne ($line = [Console]::In.ReadLine())) {
    $idle = [long]0; $kernel = [long]0; $user = [long]0
    [void][FastEmuBench]::GetSystemTimes([ref]$idle, [ref]$kernel, [ref]$user)
    $out = @(('{0} sys {1} {2} {3}' -f [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(), $idle, $kernel, $user))
    foreach ($word in ($line -split '\s+' | Where-Object { $_ })) {
        $id = [uint32]0
        if ($word -eq 'audiodg') {
            $found = [Diagnostics.Process]::GetProcessesByName('audiodg') | Select-Object -First 1
            if ($null -ne $found) { $id = [uint32]$found.Id }
        } elseif (-not [uint32]::TryParse($word, [ref]$id)) {
            $id = [uint32]0
        }
        if ($id -eq 0) { $out += ('{0} gone' -f $word); continue }
        if (-not $handles.ContainsKey($id)) {
            # PROCESS_QUERY_LIMITED_INFORMATION and PROCESS_VM_READ, or
            # only the first where the second is refused.
            $h = [FastEmuBench]::OpenProcess(0x1010, $false, $id)
            if ($h -eq [IntPtr]::Zero) { $h = [FastEmuBench]::OpenProcess(0x1000, $false, $id) }
            if ($h -eq [IntPtr]::Zero) { $out += ('{0} gone' -f $id); continue }
            $handles[$id] = $h
        }
        $h = $handles[$id]
        $code = [uint32]0
        [void][FastEmuBench]::GetExitCodeProcess($h, [ref]$code)
        if ($code -ne 259) { $out += ('{0} gone' -f $id); continue }
        $cycles = [uint64]0
        [void][FastEmuBench]::QueryProcessCycleTime($h, [ref]$cycles)
        $c = New-Object FastEmuBench+Counters
        $c.cb = [uint32]$size
        if ([FastEmuBench]::GetProcessMemoryInfo($h, [ref]$c, [uint32]$size)) {
            $out += ('{0} {1} {2} {3} {4}' -f $id, $cycles, $c.PeakWorkingSetSize, $c.WorkingSetSize, $c.PrivateUsage)
        } else {
            $out += ('{0} {1} - - -' -f $id, $cycles)
        }
    }
    [Console]::Out.WriteLine(($out -join ' | '))
    [Console]::Out.Flush()
}
