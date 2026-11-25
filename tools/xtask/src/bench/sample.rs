//! Process counters on Windows, read by one PowerShell sampler
//! (`tools/xtask/bench/sampler.ps1`) that the bench starts once and asks
//! at each sample time. It reads CPU time as cycles, since Windows' CPU
//! times count by the clock tick and miss most of the short bursts game
//! threads run in, and turns cycles into milliseconds at the rate a busy
//! thread runs them.

use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;

/// The sampler's script, written next to the report and run from there.
pub const SCRIPT: &str = include_str!("../../bench/sampler.ps1");

/// How long one sample may take before the sampler counts as stuck.
const ANSWER: Duration = Duration::from_secs(20);

/// What one line of the sampler says about one process.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Proc {
    Alive {
        pid: u32,
        cycles: u64,
        /// Peak working set, working set and private bytes, when Windows
        /// tells them.
        memory: Option<Memory>,
    },
    /// The process ended, or could not be opened; the word asked for.
    Gone { asked: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Memory {
    pub peak_ws: u64,
    pub ws: u64,
    pub private: u64,
}

/// One line of the sampler: the time, the whole system's CPU times, and
/// each process asked for, in the order asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Sample {
    pub unix_ms: u64,
    /// GetSystemTimes, in 100 ns units over every processor. `kernel`
    /// includes `idle`.
    pub idle: u64,
    pub kernel: u64,
    pub user: u64,
    pub procs: Vec<Proc>,
}

impl Sample {
    /// Reads one line the sampler wrote.
    pub fn parse(line: &str) -> Option<Sample> {
        let mut parts = line.trim().split(" | ");
        let sys: Vec<&str> = parts.next()?.split(' ').collect();
        if sys.len() != 5 || sys[1] != "sys" {
            return None;
        }
        let mut procs = Vec::new();
        for part in parts {
            let words: Vec<&str> = part.split(' ').collect();
            procs.push(match words.as_slice() {
                [asked, "gone"] => Proc::Gone {
                    asked: (*asked).to_owned(),
                },
                [pid, cycles, "-", "-", "-"] => Proc::Alive {
                    pid: pid.parse().ok()?,
                    cycles: cycles.parse().ok()?,
                    memory: None,
                },
                [pid, cycles, peak, ws, private] => Proc::Alive {
                    pid: pid.parse().ok()?,
                    cycles: cycles.parse().ok()?,
                    memory: Some(Memory {
                        peak_ws: peak.parse().ok()?,
                        ws: ws.parse().ok()?,
                        private: private.parse().ok()?,
                    }),
                },
                _ => return None,
            });
        }
        Some(Sample {
            unix_ms: sys[0].parse().ok()?,
            idle: sys[2].parse().ok()?,
            kernel: sys[3].parse().ok()?,
            user: sys[4].parse().ok()?,
            procs,
        })
    }

    /// The whole system's busy time so far, in milliseconds.
    fn busy_ms(&self) -> f64 {
        (self.kernel.saturating_sub(self.idle) + self.user) as f64 / 10_000.0
    }
}

/// What two samples say about a group of processes between them.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Usage {
    /// Milliseconds between the samples.
    pub span_ms: f64,
    /// CPU time the processes used between the samples, in milliseconds.
    pub cpu_ms: f64,
    /// That per second of the span.
    pub cpu_ms_per_s: f64,
    /// Memory at the second sample, summed over the processes; `None` if
    /// one of them did not tell.
    pub memory: Option<Memory>,
    /// The words asked for of processes that were gone at either sample.
    pub gone: Vec<String>,
}

