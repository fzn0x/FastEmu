//! The manifest and JAD corpus (`docs/compat/notes/01-suite-loading.md`):
//! a byte-order mark, CR LF, LF and CR line ends, values wrapped in the
//! middle of a class name, a lower-case `meta-inf`, `MANIFEST.FM`, a
//! missing comma, class names written as paths, repeated keys, stray
//! spaces, no `MIDlet-<n>` at all, and JADs found and followed every way
//! the note lists. Each suite is inspected and its MIDlet launched to
//! print what `getAppProperty` gives.

use std::path::{Path, PathBuf};

use fastemu_core::Launcher;
use fastemu_midp::MidpLauncher;

use crate::conformance_support::{Jar, assert_lines, compile, scratch};
use crate::support::Running;

/// The attributes every corpus manifest starts with.
const BASE: [&str; 4] = [
    "Manifest-Version: 1.0",
    "MIDlet-Name: Corpus",
    "MIDlet-Vendor: FastEmu",
    "MIDlet-Version: 1.0",
];

/// The `MIDlet-1` the cases expect to find.
struct Expected {
    name: &'static str,
    icon: Option<&'static str>,
}

/// One manifest of the corpus.
struct Case {
    what: &'static str,
    /// The manifest's entry name.
    entry: &'static str,
    bom: bool,
    eol: &'static str,
    /// The lines after [`BASE`].
    lines: &'static [&'static str],
    midlet: Expected,
    custom: &'static str,
    wrapped: &'static str,
}

const PLAIN: &[&str] = &["MIDlet-1: Props, /icons/p.png, m.Props", "Custom: plain"];

const ICON: Expected = Expected {
    name: "Props",
    icon: Some("icons/p.png"),
};

const NO_ICON: Expected = Expected {
    name: "Props",
    icon: None,
};

