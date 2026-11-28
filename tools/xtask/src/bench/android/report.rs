//! The Android bench's report: `android.md` for people, `android.json`
//! with every measurement. Each number is the median of the runs, with
//! the smallest and the largest beside it, as on the desktop.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use super::apps::{self, App};
use super::device::{self, Emulator, Sdk};
use super::measure::{Measurement, QUIET_LOAD};
use super::{AndroidOptions, parse};
use crate::bench::report::{
    Better, CHECKED, DRAWING, Stat, Work, cold_of, string, timed, verdict_range_vs, verdict_vs,
    warm_of,
};

const RIVAL: &str = "J2ME Loader";

/// What a row of a table reads from a measurement.
type Value = dyn Fn(&Measurement) -> Option<f64>;

/// Everything the report shows.
pub struct Report {
    pub facts: Vec<(String, String)>,
    /// Each real game's slug and `MIDlet-Name`.
    pub games: Vec<(String, String)>,
    /// The suites J2ME Loader could not install, by name, and why.
    pub refused: Vec<(String, String)>,
    pub measurements: Vec<Measurement>,
    pub runs: usize,
}

/// The device, the versions and the settings.
pub fn facts(
    sdk: &Sdk,
    emulator: &Emulator,
    options: &AndroidOptions,
) -> Result<Vec<(String, String)>, String> {
    let device = &emulator.device;
    let prop = |name: &str| {
        device
            .shell_any(&format!("getprop {name}"))
            .trim()
            .to_owned()
    };
    let stat = device.shell_any("cat /proc/stat");
    let memory = parse::status_kib(&device.shell_any("cat /proc/meminfo"), "MemTotal").map_or_else(
        || "n/a".into(),
        |kib| format!("{:.1} GB", kib as f64 / 1024.0 / 1024.0),
    );
    let fastemu_sha = std::fs::read(&options.fastemu_apk)
        .map(|bytes| parse::sha256_hex(&bytes))
        .map_err(|e| format!("{}: {e}", options.fastemu_apk.display()))?;
    let mut flags = vec![
        "-avd".to_owned(),
        options.avd.clone(),
        "-port".to_owned(),
        device::PORT.to_string(),
    ];
    flags.extend(device::emulator_flags(&options.gpu));
    let mut facts = vec![
        (
            "Date".to_owned(),
            device
                .shell_any("date -u +'%Y-%m-%d %H:%M UTC'")
                .trim()
                .to_owned(),
        ),
        (
            "FastEmu".to_owned(),
            format!(
                "{} (APK SHA-256 {fastemu_sha}), the bench log on",
                apps::version(device, apps::FASTEMU)
            ),
        ),
        (
            RIVAL.to_owned(),
            format!(
                "{} (APK SHA-256 {})",
                apps::version(device, apps::J2ML),
                apps::J2ML_SHA256
            ),
        ),
        (
            "Both apps".to_owned(),
            "installed fresh with adb install -g, then compiled with \
             cmd package compile -m speed"
                .to_owned(),
        ),
        (
            "Emulator".to_owned(),
            format!("{}, {}", sdk.emulator_version(), flags.join(" ")),
        ),
        (
            "Android".to_owned(),
            format!(
                "{} (API {}), {}, image {}",
                prop("ro.build.version.release"),
                prop("ro.build.version.sdk"),
                prop("ro.product.cpu.abi"),
                prop("ro.build.fingerprint"),
            ),
        ),
        (
            "Guest".to_owned(),
            format!("{} processors, {memory}", parse::guest_cores(&stat)),
        ),
        (
            "Host".to_owned(),
            format!(
                "{}, {} logical processors",
                std::env::consts::OS,
                std::thread::available_parallelism().map_or(0, std::num::NonZero::get)
            ),
        ),
    ];
    for line in emulator.renderers() {
        facts.push(("Renderer".to_owned(), line));
    }
    Ok(facts)
}