/// The usage of the processes at `indices` of the samples' lists between
/// samples `a` and `b`, with cycles turned to time at `rate` a second.
/// A process gone at either sample counts as gone, never as zero.
pub fn usage(a: &Sample, b: &Sample, rate: f64, indices: &[usize]) -> Usage {
    let span_ms = b.unix_ms.saturating_sub(a.unix_ms) as f64;
    let mut cycles = 0u64;
    let mut memory = Some(Memory::default());
    let mut gone = Vec::new();
    for &i in indices {
        match (a.procs.get(i), b.procs.get(i)) {
            (
                Some(Proc::Alive {
                    pid: p0,
                    cycles: c0,
                    ..
                }),
                Some(Proc::Alive {
                    pid: p1,
                    cycles: c1,
                    memory: m,
                }),
            ) if p0 == p1 => {
                cycles += c1.saturating_sub(*c0);
                memory = match (memory, m) {
                    (Some(sum), Some(m)) => Some(Memory {
                        peak_ws: sum.peak_ws + m.peak_ws,
                        ws: sum.ws + m.ws,
                        private: sum.private + m.private,
                    }),
                    _ => None,
                };
            }
            (Some(Proc::Gone { asked }), _) | (_, Some(Proc::Gone { asked })) => {
                gone.push(asked.clone());
                memory = None;
            }
            _ => {
                gone.push(format!("#{i}"));
                memory = None;
            }
        }
    }
    let cpu_ms = if rate > 0.0 {
        cycles as f64 / rate * 1000.0
    } else {
        0.0
    };
    Usage {
        span_ms,
        cpu_ms,
        cpu_ms_per_s: if span_ms > 0.0 {
            cpu_ms * 1000.0 / span_ms
        } else {
            0.0
        },
        memory,
        gone,
    }
}

/// The whole system's busy CPU time per second between two samples, in
/// milliseconds a second over every processor.
pub fn system_busy(a: &Sample, b: &Sample) -> f64 {
    let span_ms = b.unix_ms.saturating_sub(a.unix_ms) as f64;
    if span_ms <= 0.0 {
        return 0.0;
    }
    (b.busy_ms() - a.busy_ms()).max(0.0) * 1000.0 / span_ms
}

/// The busy CPU time per second of everything but the processes at
/// `indices`: the other load on the machine while they were measured.
pub fn other_load(a: &Sample, b: &Sample, rate: f64, indices: &[usize]) -> f64 {
    let all: Vec<usize> = (0..b.procs.len()).collect();
    let indices = if indices.is_empty() { &all } else { indices };
    (system_busy(a, b) - usage(a, b, rate, indices).cpu_ms_per_s).max(0.0)
}

/// The running sampler.
pub struct Sampler {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
    /// Cycles a second of a busy thread.
    pub rate: f64,
}

