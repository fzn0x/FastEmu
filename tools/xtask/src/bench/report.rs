//! What a bench measured, and `bench.md` and `bench.json` from it.
//!
//! Each number in `bench.md` is the median of the runs, with the smallest
//! and the largest beside it. A kernel's warm time in one run is the
//! median of its rounds after the first; its cold time is the first
//! repetition of the first round. The headline table gives each ratio in
//! words, such as `FastEmu 12.0x less` or `MicroEmulator 30.0x faster`.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use super::sample::Memory;

/// How much more than the idle baseline the other load may be before a
/// measurement counts as noisy, in CPU milliseconds a second.
pub const NOISE_MARGIN: f64 = 100.0;
/// How far apart the runs of one number may be, as a share of their
/// median, before the number counts as noisy.
pub const MAX_SPREAD: f64 = 0.15;
/// How far apart two frame rates may be, as a share, before their CPU
/// times are not compared.
pub const FPS_TOLERANCE: f64 = 0.05;
/// A time shorter than this many steps of the clock that took it is
/// marked as under the clock's resolution.
pub const CLOCK_STEPS: f64 = 20.0;

/// The CPU kernels and the memory kernels, whose checks must agree.
pub const CHECKED: [&str; 12] = [
    "cpu.int",
    "cpu.call",
    "cpu.sieve",
    "cpu.sort",
    "cpu.actors",
    "cpu.long",
    "cpu.float",
    "cpu.string",
    "cpu.collections",
    "mem.small",
    "mem.arrays",
    "mem.gc",
];

/// The drawing kernels.
pub const DRAWING: [&str; 8] = [
    "draw.fillrect",
    "draw.line",
    "draw.image",
    "draw.alpha",
    "draw.region",
    "draw.text",
    "draw.rgb",
    "draw.scene",
];

/// Which emulator ran, and how.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Side {
    FastEmu,
    MicroEmu,
    /// MicroEmulator with `-Xms16m -Xmx128m -XX:+UseSerialGC`.
    MicroEmuSmallHeap,
    /// MicroEmulator with its frame rate capped by MeBench's `--max-fps`.
    MicroEmuCapped,
}

impl Side {
    pub fn key(self) -> &'static str {
        match self {
            Side::FastEmu => "fastemu",
            Side::MicroEmu => "microemu",
            Side::MicroEmuSmallHeap => "microemu-small-heap",
            Side::MicroEmuCapped => "microemu-capped",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Side::FastEmu => "FastEmu",
            Side::MicroEmu => "MicroEmulator",
            Side::MicroEmuSmallHeap => "MicroEmulator, small heap",
            Side::MicroEmuCapped => "MicroEmulator, capped",
        }
    }
}

/// What a measurement ran.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Work {
    /// bench.jar: every kernel, then the unpaced frame loop.
    Kernels,
    /// scene.jar at 30 frames a second.
    Scene,
    /// One real game, by its slug.
    Game(String),
    /// Every real game at once.
    Eight,
    /// The desktop app with every game, against MicroEmulator windows.
    Windowed,
}

impl Work {
    pub fn key(&self) -> String {
        match self {
            Work::Kernels => "kernels".into(),
            Work::Scene => "scene".into(),
            Work::Game(slug) => format!("game-{slug}"),
            Work::Eight => "eight".into(),
            Work::Windowed => "windowed".into(),
        }
    }
}

/// One round of one kernel: `BENCH name round= reps= ms= first= check=`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelRound {
    pub name: String,
    pub round: i64,
    pub reps: i64,
    pub ms: i64,
    pub first: i64,
    pub check: i64,
}

/// How a game ended in a measurement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct End {
    pub status: String,
    pub frames: u64,
    pub errors: u64,
}

/// One measurement: one work, on one side, in one run.
#[derive(Clone, Debug)]
pub struct Measurement {
    pub work: Work,
    pub side: Side,
    /// From 1.
    pub run: usize,
    /// Which side went first in this measurement's pair.
    pub first_side: Side,
    /// From the process start to the first frame, on the bench's clock;
    /// with many games, to the first frame of the last of them.
    pub first_ms: Option<f64>,
    /// Frames a second in the sample window, summed over the games.
    pub fps: Option<f64>,
    /// The same per game.
    pub game_fps: Vec<Option<f64>>,
    /// CPU per second in the sample window, summed over the processes.
    pub cpu_ms_per_s: Option<f64>,
    /// For the kernels: CPU time from the first frame to `BENCH done`,
    /// and the wall time between those two samples.
    pub cpu_ms: Option<f64>,
    pub wall_ms: Option<f64>,
    /// At the second sample, summed over the processes.
    pub memory: Option<Memory>,
    /// The Windows audio engine's CPU per second in the window.
    pub audio_ms_per_s: Option<f64>,
    /// The sampler's own CPU per second in the window: its cost.
    pub sampler_ms_per_s: Option<f64>,
    /// The other load just before, and during, the measurement.
    pub quiet: Option<f64>,
    pub other: Option<f64>,
    /// The benchmark MIDlet's results.
    pub rounds: Vec<KernelRound>,
    pub frames_free: Option<f64>,
    pub screen: Option<(i64, i64)>,
    pub clock_step: Option<i64>,
    pub kernel_errors: Vec<(String, String)>,
    /// Per game: the size of its screen at the last report, how it ended,
    /// and its screenshot, relative to the report.
    pub sizes: Vec<Option<(i32, i32)>>,
    pub ends: Vec<Option<End>>,
    pub shots: Vec<Option<String>>,
    pub problems: Vec<String>,
    /// The raw sampler lines and output lines, for bench.json.
    pub samples: Vec<String>,
    pub lines: Vec<String>,
}