fn of(ms: &[&Measurement], value: impl Fn(&Measurement) -> Option<f64>) -> Option<Stat> {
    let values: Vec<f64> = ms.iter().filter_map(|m| value(m)).collect();
    Stat::of(&values)
}

fn text(stat: Option<Stat>, decimals: usize) -> String {
    stat.map_or_else(|| "n/a".into(), |s| s.text(decimals))
}

fn mib(kib: Option<u64>) -> Option<f64> {
    kib.map(|k| k as f64 / 1024.0)
}

/// The measurements of `work` on `app`.
fn pick<'a>(report: &'a Report, work: &Work, app: App) -> Vec<&'a Measurement> {
    report
        .measurements
        .iter()
        .filter(|m| &m.work == work && m.app == app)
        .collect()
}

fn ratio(f: Option<Stat>, j: Option<Stat>, better: Better) -> String {
    match (f, j) {
        (Some(f), Some(j)) => verdict_vs(f.median, j.median, better, RIVAL),
        _ => "n/a".into(),
    }
}

fn row(out: &mut String, cells: &[String]) {
    let _ = writeln!(out, "| {} |", cells.join(" | "));
}

fn head(out: &mut String, cells: &[&str]) {
    let _ = writeln!(out, "| {} |", cells.join(" | "));
    let _ = writeln!(out, "|{}", " --- |".repeat(cells.len()));
}

/// The pairs of medians of each kernel in `names`, warm or cold.
fn kernel_pairs(report: &Report, names: &[&str], warm: bool) -> Vec<(f64, f64)> {
    let f = pick(report, &Work::Kernels, App::FastEmu);
    let j = pick(report, &Work::Kernels, App::J2meLoader);
    names
        .iter()
        .filter_map(|name| {
            let time = |m: &Measurement| {
                if warm {
                    warm_of(&m.rounds, name)
                } else {
                    cold_of(&m.rounds, name)
                }
            };
            Some((of(&f, time)?.median, of(&j, time)?.median))
        })
        .collect()
}

/// The pairs of each real game.
fn game_pairs(
    report: &Report,
    value: impl Fn(&Measurement) -> Option<f64> + Copy,
) -> Vec<(f64, f64)> {
    report
        .games
        .iter()
        .filter_map(|(slug, _)| {
            let work = Work::Game(slug.clone());
            Some((
                of(&pick(report, &work, App::FastEmu), value)?.median,
                of(&pick(report, &work, App::J2meLoader), value)?.median,
            ))
        })
        .collect()
}