fn case(what: &'static str, lines: &'static [&'static str], midlet: Expected) -> Case {
    Case {
        what,
        entry: "META-INF/MANIFEST.MF",
        bom: false,
        eol: "\r\n",
        lines,
        midlet,
        custom: "plain",
        wrapped: "null",
    }
}

fn corpus() -> Vec<Case> {
    vec![
        case("CR LF", PLAIN, ICON),
        Case {
            bom: true,
            ..case("a byte-order mark", PLAIN, ICON)
        },
        Case {
            eol: "\n",
            ..case("LF", PLAIN, ICON)
        },
        Case {
            eol: "\r",
            ..case("CR", PLAIN, ICON)
        },
        Case {
            custom: "wrapped: yes",
            wrapped: "http://example.com/a/b:c",
            ..case(
                "values wrapped at any byte",
                &[
                    "MIDlet-1: Props, , m.Pr",
                    " ops",
                    "Custom: wr",
                    " apped: yes",
                    "Wrapped: http://example.com/a",
                    " /b:c",
                ],
                NO_ICON,
            )
        },
        Case {
            entry: "meta-inf/manifest.mf",
            ..case("a lower-case manifest path", PLAIN, ICON)
        },
        Case {
            entry: "META-INF/MANIFEST.FM",
            ..case("MANIFEST.FM", PLAIN, ICON)
        },
        case(
            "a missing comma",
            &["MIDlet-1: Props/icons/p.png, m.Props", "Custom: plain"],
            Expected {
                name: "Props/icons/p.png",
                icon: None,
            },
        ),
        case(
            "a class written as a path",
            &["MIDlet-1: Props, , /m/Props.class", "Custom: plain"],
            NO_ICON,
        ),
        case(
            "a repeated key",
            &[
                "MIDlet-1: Props, , m.Props",
                "Custom: plain",
                "Custom: second",
            ],
            NO_ICON,
        ),
        case(
            "spaces and blank lines",
            &[
                "",
                "MIDlet-1 :  Props ,  , m.Props  ",
                "",
                "Custom :   plain   ",
            ],
            NO_ICON,
        ),
        case(
            "a gap after MIDlet-1",
            &[
                "MIDlet-1: Props, , m.Props",
                "MIDlet-3: Other, , m.Other",
                "Custom: plain",
            ],
            NO_ICON,
        ),
        case(
            "no MIDlet-<n>",
            &["Custom: plain"],
            Expected {
                name: "Corpus",
                icon: None,
            },
        ),
    ]
}

fn manifest(case: &Case) -> Vec<u8> {
    let mut text = String::new();
    if case.bom {
        text.push('\u{feff}');
    }
    for line in BASE.iter().chain(case.lines) {
        text.push_str(line);
        text.push_str(case.eol);
    }
    text.into_bytes()
}

/// The `prop` lines `m.Props` prints for the attribute values given.
fn props(custom: &str, wrapped: &str, jad_only: &str, manifest_only: &str) -> Vec<String> {
    [
        ("MIDlet-Name", "Corpus"),
        ("MIDlet-Vendor", "FastEmu"),
        ("Custom", custom),
        ("Wrapped", wrapped),
        ("Jad-Only", jad_only),
        ("Manifest-Only", manifest_only),
    ]
    .iter()
    .map(|(k, v)| format!("prop {k} = {v}"))
    .collect()
}

/// Launches the suite and checks the properties its MIDlet prints.
fn check_props(suite: &Path, want: &[String]) {
    let mut run = Running::launch(suite, 0);
    run.wait_log("props done");
    let log = run.lines();
    let got: Vec<&str> = log
        .iter()
        .map(String::as_str)
        .filter(|l| l.starts_with("prop "))
        .collect();
    let want: Vec<&str> = want.iter().map(String::as_str).collect();
    assert_lines(&got, &want, &log);
}

#[test]
fn every_manifest_of_the_corpus_opens_and_runs() {
    let Some(classes) = compile("suites") else {
        return;
    };
    let dir = scratch("manifests");
    for (i, case) in corpus().iter().enumerate() {
        let jar = Jar::new()
            .stored(case.entry, manifest(case))
            .classes(&classes)
            .write(&dir.join(format!("case{i}.jar")));
        let info = MidpLauncher::new(None)
            .inspect(&jar)
            .unwrap_or_else(|e| panic!("{}: {e}", case.what));
        assert_eq!(info.name, "Corpus", "{}", case.what);
        assert_eq!(info.vendor, "FastEmu", "{}", case.what);
        assert_eq!(info.midlets.len(), 1, "{}: {:?}", case.what, info.midlets);
        let midlet = &info.midlets[0];
        assert_eq!(midlet.class_name, "m.Props", "{}", case.what);
        assert_eq!(midlet.name, case.midlet.name, "{}", case.what);
        assert_eq!(
            midlet.icon_path.as_deref(),
            case.midlet.icon,
            "{}",
            case.what
        );
        check_props(&jar, &props(case.custom, case.wrapped, "null", "null"));
    }
}

/// A JAR whose manifest sets `Custom` and `Manifest-Only`.
fn jad_jar(classes: &Path, path: &Path) -> PathBuf {
    std::fs::create_dir_all(path.parent().expect("a folder")).expect("making the folder");
    let mut text = BASE.join("\r\n");
    text.push_str(
        "\r\nMIDlet-1: Props, , m.Props\r\nCustom: from the manifest\r\nManifest-Only: yes\r\n",
    );
    Jar::new().manifest(text).classes(classes).write(path)
}

/// A JAD whose `MIDlet-Jar-URL` is `url` and that sets `Custom` and
/// `Jad-Only`.
fn jad(path: &Path, url: &str) -> PathBuf {
    let text = format!(
        "MIDlet-Name: Corpus\nMIDlet-Vendor: FastEmu\nMIDlet-Version: 1.0\n\
         MIDlet-Jar-URL: {url}\nMIDlet-Jar-Size: 1\nMIDlet-1: Props, , m.Props\n\
         Custom: from the JAD\nJad-Only: yes\n"
    );
    std::fs::write(path, text).expect("writing the JAD");
    path.to_path_buf()
}

#[test]
fn jads_are_found_and_followed_and_win_over_the_manifest() {
    let Some(classes) = compile("suites") else {
        return;
    };
    let dir = scratch("jads");
    let both = props("from the JAD", "null", "yes", "yes");
    // A JAR picks up the JAD of the same name beside it.
    let jar = jad_jar(&classes, &dir.join("pair/corpus.jar"));
    jad(&dir.join("pair/corpus.jad"), "corpus.jar");
    check_props(&jar, &both);
    // A JAD finds its JAR by a relative URL, by the URL's last segment,
    // and by its own name.
    let cases = [
        ("relative", "sub/inner.jar", "sub/inner.jar"),
        (
            "last-segment",
            "http://example.com/games/real.jar",
            "real.jar",
        ),
        ("own-name", "elsewhere/missing.jar", "game.jar"),
    ];
    for (folder, url, jar) in cases {
        let folder = dir.join(folder);
        jad_jar(&classes, &folder.join(jar));
        let jad = jad(&folder.join("game.jad"), url);
        let info = MidpLauncher::new(None)
            .inspect(&jad)
            .unwrap_or_else(|e| panic!("{url}: {e}"));
        assert_eq!(info.property("Custom"), Some("from the JAD"), "{url}");
        check_props(&jad, &both);
    }
}