impl Measurement {
    pub fn new(work: Work, side: Side, run: usize, first_side: Side) -> Self {
        Self {
            work,
            side,
            run,
            first_side,
            first_ms: None,
            fps: None,
            game_fps: Vec::new(),
            cpu_ms_per_s: None,
            cpu_ms: None,
            wall_ms: None,
            memory: None,
            audio_ms_per_s: None,
            sampler_ms_per_s: None,
            quiet: None,
            other: None,
            rounds: Vec::new(),
            frames_free: None,
            screen: None,
            clock_step: None,
            kernel_errors: Vec::new(),
            sizes: Vec::new(),
            ends: Vec::new(),
            shots: Vec::new(),
            problems: Vec::new(),
            samples: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// The warm time of `name` in this run: the median time per
    /// repetition of the rounds after the first.
    pub fn warm(&self, name: &str) -> Option<f64> {
        let per_rep: Vec<f64> = self
            .rounds
            .iter()
            .filter(|r| r.name == name && r.round > 1 && r.reps > 0)
            .map(|r| r.ms as f64 / r.reps as f64)
            .collect();
        median(&per_rep)
    }

    /// The cores the kernel run kept busy: its CPU time over its wall time.
    pub fn cores(&self) -> Option<f64> {
        match (self.cpu_ms, self.wall_ms) {
            (Some(cpu), Some(wall)) if wall > 0.0 => Some(cpu / wall),
            _ => None,
        }
    }

    /// The cold time of `name`: the first repetition of the first round.
    pub fn cold(&self, name: &str) -> Option<f64> {
        self.rounds
            .iter()
            .find(|r| r.name == name && r.round == 1)
            .map(|r| r.first as f64)
    }

    /// Every check value `name` gave.
    pub fn checks(&self, name: &str) -> BTreeSet<i64> {
        self.rounds
            .iter()
            .filter(|r| r.name == name)
            .map(|r| r.check)
            .collect()
    }
}

/// A game of the corpus.
#[derive(Clone, Debug)]
pub struct GameInfo {
    pub slug: String,
    pub file: String,
}

/// Everything a bench run gathered.
#[derive(Clone, Debug, Default)]
pub struct Report {
    /// Machine, versions and settings, in order.
    pub facts: Vec<(String, String)>,
    /// The other load of the idle machine at the start, in ms a second.
    pub baseline: Option<f64>,
    pub games: Vec<GameInfo>,
    pub measurements: Vec<Measurement>,
    /// Problems of the run itself, besides the measurements'.
    pub problems: Vec<String>,
    pub fastemu_only: bool,
    /// MeBench's cap for the capped rows, when asked.
    pub me_cap: Option<u32>,
}

/// The median, smallest and largest of some runs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stat {
    pub median: f64,
    pub min: f64,
    pub max: f64,
    pub n: usize,
}

impl Stat {
    pub fn of(values: &[f64]) -> Option<Stat> {
        let values: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
        Some(Stat {
            median: median(&values)?,
            min: values.iter().copied().fold(f64::INFINITY, f64::min),
            max: values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
            n: values.len(),
        })
    }

    /// How far apart the runs are, as a share of the median.
    pub fn spread(&self) -> f64 {
        if self.median.abs() < f64::EPSILON {
            0.0
        } else {
            (self.max - self.min) / self.median.abs()
        }
    }

    /// `median (min to max)`, with `decimals` places.
    pub fn text(&self, decimals: usize) -> String {
        if self.n > 1 && (self.max - self.min).abs() > f64::EPSILON {
            format!(
                "{:.d$} ({:.d$} to {:.d$})",
                self.median,
                self.min,
                self.max,
                d = decimals
            )
        } else {
            format!("{:.d$}", self.median, d = decimals)
        }
    }
}

/// The median of `values`, the mean of the middle two for an even count.
pub fn median(values: &[f64]) -> Option<f64> {
    let mut sorted: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    if sorted.is_empty() {
        return None;
    }
    sorted.sort_by(f64::total_cmp);
    let mid = sorted.len() / 2;
    Some(if sorted.len().is_multiple_of(2) {
        f64::midpoint(sorted[mid - 1], sorted[mid])
    } else {
        sorted[mid]
    })
}

/// Which way a number is better.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Better {
    /// A time: the lower is `faster`.
    Faster,
    /// CPU or memory: the lower is `less`.
    Less,
    /// Frames: the higher is `more frames`.
    MoreFrames,
}

/// The ratio of `fastemu` to `microemu` in words, such as `FastEmu 12.0x
/// less` or `MicroEmulator 30.0x faster`.
pub fn verdict(fastemu: f64, microemu: f64, better: Better) -> String {
    if !(fastemu > 0.0 && microemu > 0.0) {
        return "n/a".into();
    }
    let fastemu_wins = match better {
        Better::Faster | Better::Less => fastemu < microemu,
        Better::MoreFrames => fastemu > microemu,
    };
    let ratio = fastemu.max(microemu) / fastemu.min(microemu);
    if ratio < 1.05 {
        return "about the same".into();
    }
    let who = if fastemu_wins {
        "FastEmu"
    } else {
        "MicroEmulator"
    };
    let word = match better {
        Better::Faster => "faster",
        Better::Less => "less",
        Better::MoreFrames => "more frames",
    };
    format!("{who} {ratio:.1}x {word}")
}

/// The verdicts of many pairs at once, as a range: `MicroEmulator 7.0x to
/// 98.0x faster`, or both directions when the winner changes.
pub fn verdict_range(pairs: &[(f64, f64)], better: Better) -> String {
    let mut fastemu = Vec::new();
    let mut microemu = Vec::new();
    let mut same = 0;
    for &(f, m) in pairs {
        if !(f > 0.0 && m > 0.0) {
            continue;
        }
        let ratio = f.max(m) / f.min(m);
        let fastemu_wins = match better {
            Better::Faster | Better::Less => f < m,
            Better::MoreFrames => f > m,
        };
        if ratio < 1.05 {
            same += 1;
        } else if fastemu_wins {
            fastemu.push(ratio);
        } else {
            microemu.push(ratio);
        }
    }
    let word = match better {
        Better::Faster => "faster",
        Better::Less => "less",
        Better::MoreFrames => "more frames",
    };
    let range = |who: &str, ratios: &[f64]| {
        let lo = ratios.iter().copied().fold(f64::INFINITY, f64::min);
        let hi = ratios.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        if ratios.len() == 1 || (hi - lo) < 0.05 {
            format!("{who} {hi:.1}x {word}")
        } else {
            format!("{who} {lo:.1}x to {hi:.1}x {word}")
        }
    };
    let mut parts = Vec::new();
    if !microemu.is_empty() {
        parts.push(format!(
            "{} in {}",
            range("MicroEmulator", &microemu),
            microemu.len()
        ));
    }
    if !fastemu.is_empty() {
        parts.push(format!(
            "{} in {}",
            range("FastEmu", &fastemu),
            fastemu.len()
        ));
    }
    if same > 0 {
        parts.push(format!("about the same in {same}"));
    }
    if parts.is_empty() {
        "n/a".into()
    } else {
        parts.join("; ")
    }
}

/// Whether a measurement's other load was too high: more than the
/// baseline plus [`NOISE_MARGIN`].
pub fn is_noisy(other: f64, baseline: f64) -> bool {
    other > baseline + NOISE_MARGIN
}

/// Whether two frame rates are close enough to compare CPU times.
pub fn same_rate(a: f64, b: f64) -> bool {
    a > 0.0 && b > 0.0 && (a - b).abs() / a.max(b) <= FPS_TOLERANCE
}

/// `value` milliseconds, marked when shorter than [`CLOCK_STEPS`] steps
/// of the clock that took it.
pub fn timed(value: f64, step: Option<i64>, decimals: usize) -> String {
    match step {
        Some(step) if step > 0 && value < CLOCK_STEPS * step as f64 => {
            format!("{value:.decimals$} (+/-{step})")
        }
        _ => format!("{value:.decimals$}"),
    }
}