pub fn markdown(report: &Report) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# Android bench: FastEmu and {RIVAL}\n");
    let _ = writeln!(
        out,
        "Each number is the median of {} runs, with the smallest and the largest in \
         brackets. Times are in milliseconds and memory in MiB. Both apps ran on one \
         Android emulator with no window: the comparison holds, but the numbers are not \
         a phone's.\n",
        report.runs
    );
    head(&mut out, &["What", "Value"]);
    for (key, value) in &report.facts {
        row(&mut out, &[key.clone(), value.replace('|', "/")]);
    }

    let scene_f = pick(report, &Work::Scene, App::FastEmu);
    let scene_j = pick(report, &Work::Scene, App::J2meLoader);
    out.push_str("\n## In short\n\n");
    let first = |ms: &[&Measurement]| of(ms, |m| m.first_ms);
    let _ = writeln!(
        out,
        "- **Start**, from the activity's start to the MIDlet's first paint (`BENCH \
         screen`): {}.",
        ratio(first(&scene_f), first(&scene_j), Better::Faster)
    );
    let _ = writeln!(
        out,
        "- **Real games, first frame on screen**: {}.",
        verdict_range_vs(&game_pairs(report, |m| m.first_ms), Better::Faster, RIVAL)
    );
    let _ = writeln!(
        out,
        "- **Real games, processor time a second**: {}.",
        verdict_range_vs(&game_pairs(report, |m| m.cpu_ms_per_s), Better::Less, RIVAL)
    );
    let _ = writeln!(
        out,
        "- **Real games, memory (PSS of the game's process)**: {}.",
        verdict_range_vs(&game_pairs(report, |m| mib(m.pss_kib)), Better::Less, RIVAL)
    );
    for (label, names, warm) in [
        ("CPU kernels, warm", &CHECKED[..9], true),
        ("CPU kernels, cold", &CHECKED[..9], false),
        ("Memory kernels, warm", &CHECKED[9..], true),
        ("Drawing kernels, warm", &DRAWING[..], true),
        ("Drawing kernels, cold", &DRAWING[..], false),
    ] {
        let _ = writeln!(
            out,
            "- **{label}**: {}.",
            verdict_range_vs(&kernel_pairs(report, names, warm), Better::Faster, RIVAL)
        );
    }

    kernels(&mut out, report);

    out.push_str("\n## The scene at 30 frames a second\n\n");
    head(&mut out, &["Measure", "FastEmu", RIVAL, "Ratio"]);
    let measures: [(&str, &Value, usize, Better); 5] = [
        (
            "Start to first paint, ms",
            &|m| m.first_ms,
            0,
            Better::Faster,
        ),
        ("Presents a second", &|m| m.fps, 1, Better::MoreFrames),
        ("CPU ms a second", &|m| m.cpu_ms_per_s, 1, Better::Less),
        ("PSS, MiB", &|m| mib(m.pss_kib), 1, Better::Less),
        ("Peak resident, MiB", &|m| mib(m.peak_kib), 1, Better::Less),
    ];
    for (label, value, decimals, better) in measures {
        let f = of(&scene_f, value);
        let j = of(&scene_j, value);
        row(
            &mut out,
            &[
                label.into(),
                text(f, decimals),
                text(j, decimals),
                ratio(f, j, better),
            ],
        );
    }
    let main = of(&scene_j, |m| mib(m.main_pss_kib));
    let _ = writeln!(
        out,
        "\n{RIVAL} also keeps its main process beside the game's: {} MiB PSS, not counted \
         above.",
        text(main, 1)
    );

    games(&mut out, report);

    let eight = pick(report, &Work::Eight, App::FastEmu);
    if !eight.is_empty() {
        out.push_str("\n## Every game at once, FastEmu only\n\n");
        let _ = writeln!(
            out,
            "{RIVAL} runs one game at a time, so only FastEmu runs this.\n"
        );
        head(&mut out, &["Measure", "FastEmu"]);
        row(
            &mut out,
            &[
                "Last first frame, ms".into(),
                text(of(&eight, |m| m.first_ms), 0),
            ],
        );
        row(
            &mut out,
            &[
                "CPU ms a second".into(),
                text(of(&eight, |m| m.cpu_ms_per_s), 1),
            ],
        );
        row(
            &mut out,
            &["PSS, MiB".into(), text(of(&eight, |m| mib(m.pss_kib)), 1)],
        );
        row(
            &mut out,
            &["Presents a second".into(), text(of(&eight, |m| m.fps), 1)],
        );
    }

    out.push_str("\n## What one side cannot measure\n\n");
    for line in [
        "- **Every game at once** on J2ME Loader: it runs one game at a time.",
        "- **The unpaced frame loop** (`frames.free`) as a speed: FastEmu caps a game at 60 \
         frames a second, and J2ME Loader turns no limit into 1000.",
        "- **Frames a game draws** on J2ME Loader: its counter is drawn on screen only. Both \
         apps' frames a second here are SurfaceFlinger's presents of the app.",
        "- **The drawing kernels' text**: J2ME Loader draws its medium font at 22 pixels, \
         FastEmu at 14.",
        "- **GPU time**: the emulator draws on the host's GPU, outside the guest's counters, \
         for both apps.",
    ] {
        let _ = writeln!(out, "{line}");
    }
    for (name, why) in &report.refused {
        let _ = writeln!(
            out,
            "- **{name}** on {RIVAL}: it could not install it, so only FastEmu ran it. \
             Its converter said: `{why}`"
        );
    }

    load(&mut out, report);
    problems(&mut out, report);
    out
}