impl Sampler {
    /// Writes the script into `dir` and starts it; its errors go to
    /// `sampler.err` there.
    pub fn start(dir: &Path) -> Result<Self, String> {
        let script = dir.join("sampler.ps1");
        std::fs::write(&script, SCRIPT).map_err(|e| format!("{}: {e}", script.display()))?;
        let err_path = dir.join("sampler.err");
        let err = File::create(&err_path).map_err(|e| format!("{}: {e}", err_path.display()))?;
        let mut child = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(err)
            .spawn()
            .map_err(|e| format!("starting the sampler: {e}"))?;
        let input = child.stdin.take().expect("the sampler's input is piped");
        let output = child.stdout.take().expect("the sampler's output is piped");
        let (send, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines().map_while(Result::ok) {
                if send.send(line).is_err() {
                    break;
                }
            }
        });
        let mut sampler = Self {
            child,
            input,
            lines,
            rate: 0.0,
        };
        let first = sampler.answer()?;
        sampler.rate = first
            .strip_prefix("rate ")
            .and_then(|r| r.trim().parse().ok())
            .filter(|r: &f64| *r > 0.0)
            .ok_or_else(|| format!("the sampler began with {first:?}, not its rate"))?;
        Ok(sampler)
    }

    fn answer(&mut self) -> Result<String, String> {
        match self.lines.recv_timeout(ANSWER) {
            Ok(line) => Ok(line),
            Err(RecvTimeoutError::Timeout) => Err(format!(
                "the sampler did not answer within {} s",
                ANSWER.as_secs()
            )),
            Err(RecvTimeoutError::Disconnected) => Err("the sampler ended".into()),
        }
    }

    /// Samples the processes `asked` (process ids, or `audiodg`).
    pub fn sample(&mut self, asked: &[String]) -> Result<Sample, String> {
        writeln!(self.input, "{}", asked.join(" "))
            .and_then(|()| self.input.flush())
            .map_err(|e| format!("asking the sampler: {e}"))?;
        let line = self.answer()?;
        Sample::parse(&line).ok_or_else(|| format!("the sampler said {line:?}"))
    }

    /// The sampler's own process id, to check its cost.
    pub fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for Sampler {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_and_memory_from_two_samples() {
        // At 2e9 cycles a second over 2 s: the first process used 4e8
        // cycles (200 ms, 100 ms a second), the second 2e9 (1000 ms, 500
        // ms a second). The system was busy (kernel - idle + user) 18e6
        // units of 100 ns, 1800 ms, 900 ms a second.
        let a = Sample::parse(
            "1000 sys 100000000 150000000 20000000 | 11 1000 5 4 3 | 22 0 10 9 8 | audiodg gone",
        )
        .unwrap();
        let b = Sample::parse(
            "3000 sys 132000000 190000000 30000000 | 11 400001000 50 40 30 | 22 2000000000 100 90 80 | audiodg gone",
        )
        .unwrap();
        let both = usage(&a, &b, 2e9, &[0, 1]);
        assert!((both.span_ms - 2000.0).abs() < 1e-9);
        assert!((both.cpu_ms - 1200.0).abs() < 1e-6, "{both:?}");
        assert!((both.cpu_ms_per_s - 600.0).abs() < 1e-6, "{both:?}");
        assert_eq!(
            both.memory,
            Some(Memory {
                peak_ws: 150,
                ws: 130,
                private: 110
            })
        );
        assert!(both.gone.is_empty());
        assert!((system_busy(&a, &b) - 900.0).abs() < 1e-9);
        assert!((other_load(&a, &b, 2e9, &[0, 1]) - 300.0).abs() < 1e-6);
        // A process that is gone is reported gone, not as zero.
        let audio = usage(&a, &b, 2e9, &[2]);
        assert_eq!(audio.gone, ["audiodg"]);
        assert_eq!(audio.memory, None);
        let c = Sample::parse("5000 sys 1 2 3 | 11 gone | 22 3000000000 - - -").unwrap();
        let after = usage(&b, &c, 2e9, &[0, 1]);
        assert_eq!(after.gone, ["11"]);
        assert_eq!(after.memory, None);
        // Lines that are not the sampler's.
        assert_eq!(Sample::parse("rate 2000"), None);
        assert_eq!(Sample::parse("1 sys 1 2 3 | 11 x 1 2 3"), None);
        assert_eq!(Sample::parse("1 sys 1 2 3 | 11 1 2"), None);
    }

    /// The real sampler, on this test's own process and on a child that
    /// has already exited. No time is asserted.
    #[cfg(windows)]
    #[test]
    fn sampler_reads_this_process() {
        let dir = std::env::temp_dir().join(format!("fastemu-sampler-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut ended = Command::new("cmd").args(["/C", "exit 0"]).spawn().unwrap();
        let ended_pid = ended.id();
        ended.wait().unwrap();
        // The child is waited for and its handle closed: its id names no
        // running process now.
        drop(ended);
        let mut sampler = Sampler::start(&dir).expect("the sampler starts");
        assert!(sampler.rate > 0.0);
        let asked = [std::process::id().to_string(), ended_pid.to_string()];
        let sample = sampler.sample(&asked).expect("a sample");
        match &sample.procs[0] {
            Proc::Alive {
                pid,
                cycles,
                memory,
            } => {
                assert_eq!(*pid, std::process::id());
                assert!(*cycles > 0);
                let memory = memory.expect("this process's memory");
                assert!(memory.peak_ws > 0 && memory.private > 0);
            }
            gone @ Proc::Gone { .. } => panic!("this process is {gone:?}"),
        }
        assert_eq!(
            sample.procs[1],
            Proc::Gone {
                asked: ended_pid.to_string()
            }
        );
        assert!(sample.kernel >= sample.idle);
        drop(sampler);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