fn mib(bytes: f64) -> f64 {
    bytes / (1024.0 * 1024.0)
}

/// The measurements of `work` on `side`.
fn of<'a>(report: &'a Report, work: &Work, side: Side) -> Vec<&'a Measurement> {
    report
        .measurements
        .iter()
        .filter(|m| &m.work == work && m.side == side)
        .collect()
}

fn stat_by(ms: &[&Measurement], f: impl Fn(&Measurement) -> Option<f64>) -> Option<Stat> {
    let values: Vec<f64> = ms.iter().filter_map(|m| f(m)).collect();
    Stat::of(&values)
}

fn cell(stat: Option<Stat>, decimals: usize, ran: bool) -> String {
    match stat {
        Some(s) => s.text(decimals),
        None if ran => "n/a".into(),
        None => "not run".into(),
    }
}

/// Why a game is left out of the ratios, if it is: no frames, a crash or
/// an end on one side only, or screens of different sizes.
pub fn left_out(fastemu: &[&Measurement], microemu: &[&Measurement]) -> Option<String> {
    let describe = |ms: &[&Measurement], label: &str| -> Option<String> {
        for m in ms {
            match m.ends.first().and_then(Option::as_ref) {
                Some(end) if end.frames == 0 => return Some(format!("no frames on {label}")),
                Some(end) if end.status == "crashed" => return Some(format!("crashed on {label}")),
                None => return Some(format!("no end reported on {label}")),
                _ => {}
            }
        }
        None
    };
    if let Some(why) = describe(fastemu, "FastEmu") {
        return Some(why);
    }
    if let Some(why) = describe(microemu, "MicroEmulator") {
        return Some(why);
    }
    let statuses = |ms: &[&Measurement]| -> BTreeSet<String> {
        ms.iter()
            .filter_map(|m| m.ends.first().and_then(Option::as_ref))
            .map(|e| e.status.clone())
            .collect()
    };
    let (sf, sm) = (statuses(fastemu), statuses(microemu));
    if sf != sm {
        return Some(format!(
            "it ended differently: {} on FastEmu, {} on MicroEmulator",
            sf.into_iter().collect::<Vec<_>>().join(", "),
            sm.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    let sizes = |ms: &[&Measurement]| -> BTreeSet<(i32, i32)> {
        ms.iter()
            .filter_map(|m| m.sizes.first().copied().flatten())
            .collect()
    };
    let (zf, zm) = (sizes(fastemu), sizes(microemu));
    if !zf.is_empty() && !zm.is_empty() && zf != zm {
        let show = |s: BTreeSet<(i32, i32)>| {
            s.into_iter()
                .map(|(w, h)| format!("{w}x{h}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        return Some(format!(
            "its screen differs: {} on FastEmu, {} on MicroEmulator",
            show(zf),
            show(zm)
        ));
    }
    None
}

/// A Markdown table.
struct Table {
    head: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl Table {
    fn new(head: &[&str]) -> Self {
        Self {
            head: head.iter().map(|&h| h.to_owned()).collect(),
            rows: Vec::new(),
        }
    }

    fn row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    fn render(&self, out: &mut String) {
        let line = |cells: &[String]| {
            let cells: Vec<String> = cells.iter().map(|c| c.replace('|', "\\|")).collect();
            format!("| {} |\n", cells.join(" | "))
        };
        out.push_str(&line(&self.head));
        let _ = writeln!(out, "|{}", " --- |".repeat(self.head.len()));
        for row in &self.rows {
            out.push_str(&line(row));
        }
        out.push('\n');
    }
}

/// The sides a measurement of `work` compares FastEmu with.
fn me_sides(report: &Report, work: &Work) -> Vec<Side> {
    [
        Side::MicroEmu,
        Side::MicroEmuSmallHeap,
        Side::MicroEmuCapped,
    ]
    .into_iter()
    .filter(|&s| !of(report, work, s).is_empty())
    .collect()
}

/// The problems of the run: what the measurements noted, checks that
/// differ, screens that are not 240x320, kernels that failed, games with
/// no frames, noisy measurements and numbers whose runs spread apart.
pub fn problems(report: &Report) -> Vec<String> {
    let mut out = report.problems.clone();
    let baseline = report.baseline.unwrap_or(0.0);
    for m in &report.measurements {
        let what = format!("{} on {}, run {}", m.work.key(), m.side.label(), m.run);
        for p in &m.problems {
            out.push(format!("{what}: {p}"));
        }
        if let Some(other) = m.other
            && is_noisy(other, baseline)
        {
            out.push(format!(
                "{what}: noisy, other load {other:.0} ms/s against a baseline of {baseline:.0}"
            ));
        }
        for (name, why) in &m.kernel_errors {
            out.push(format!("{what}: kernel {name} failed: {why}"));
        }
        if let Some((w, h)) = m.screen
            && (w, h) != (240, 320)
        {
            out.push(format!(
                "{what}: the MIDlet's screen is {w}x{h}, not 240x320"
            ));
        }
        if matches!(m.work, Work::Game(_) | Work::Eight) {
            for (i, end) in m.ends.iter().enumerate() {
                match end {
                    Some(end) if end.frames == 0 => {
                        out.push(format!("{what}: game {i} showed no frame"));
                    }
                    Some(end) if end.status == "crashed" => {
                        out.push(format!("{what}: game {i} crashed"));
                    }
                    None => out.push(format!("{what}: game {i} reported no end")),
                    _ => {}
                }
            }
        }
    }
    // Checks: one value per kernel on each side, and the same on both.
    let fastemu = of(report, &Work::Kernels, Side::FastEmu);
    for side in me_sides(report, &Work::Kernels)
        .into_iter()
        .chain([Side::FastEmu])
    {
        let ms = of(report, &Work::Kernels, side);
        for name in CHECKED {
            let all: BTreeSet<i64> = ms.iter().flat_map(|m| m.checks(name)).collect();
            if all.len() > 1 {
                out.push(format!(
                    "{name} gave different checks on {}: {all:?}",
                    side.label()
                ));
            }
        }
    }
    let microemu = of(report, &Work::Kernels, Side::MicroEmu);
    if !microemu.is_empty() {
        for name in CHECKED {
            let f: BTreeSet<i64> = fastemu.iter().flat_map(|m| m.checks(name)).collect();
            let m: BTreeSet<i64> = microemu.iter().flat_map(|m| m.checks(name)).collect();
            if !f.is_empty() && !m.is_empty() && f != m {
                out.push(format!(
                    "{name}: the check differs: {f:?} on FastEmu, {m:?} on MicroEmulator"
                ));
            }
        }
    }
    // Numbers whose runs spread apart.
    let mut seen = BTreeSet::new();
    for m in &report.measurements {
        if !seen.insert((m.work.clone(), m.side)) {
            continue;
        }
        let ms = of(report, &m.work, m.side);
        let mut spread = |label: &str, stat: Option<Stat>| {
            if let Some(s) = stat
                && s.n > 1
                && s.spread() > MAX_SPREAD
            {
                out.push(format!(
                    "{} on {}: {label} spreads {:.0} percent across runs ({})",
                    m.work.key(),
                    m.side.label(),
                    s.spread() * 100.0,
                    s.text(1)
                ));
            }
        };
        spread("CPU per second", stat_by(&ms, |m| m.cpu_ms_per_s));
        spread("frames per second", stat_by(&ms, |m| m.fps));
        spread("kernel CPU time", stat_by(&ms, |m| m.cpu_ms));
    }
    out
}

/// The headline: each ratio in words.
fn headline(report: &Report, out: &mut String) {
    let mut table = Table::new(&["Measure", "FastEmu", "MicroEmulator", "Ratio"]);
    let fe_k = of(report, &Work::Kernels, Side::FastEmu);
    let me_k = of(report, &Work::Kernels, Side::MicroEmu);
    let ran = !report.fastemu_only;
    let pair_row = |table: &mut Table,
                    label: &str,
                    f: Option<Stat>,
                    m: Option<Stat>,
                    decimals: usize,
                    better: Better| {
        let ratio = match (f, m) {
            (Some(f), Some(m)) => verdict(f.median, m.median, better),
            _ => "n/a".into(),
        };
        table.row(vec![
            label.into(),
            cell(f, decimals, true),
            cell(m, decimals, ran),
            ratio,
        ]);
    };
    let fe_s = of(report, &Work::Scene, Side::FastEmu);
    let me_s = of(report, &Work::Scene, Side::MicroEmu);
    pair_row(
        &mut table,
        "Start to first frame, scene, ms",
        stat_by(&fe_s, |m| m.first_ms),
        stat_by(&me_s, |m| m.first_ms),
        0,
        Better::Faster,
    );
    // Kernels by group, as ranges.
    let groups: [(&str, &[&str], bool); 5] = [
        ("CPU kernels, warm", &CHECKED[..9], true),
        ("CPU kernels, cold", &CHECKED[..9], false),
        ("Memory kernels, warm", &CHECKED[9..], true),
        ("Drawing kernels, warm", &DRAWING, true),
        ("Drawing kernels, cold", &DRAWING, false),
    ];
    for (label, names, warm) in groups {
        let pairs: Vec<(f64, f64)> = names
            .iter()
            .filter_map(|name| {
                let pick = |m: &Measurement| {
                    if warm { m.warm(name) } else { m.cold(name) }
                };
                Some((stat_by(&fe_k, pick)?.median, stat_by(&me_k, pick)?.median))
            })
            .collect();
        let ratio = if pairs.is_empty() {
            "n/a".into()
        } else {
            verdict_range(&pairs, Better::Faster)
        };
        table.row(vec![
            label.into(),
            "see below".into(),
            if ran { "see below" } else { "not run" }.into(),
            ratio,
        ]);
    }
    pair_row(
        &mut table,
        "Kernel run, cores in use (CPU time over wall time)",
        stat_by(&fe_k, Measurement::cores),
        stat_by(&me_k, Measurement::cores),
        2,
        Better::Less,
    );
    pair_row(
        &mut table,
        "Unpaced frame loop, frames a second",
        stat_by(&fe_k, |m| m.frames_free),
        stat_by(&me_k, |m| m.frames_free),
        1,
        Better::MoreFrames,
    );
    let (ff, mf) = (
        stat_by(&fe_s, |m| m.fps).map(|s| s.median),
        stat_by(&me_s, |m| m.fps).map(|s| s.median),
    );
    let scene_cpu_ok = matches!((ff, mf), (Some(a), Some(b)) if same_rate(a, b));
    let (fc, mc) = (
        stat_by(&fe_s, |m| m.cpu_ms_per_s),
        stat_by(&me_s, |m| m.cpu_ms_per_s),
    );
    if scene_cpu_ok || !ran {
        pair_row(
            &mut table,
            "Scene at 30 fps, CPU ms a second",
            fc,
            mc,
            1,
            Better::Less,
        );
    } else {
        table.row(vec![
            "Scene at 30 fps, CPU ms a second".into(),
            cell(fc, 1, true),
            cell(mc, 1, ran),
            "left out: the frame rates differ".into(),
        ]);
    }
    let peak = |m: &Measurement| m.memory.map(|x| mib(x.peak_ws as f64));
    let private = |m: &Measurement| m.memory.map(|x| mib(x.private as f64));
    pair_row(
        &mut table,
        "Scene, peak working set, MiB",
        stat_by(&fe_s, peak),
        stat_by(&me_s, peak),
        1,
        Better::Less,
    );
    pair_row(
        &mut table,
        "Scene, private bytes, MiB",
        stat_by(&fe_s, private),
        stat_by(&me_s, private),
        1,
        Better::Less,
    );
    let me_small = of(report, &Work::Scene, Side::MicroEmuSmallHeap);
    if !me_small.is_empty() {
        pair_row(
            &mut table,
            "Scene, peak working set, MiB, MicroEmulator small heap",
            stat_by(&fe_s, peak),
            stat_by(&me_small, peak),
            1,
            Better::Less,
        );
    }
    // Real games, as ranges over the games both sides ran alike.
    let mut cpu = Vec::new();
    let mut mem = Vec::new();
    let mut first = Vec::new();
    for game in &report.games {
        let work = Work::Game(game.slug.clone());
        let (f, m) = (
            of(report, &work, Side::FastEmu),
            of(report, &work, Side::MicroEmu),
        );
        if m.is_empty() || left_out(&f, &m).is_some() {
            continue;
        }
        let med = |ms: &[&Measurement], g: &dyn Fn(&Measurement) -> Option<f64>| {
            stat_by(ms, g).map(|s| s.median)
        };
        // CPU only at the same frame rate: MicroEmulator as a player gets
        // it, else capped like FastEmu when the capped rows ran.
        let capped = of(report, &work, Side::MicroEmuCapped);
        let same = [&m, &capped].into_iter().find_map(|me| {
            let (a, b) = (med(&f, &|x| x.fps)?, med(me, &|x| x.fps)?);
            if !same_rate(a, b) {
                return None;
            }
            Some((med(&f, &|x| x.cpu_ms_per_s)?, med(me, &|x| x.cpu_ms_per_s)?))
        });
        if let Some(pair) = same {
            cpu.push(pair);
        }
        if let (Some(x), Some(y)) = (med(&f, &peak), med(&m, &peak)) {
            mem.push((x, y));
        }
        if let (Some(x), Some(y)) = (med(&f, &|m| m.first_ms), med(&m, &|m| m.first_ms)) {
            first.push((x, y));
        }
    }
    if !report.games.is_empty() {
        for (label, pairs, better) in [
            ("Real games, start to first frame", &first, Better::Faster),
            ("Real games, CPU at the same frame rate", &cpu, Better::Less),
            ("Real games, peak working set", &mem, Better::Less),
        ] {
            table.row(vec![
                label.into(),
                "see below".into(),
                if ran { "see below" } else { "not run" }.into(),
                if pairs.is_empty() {
                    "n/a".into()
                } else {
                    verdict_range(pairs, better)
                },
            ]);
        }
        let fe_e = of(report, &Work::Eight, Side::FastEmu);
        let me_e = of(report, &Work::Eight, Side::MicroEmu);
        pair_row(
            &mut table,
            "Eight at once, peak working set, MiB (MicroEmulator: sum of 8 processes)",
            stat_by(&fe_e, peak),
            stat_by(&me_e, peak),
            1,
            Better::Less,
        );
        pair_row(
            &mut table,
            "Eight at once, private bytes, MiB (MicroEmulator: sum)",
            stat_by(&fe_e, private),
            stat_by(&me_e, private),
            1,
            Better::Less,
        );
        let (a, b) = (
            stat_by(&fe_e, |m| m.fps).map(|s| s.median),
            stat_by(&me_e, |m| m.fps).map(|s| s.median),
        );
        let (fc, mc) = (
            stat_by(&fe_e, |m| m.cpu_ms_per_s),
            stat_by(&me_e, |m| m.cpu_ms_per_s),
        );
        if matches!((a, b), (Some(a), Some(b)) if same_rate(a, b)) || !ran {
            pair_row(
                &mut table,
                "Eight at once, CPU ms a second",
                fc,
                mc,
                1,
                Better::Less,
            );
        } else {
            table.row(vec![
                "Eight at once, CPU ms a second".into(),
                cell(fc, 1, true),
                cell(mc, 1, ran),
                "left out: the frame rates differ".into(),
            ]);
        }
    }
    let fe_w = of(report, &Work::Windowed, Side::FastEmu);
    let me_w = of(report, &Work::Windowed, Side::MicroEmu);
    if !fe_w.is_empty() || !me_w.is_empty() {
        pair_row(
            &mut table,
            "Windows open, every game, peak working set, MiB",
            stat_by(&fe_w, peak),
            stat_by(&me_w, peak),
            1,
            Better::Less,
        );
    }
    table.render(out);
}

/// One table of kernels: warm and cold per side, the ratio, the checks.
fn kernel_table(report: &Report, names: &[&str], checked: bool, out: &mut String) {
    let fe = of(report, &Work::Kernels, Side::FastEmu);
    let mut head = vec!["Kernel", "FastEmu warm", "FastEmu cold"];
    let sides = me_sides(report, &Work::Kernels);
    let me_sides_shown: Vec<Side> = if sides.is_empty() {
        vec![Side::MicroEmu]
    } else {
        sides
    };
    let labels: Vec<String> = me_sides_shown
        .iter()
        .flat_map(|s| [format!("{} warm", s.label()), format!("{} cold", s.label())])
        .collect();
    head.extend(labels.iter().map(String::as_str));
    head.push("Warm ratio");
    if checked {
        head.push("Check");
    }
    let mut table = Table::new(&head);
    let step = |ms: &[&Measurement]| ms.iter().filter_map(|m| m.clock_step).max();
    let fe_step = step(&fe);
    for name in names {
        let fw = stat_by(&fe, |m| m.warm(name));
        let fc = stat_by(&fe, |m| m.cold(name));
        let mut row = vec![
            (*name).to_owned(),
            cell(fw, 2, true),
            fc.map_or_else(|| "n/a".into(), |s| timed(s.median, fe_step, 0)),
        ];
        let mut ratio = "n/a".to_owned();
        let mut check = String::new();
        for (i, side) in me_sides_shown.iter().enumerate() {
            let ms = of(report, &Work::Kernels, *side);
            let ran = !ms.is_empty();
            let mw = stat_by(&ms, |m| m.warm(name));
            let mc = stat_by(&ms, |m| m.cold(name));
            row.push(cell(mw, 2, ran));
            row.push(match mc {
                Some(s) => timed(s.median, step(&ms), 0),
                None if ran => "n/a".into(),
                None => "not run".into(),
            });
            if i == 0 {
                if let (Some(f), Some(m)) = (fw, mw) {
                    ratio = verdict(f.median, m.median, Better::Faster);
                }
                let f: BTreeSet<i64> = fe.iter().flat_map(|m| m.checks(name)).collect();
                let m: BTreeSet<i64> = ms.iter().flat_map(|m| m.checks(name)).collect();
                check = match (f.len(), m.len()) {
                    (0, _) => "n/a".into(),
                    (_, 0) if !ran => "not run".into(),
                    (_, 0) => "n/a".into(),
                    _ if f == m && f.len() == 1 => "equal".into(),
                    _ => "differs".into(),
                };
            }
        }
        row.push(ratio);
        if checked {
            row.push(check);
        }
        table.row(row);
    }
    table.render(out);
}

/// The rows of a timed work (scene, game, eight at once): one per side.
fn timed_rows(report: &Report, work: &Work, table: &mut Table, label: &str) {
    let sides = [Side::FastEmu]
        .into_iter()
        .chain(if report.fastemu_only {
            vec![Side::MicroEmu]
        } else {
            me_sides(report, work)
        })
        .collect::<Vec<_>>();
    for side in sides {
        let ms = of(report, work, side);
        let ran = !ms.is_empty();
        let fps = stat_by(&ms, |m| m.fps);
        let cpu = stat_by(&ms, |m| m.cpu_ms_per_s);
        let per_frame = stat_by(&ms, |m| match (m.cpu_ms_per_s, m.fps) {
            (Some(c), Some(f)) if f > 0.0 => Some(c / f),
            _ => None,
        });
        let sizes: BTreeSet<String> = ms
            .iter()
            .flat_map(|m| m.sizes.iter().flatten())
            .map(|(w, h)| format!("{w}x{h}"))
            .collect();
        let statuses: BTreeSet<String> = ms
            .iter()
            .flat_map(|m| m.ends.iter().flatten())
            .map(|e| e.status.clone())
            .collect();
        let shot = ms
            .iter()
            .rev()
            .find_map(|m| m.shots.first().cloned().flatten())
            .map_or_else(String::new, |p| format!("[shot]({p})"));
        let join = |s: BTreeSet<String>| s.into_iter().collect::<Vec<_>>().join(", ");
        let side_label = match (side, report.me_cap) {
            (Side::MicroEmuCapped, Some(cap)) => format!("MicroEmulator, capped at {cap} fps"),
            _ => side.label().to_owned(),
        };
        table.row(vec![
            label.into(),
            side_label,
            cell(stat_by(&ms, |m| m.first_ms), 0, ran),
            cell(fps, 1, ran),
            cell(cpu, 1, ran),
            cell(per_frame, 2, ran),
            cell(
                stat_by(&ms, |m| m.memory.map(|x| mib(x.peak_ws as f64))),
                1,
                ran,
            ),
            cell(stat_by(&ms, |m| m.memory.map(|x| mib(x.ws as f64))), 1, ran),
            cell(
                stat_by(&ms, |m| m.memory.map(|x| mib(x.private as f64))),
                1,
                ran,
            ),
            cell(stat_by(&ms, |m| m.audio_ms_per_s), 1, ran),
            if ran { join(sizes) } else { String::new() },
            if ran { join(statuses) } else { String::new() },
            shot,
        ]);
    }
}

const TIMED_HEAD: [&str; 13] = [
    "Work",
    "Emulator",
    "First frame, ms",
    "fps",
    "CPU ms/s",
    "CPU ms/frame",
    "Peak WS, MiB",
    "WS, MiB",
    "Private, MiB",
    "audiodg ms/s",
    "Screen",
    "End",
    "Picture",
];

/// `bench.md`.
pub fn markdown(report: &Report) -> String {
    let mut out = String::from("# FastEmu and MicroEmulator: bench results\n\n");
    let mut facts = Table::new(&["What", "Value"]);
    for (k, v) in &report.facts {
        facts.row(vec![k.clone(), v.clone()]);
    }
    facts.render(&mut out);
    out.push_str(
        "Each number is the median of the runs, with the smallest and the largest \
         in brackets. Times are in milliseconds, memory in MiB. `+/-S` marks a time \
         under 20 steps of its clock: it is within S ms of the true time. Working set \
         (WS) is the memory in RAM; private bytes is what the process reserved for \
         itself, which a JVM's heap makes large.\n\n",
    );
    out.push_str("## Headline\n\n");
    headline(report, &mut out);
    let any_kernels = report.measurements.iter().any(|m| m.work == Work::Kernels);
    if any_kernels {
        out.push_str("## CPU kernels\n\nTime per repetition, ms.\n\n");
        kernel_table(report, &CHECKED[..9], true, &mut out);
        out.push_str(
            "## Memory kernels\n\nTime per repetition, ms; mem.gc is one full collection.\n\n",
        );
        kernel_table(report, &CHECKED[9..], true, &mut out);
        out.push_str("## Drawing kernels\n\nTime per repetition, ms, into a 240x320 image.\n\n");
        kernel_table(report, &DRAWING, false, &mut out);
        out.push_str("## The kernel run\n\n");
        let mut table = Table::new(&[
            "Emulator",
            "Screen",
            "Clock step, ms",
            "CPU time, ms",
            "Wall time, ms",
            "Cores in use",
            "Unpaced frames a second",
        ]);
        for side in [Side::FastEmu]
            .into_iter()
            .chain(me_sides(report, &Work::Kernels))
        {
            let ms = of(report, &Work::Kernels, side);
            let screens: BTreeSet<String> = ms
                .iter()
                .filter_map(|m| m.screen)
                .map(|(w, h)| format!("{w}x{h}"))
                .collect();
            table.row(vec![
                side.label().into(),
                screens.into_iter().collect::<Vec<_>>().join(", "),
                ms.iter()
                    .filter_map(|m| m.clock_step)
                    .max()
                    .map_or_else(|| "n/a".into(), |s| s.to_string()),
                cell(stat_by(&ms, |m| m.cpu_ms), 0, true),
                cell(stat_by(&ms, |m| m.wall_ms), 0, true),
                cell(stat_by(&ms, Measurement::cores), 2, true),
                cell(stat_by(&ms, |m| m.frames_free), 1, true),
            ]);
        }
        table.render(&mut out);
        out.push_str(
            "Each round of a kernel lasts at least its set time on both emulators, \
             so the run takes about as long on each. CPU time counts every thread \
             of the process, so a JVM's compiler and collector threads on other \
             cores count too: cores in use above 1 is that work. The kernel times \
             above are the MIDlet's own clock.\n\n",
        );
    }
    let mut timed = Table::new(&TIMED_HEAD);
    if report.measurements.iter().any(|m| m.work == Work::Scene) {
        timed_rows(report, &Work::Scene, &mut timed, "scene, 30 fps");
    }
    for game in &report.games {
        timed_rows(
            report,
            &Work::Game(game.slug.clone()),
            &mut timed,
            &game.slug,
        );
    }
    if report.measurements.iter().any(|m| m.work == Work::Eight) {
        timed_rows(report, &Work::Eight, &mut timed, "all at once");
    }
    if !timed.rows.is_empty() {
        out.push_str(
            "## The scene and the real games\n\nCPU and memory come from two samples: \
             fps from the frames the emulator reported between them. For many \
             MicroEmulator processes, CPU and memory are the sums of the processes. \
             The screen is the size the game drew on: FastEmu's frame, and the \
             current screen's getWidth and getHeight on MicroEmulator. For all \
             at once, the first frame is when the last game showed its first; \
             FastEmu starts the games one after another in one process, \
             MicroEmulator starts one process for each at the same time. The \
             error counts are not compared: MicroEmulator counts every line a \
             game writes to System.err, FastEmu counts uncaught exceptions.\n\n",
        );
        timed.render(&mut out);
    }
    if !report.games.is_empty() {
        out.push_str("### Games left out of the ratios\n\n");
        let mut any = false;
        for game in &report.games {
            let work = Work::Game(game.slug.clone());
            let (f, m) = (
                of(report, &work, Side::FastEmu),
                of(report, &work, Side::MicroEmu),
            );
            if m.is_empty() {
                continue;
            }
            let why = left_out(&f, &m).or_else(|| {
                let a = stat_by(&f, |x| x.fps)?.median;
                let b = stat_by(&m, |x| x.fps)?.median;
                (!same_rate(a, b))
                    .then(|| format!("CPU only: the frame rates differ, {a:.1} against {b:.1} fps"))
            });
            if let Some(why) = why {
                any = true;
                let _ = writeln!(out, "- {}: {why}", game.slug);
            }
        }
        if !any {
            out.push_str("None.\n");
        }
        out.push('\n');
    }
    if report.measurements.iter().any(|m| m.work == Work::Windowed) {
        out.push_str(
            "## Windows open\n\nThe desktop app with every game in one window, and one \
             MicroEmulator window per game. No keys are pressed, so every game sits on \
             its title screen, and no frames are counted: compare the CPU only beside \
             the headless frame rates above.\n\n",
        );
        let mut table = Table::new(&[
            "Emulator",
            "CPU ms/s",
            "Peak WS, MiB",
            "WS, MiB",
            "Private, MiB",
            "audiodg ms/s",
        ]);
        for side in [Side::FastEmu, Side::MicroEmu] {
            let ms = of(report, &Work::Windowed, side);
            let ran = !ms.is_empty();
            table.row(vec![
                side.label().into(),
                cell(stat_by(&ms, |m| m.cpu_ms_per_s), 1, ran),
                cell(
                    stat_by(&ms, |m| m.memory.map(|x| mib(x.peak_ws as f64))),
                    1,
                    ran,
                ),
                cell(stat_by(&ms, |m| m.memory.map(|x| mib(x.ws as f64))), 1, ran),
                cell(
                    stat_by(&ms, |m| m.memory.map(|x| mib(x.private as f64))),
                    1,
                    ran,
                ),
                cell(stat_by(&ms, |m| m.audio_ms_per_s), 1, ran),
            ]);
        }
        table.render(&mut out);
    }
    out.push_str("## Load\n\n");
    let _ = writeln!(
        out,
        "Baseline of the idle machine: {} ms/s. A measurement is noisy when the \
         other load during it passes the baseline by more than {NOISE_MARGIN:.0} ms/s, \
         and a number is noisy when its runs spread more than {:.0} percent from \
         the median.\n",
        report
            .baseline
            .map_or_else(|| "n/a".into(), |b| format!("{b:.0}")),
        MAX_SPREAD * 100.0
    );
    let sampler: Vec<f64> = report
        .measurements
        .iter()
        .filter_map(|m| m.sampler_ms_per_s)
        .collect();
    if let Some(s) = Stat::of(&sampler) {
        let _ = writeln!(
            out,
            "The sampler itself used {} ms/s of CPU in a measurement window.\n",
            s.text(2)
        );
    }
    out.push_str("## Problems\n\n");
    let problems = problems(report);
    if problems.is_empty() {
        out.push_str("None.\n");
    }
    for p in problems {
        let _ = writeln!(out, "- {p}");
    }
    out
}

/// A JSON string.
pub fn string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn number(value: Option<f64>) -> String {
    value
        .filter(|v| v.is_finite())
        .map_or_else(|| "null".into(), |v| format!("{v:.3}"))
}

fn list(items: impl IntoIterator<Item = String>) -> String {
    format!("[{}]", items.into_iter().collect::<Vec<_>>().join(", "))
}

/// `bench.json`: the settings, and every measurement with its raw lines.
pub fn json(report: &Report) -> String {
    let mut out = String::from("{\n");
    let facts: Vec<String> = report
        .facts
        .iter()
        .map(|(k, v)| format!("{}: {}", string(k), string(v)))
        .collect();
    let _ = writeln!(out, "  \"facts\": {{{}}},", facts.join(", "));
    let _ = writeln!(out, "  \"baseline\": {},", number(report.baseline));
    let _ = writeln!(
        out,
        "  \"games\": {},",
        list(report.games.iter().map(|g| format!(
            "{{\"slug\": {}, \"file\": {}}}",
            string(&g.slug),
            string(&g.file)
        )))
    );
    let _ = writeln!(
        out,
        "  \"problems\": {},",
        list(problems(report).iter().map(|p| string(p)))
    );
    out.push_str("  \"measurements\": [\n");
    for (i, m) in report.measurements.iter().enumerate() {
        let memory = m.memory.map_or_else(
            || "null".into(),
            |x| {
                format!(
                    "{{\"peak_ws\": {}, \"ws\": {}, \"private\": {}}}",
                    x.peak_ws, x.ws, x.private
                )
            },
        );
        let rounds = list(m.rounds.iter().map(|r| {
            format!(
                "{{\"name\": {}, \"round\": {}, \"reps\": {}, \"ms\": {}, \"first\": {}, \"check\": {}}}",
                string(&r.name),
                r.round,
                r.reps,
                r.ms,
                r.first,
                r.check
            )
        }));
        let ends = list(m.ends.iter().map(|e| {
            e.as_ref().map_or_else(
                || "null".into(),
                |e| {
                    format!(
                        "{{\"status\": {}, \"frames\": {}, \"errors\": {}}}",
                        string(&e.status),
                        e.frames,
                        e.errors
                    )
                },
            )
        }));
        let sizes = list(
            m.sizes
                .iter()
                .map(|s| s.map_or_else(|| "null".into(), |(w, h)| format!("[{w}, {h}]"))),
        );
        let fields =
            [
                ("work", string(&m.work.key())),
                ("side", string(m.side.key())),
                ("run", m.run.to_string()),
                ("first_side", string(m.first_side.key())),
                ("first_ms", number(m.first_ms)),
                ("fps", number(m.fps)),
                ("game_fps", list(m.game_fps.iter().map(|f| number(*f)))),
                ("cpu_ms_per_s", number(m.cpu_ms_per_s)),
                ("cpu_ms", number(m.cpu_ms)),
                ("wall_ms", number(m.wall_ms)),
                ("memory", memory),
                ("audiodg_ms_per_s", number(m.audio_ms_per_s)),
                ("sampler_ms_per_s", number(m.sampler_ms_per_s)),
                ("quiet_before", number(m.quiet)),
                ("other_load", number(m.other)),
                ("frames_free", number(m.frames_free)),
                (
                    "screen",
                    m.screen
                        .map_or_else(|| "null".into(), |(w, h)| format!("[{w}, {h}]")),
                ),
                (
                    "clock_step",
                    m.clock_step
                        .map_or_else(|| "null".into(), |s| s.to_string()),
                ),
                ("rounds", rounds),
                (
                    "kernel_errors",
                    list(m.kernel_errors.iter().map(|(n, w)| {
                        format!("{{\"name\": {}, \"what\": {}}}", string(n), string(w))
                    })),
                ),
                ("sizes", sizes),
                ("ends", ends),
                (
                    "shots",
                    list(
                        m.shots
                            .iter()
                            .map(|s| s.as_deref().map_or_else(|| "null".into(), string)),
                    ),
                ),
                ("problems", list(m.problems.iter().map(|p| string(p)))),
                ("samples", list(m.samples.iter().map(|s| string(s)))),
                ("lines", list(m.lines.iter().map(|s| string(s)))),
            ];
        let body: Vec<String> = fields
            .iter()
            .map(|(k, v)| format!("      \"{k}\": {v}"))
            .collect();
        let comma = if i + 1 < report.measurements.len() {
            ","
        } else {
            ""
        };
        let _ = writeln!(out, "    {{\n{}\n    }}{comma}", body.join(",\n"));
    }
    out.push_str("  ]\n}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kernels(side: Side, run: usize, int_ms: &[i64], check: i64, step: i64) -> Measurement {
        let mut m = Measurement::new(Work::Kernels, side, run, Side::FastEmu);
        for (i, &ms) in int_ms.iter().enumerate() {
            m.rounds.push(KernelRound {
                name: "cpu.int".into(),
                round: i as i64 + 1,
                reps: 2,
                ms,
                first: ms / 2,
                check,
            });
        }
        m.clock_step = Some(step);
        m.screen = Some((240, 320));
        m
    }

    fn game(
        side: Side,
        run: usize,
        fps: f64,
        cpu: f64,
        size: (i32, i32),
        frames: u64,
    ) -> Measurement {
        let mut m = Measurement::new(Work::Game("g".into()), side, run, Side::FastEmu);
        m.fps = Some(fps);
        m.cpu_ms_per_s = Some(cpu);
        m.sizes = vec![Some(size)];
        m.ends = vec![Some(End {
            status: "running".into(),
            frames,
            errors: 0,
        })];
        m
    }

    #[test]
    fn medians_ratios_and_flags() {
        // Odd and even medians, with the smallest and largest.
        assert_eq!(median(&[3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(&[4.0, 1.0, 3.0, 2.0]), Some(2.5));
        assert_eq!(median(&[]), None);
        let s = Stat::of(&[10.0, 12.0, 11.0, 30.0]).unwrap();
        assert_eq!((s.median, s.min, s.max, s.n), (11.5, 10.0, 30.0, 4));
        assert_eq!(s.text(1), "11.5 (10.0 to 30.0)");
        assert_eq!(Stat::of(&[5.0]).unwrap().text(0), "5");

        // Ratios in words, both ways.
        assert_eq!(verdict(10.0, 120.0, Better::Less), "FastEmu 12.0x less");
        assert_eq!(
            verdict(300.0, 10.0, Better::Faster),
            "MicroEmulator 30.0x faster"
        );
        assert_eq!(
            verdict(60.0, 30.0, Better::MoreFrames),
            "FastEmu 2.0x more frames"
        );
        assert_eq!(verdict(10.0, 10.2, Better::Less), "about the same");
        assert_eq!(verdict(0.0, 10.0, Better::Less), "n/a");
        assert_eq!(
            verdict_range(&[(70.0, 10.0), (100.0, 1.0)], Better::Faster),
            "MicroEmulator 7.0x to 100.0x faster in 2"
        );
        assert_eq!(
            verdict_range(&[(1.0, 2.0), (30.0, 10.0), (5.0, 5.0)], Better::Faster),
            "MicroEmulator 3.0x faster in 1; FastEmu 2.0x faster in 1; about the same in 1"
        );

        // Warm is the median of the rounds after the first; cold is the
        // first repetition of round 1.
        let f1 = kernels(Side::FastEmu, 1, &[1000, 500, 600, 700], 127, 1);
        assert_eq!(f1.warm("cpu.int"), Some(300.0));
        assert_eq!(f1.cold("cpu.int"), Some(500.0));

        // A check that differs between the emulators says so.
        let mut report = Report {
            measurements: vec![
                f1.clone(),
                kernels(Side::MicroEmu, 1, &[40, 20, 20, 20], 128, 16),
            ],
            ..Report::default()
        };
        let md = markdown(&report);
        assert!(md.contains("| differs |"), "{md}");
        assert!(
            problems(&report)
                .iter()
                .any(|p| p.contains("cpu.int: the check differs")),
            "{:?}",
            problems(&report)
        );
        // A cold time under 20 steps of its clock is marked.
        assert!(md.contains("| 20 (+/-16) |"), "{md}");
        assert_eq!(timed(500.0, Some(1), 0), "500");
        assert_eq!(timed(15.0, Some(1), 0), "15 (+/-1)");
        assert_eq!(timed(15.0, None, 0), "15");
        report.measurements[1]
            .rounds
            .iter_mut()
            .for_each(|r| r.check = 127);
        let md = markdown(&report);
        assert!(md.contains("| equal |"), "{md}");
        assert!(md.contains("MicroEmulator 30.0x faster"), "{md}");

        // FastEmu alone: the other columns say so.
        let alone = Report {
            measurements: vec![f1],
            fastemu_only: true,
            ..Report::default()
        };
        assert!(markdown(&alone).contains("| not run |"));

        // A game with no frames on one side is left out, with its reason;
        // so is one whose screen differs. Different frame rates keep only
        // its CPU out.
        let f = game(Side::FastEmu, 1, 30.0, 10.0, (240, 320), 900);
        let none = game(Side::MicroEmu, 1, 0.0, 5.0, (240, 320), 0);
        assert_eq!(
            left_out(&[&f], &[&none]).as_deref(),
            Some("no frames on MicroEmulator")
        );
        let small = game(Side::MicroEmu, 1, 30.0, 5.0, (176, 176), 900);
        assert_eq!(
            left_out(&[&f], &[&small]).as_deref(),
            Some("its screen differs: 240x320 on FastEmu, 176x176 on MicroEmulator")
        );
        let fast = game(Side::MicroEmu, 1, 60.0, 50.0, (240, 320), 1800);
        assert_eq!(left_out(&[&f], &[&fast]), None);
        assert!(!same_rate(30.0, 60.0));
        assert!(same_rate(30.0, 29.0));
        let games = Report {
            games: vec![GameInfo {
                slug: "g".into(),
                file: "g.jar".into(),
            }],
            measurements: vec![f.clone(), fast],
            ..Report::default()
        };
        let md = markdown(&games);
        assert!(
            md.contains("- g: CPU only: the frame rates differ, 30.0 against 60.0 fps"),
            "{md}"
        );
        assert!(
            md.contains("| Real games, CPU at the same frame rate | see below | see below | n/a |"),
            "{md}"
        );

        // Noisy: other load over the baseline plus the margin, or runs
        // that spread more than 15 percent.
        assert!(!is_noisy(150.0, 100.0));
        assert!(is_noisy(201.0, 100.0));
        let mut a = f.clone();
        a.other = Some(400.0);
        let mut b = f;
        b.run = 2;
        b.cpu_ms_per_s = Some(20.0);
        let noisy = Report {
            baseline: Some(100.0),
            measurements: vec![a, b],
            ..Report::default()
        };
        let found = problems(&noisy);
        assert!(
            found
                .iter()
                .any(|p| p.contains("noisy, other load 400 ms/s")),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|p| p.contains("CPU per second spreads 67 percent")),
            "{found:?}"
        );
        assert!(json(&noisy).contains("\"work\": \"game-g\""));
    }
}