fn kernels(out: &mut String, report: &Report) {
    let f = pick(report, &Work::Kernels, App::FastEmu);
    let j = pick(report, &Work::Kernels, App::J2meLoader);
    out.push_str("\n## Kernels\n\n");
    let _ = writeln!(
        out,
        "A kernel's warm time is the median time a repetition of its rounds after the \
         first took; its cold time is the first repetition of the first round. Ratios \
         are of the medians.\n"
    );
    head(
        out,
        &[
            "Kernel",
            "FastEmu warm",
            "J2ME Loader warm",
            "Warm ratio",
            "FastEmu cold",
            "J2ME Loader cold",
            "Cold ratio",
            "Same checks",
        ],
    );
    let step = f.iter().find_map(|m| m.clock_step);
    for name in CHECKED.iter().chain(DRAWING.iter()) {
        let fw = of(&f, |m| warm_of(&m.rounds, name));
        let jw = of(&j, |m| warm_of(&m.rounds, name));
        let fc = of(&f, |m| cold_of(&m.rounds, name));
        let jc = of(&j, |m| cold_of(&m.rounds, name));
        let checks = |ms: &[&Measurement]| -> BTreeSet<i64> {
            ms.iter()
                .flat_map(|m| m.rounds.iter())
                .filter(|r| r.name == *name)
                .map(|r| r.check)
                .collect()
        };
        let same = if !CHECKED.contains(name) || checks(&f).is_empty() || checks(&j).is_empty() {
            "n/a".to_owned()
        } else if checks(&f) == checks(&j) {
            "yes".to_owned()
        } else {
            "NO".to_owned()
        };
        let cell = |s: Option<Stat>| s.map_or_else(|| "n/a".into(), |s| timed(s.median, step, 2));
        row(
            out,
            &[
                format!("`{name}`"),
                cell(fw),
                cell(jw),
                ratio(fw, jw, Better::Faster),
                cell(fc),
                cell(jc),
                ratio(fc, jc, Better::Faster),
                same,
            ],
        );
    }
    let _ = writeln!(
        out,
        "\nWhole kernel run: FastEmu {} ms of wall time and {} ms of processor time; \
         {RIVAL} {} and {}.",
        text(of(&f, |m| m.wall_ms), 0),
        text(of(&f, |m| m.cpu_ms), 0),
        text(of(&j, |m| m.wall_ms), 0),
        text(of(&j, |m| m.cpu_ms), 0),
    );
}

fn games(out: &mut String, report: &Report) {
    if report.games.is_empty() {
        return;
    }
    out.push_str("\n## The real games\n\n");
    let _ = writeln!(
        out,
        "The first frame is the first present of the game's surface on screen; for \
         FastEmu, the first after its bench log's first frame, since its surface shows \
         its library before. Presents are the app's frames on screen, as SurfaceFlinger \
         counts them; FastEmu's bench log also counts the frames the game drew, which \
         J2ME Loader cannot tell. The bench pressed the same keys on both from each \
         game's first frame.\n"
    );
    head(
        out,
        &[
            "Game",
            "App",
            "First frame, ms",
            "Presents a second",
            "Game frames a second, its log",
            "CPU ms a second",
            "PSS, MiB",
            "Ended",
        ],
    );
    for (slug, name) in &report.games {
        let work = Work::Game(slug.clone());
        for app in [App::FastEmu, App::J2meLoader] {
            let ms = pick(report, &work, app);
            let ends: BTreeSet<String> = ms
                .iter()
                .filter_map(|m| m.ends.first().cloned().flatten())
                .map(|e| e.status)
                .collect();
            row(
                out,
                &[
                    name.clone(),
                    app.label().into(),
                    text(of(&ms, |m| m.first_ms), 0),
                    text(of(&ms, |m| m.fps), 1),
                    text(of(&ms, |m| m.own_fps.first().copied().flatten()), 1),
                    text(of(&ms, |m| m.cpu_ms_per_s), 1),
                    text(of(&ms, |m| mib(m.pss_kib)), 1),
                    ends.into_iter().collect::<Vec<_>>().join(", "),
                ],
            );
        }
    }
}

fn load(out: &mut String, report: &Report) {
    let all: Vec<&Measurement> = report.measurements.iter().collect();
    let _ = writeln!(
        out,
        "\n## Load\n\nBefore each start the bench waited for the guest's load to fall under \
         {QUIET_LOAD:.0} busy ms a second, for up to a minute: it was {} before the starts, \
         and the rest of the guest took {} while the samples ran.",
        text(of(&all, |m| m.quiet), 0),
        text(of(&all, |m| m.other), 0),
    );
}

fn problems(out: &mut String, report: &Report) {
    let listed: Vec<String> = report
        .measurements
        .iter()
        .flat_map(|m| {
            m.problems.iter().map(move |p| {
                format!(
                    "- {} on {}, run {}: {p}",
                    m.work.key(),
                    m.app.label(),
                    m.run + 1
                )
            })
        })
        .chain(report.measurements.iter().flat_map(|m| {
            m.kernel_errors.iter().map(move |(name, what)| {
                format!(
                    "- `{name}` threw on {}, run {}: {what}",
                    m.app.label(),
                    m.run + 1
                )
            })
        }))
        .collect();
    if !listed.is_empty() {
        out.push_str("\n## Problems\n\n");
        for line in listed {
            let _ = writeln!(out, "{line}");
        }
    }
}

fn number(value: Option<f64>) -> String {
    value.map_or_else(|| "null".into(), |v| format!("{v:.3}"))
}

pub fn json(report: &Report) -> String {
    let mut out = String::from("{\n  \"facts\": {");
    let facts: Vec<String> = report
        .facts
        .iter()
        .map(|(k, v)| format!("\n    {}: {}", string(k), string(v)))
        .collect();
    out.push_str(&facts.join(","));
    out.push_str("\n  },\n  \"measurements\": [");
    let ms: Vec<String> = report
        .measurements
        .iter()
        .map(|m| {
            let rounds: Vec<String> = m
                .rounds
                .iter()
                .map(|r| {
                    format!(
                        "{{\"name\": {}, \"round\": {}, \"reps\": {}, \"ms\": {}, \"first\": {}, \"check\": {}}}",
                        string(&r.name),
                        r.round,
                        r.reps,
                        r.ms,
                        r.first,
                        r.check
                    )
                })
                .collect();
            let problems: Vec<String> = m.problems.iter().map(|p| string(p)).collect();
            format!(
                "\n    {{\"work\": {}, \"app\": {}, \"run\": {}, \"first_app\": {}, \
                 \"first_ms\": {}, \"first_by\": {}, \"fps\": {}, \"cpu_ms_per_s\": {}, \
                 \"cpu_ms\": {}, \"wall_ms\": {}, \"pss_kib\": {}, \"peak_kib\": {}, \
                 \"main_pss_kib\": {}, \"quiet\": {}, \"other\": {}, \"frames_free\": {}, \
                 \"rounds\": [{}], \"problems\": [{}]}}",
                string(&m.work.key()),
                string(m.app.key()),
                m.run,
                string(m.first_app.key()),
                number(m.first_ms),
                string(m.first_by),
                number(m.fps),
                number(m.cpu_ms_per_s),
                number(m.cpu_ms),
                number(m.wall_ms),
                number(m.pss_kib.map(|v| v as f64)),
                number(m.peak_kib.map(|v| v as f64)),
                number(m.main_pss_kib.map(|v| v as f64)),
                number(m.quiet),
                number(m.other),
                number(m.frames_free),
                rounds.join(", "),
                problems.join(", "),
            )
        })
        .collect();
    out.push_str(&ms.join(","));
    out.push_str("\n  ]\n}\n");
    out
}
