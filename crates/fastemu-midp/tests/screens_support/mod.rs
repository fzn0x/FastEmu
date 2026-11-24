//! Builds the screen tests' suite: the MIDlets in
//! `tests/screens_java/midlets` compiled with `javac --release 8` against
//! the stubs in `tests/screens_java/stubs`, packed with a manifest into
//! a JAR that `crate::support::Running` launches. Only the MIDlets' own
//! classes go into the JAR, so the runtime serves the stubs' names.
//! Without a JDK the tests print why and pass, unless
//! `FASTEMU_REQUIRE_JAVA=1` is set.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::support::{scratch, zip};

/// The manifest: one MIDlet per test program.
const MANIFEST: &str = "Manifest-Version: 1.0\r\n\
MIDlet-Name: Screens\r\n\
MIDlet-Vendor: FastEmu\r\n\
MIDlet-Version: 1.0\r\n\
MIDlet-1: Lists, , s.Lists\r\n\
MIDlet-2: Texts, , s.Texts\r\n\
MIDlet-3: Alerts, , s.Alerts\r\n\
MIDlet-4: Forms, , s.Forms\r\n\
MIDlet-5: Customs, , s.Customs\r\n\
MIDlet-6: Edges, , s.Edges\r\n";

/// Which `MIDlet-<n>` (from zero) runs each program.
pub mod midlet {
    pub const LISTS: usize = 0;
    pub const TEXTS: usize = 1;
    pub const ALERTS: usize = 2;
    pub const FORMS: usize = 3;
    pub const CUSTOMS: usize = 4;
    pub const EDGES: usize = 5;
}

/// The JDK tool `name`, from `JAVA_HOME` first, then the `PATH`.
fn tool(name: &str) -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    };
    if let Some(home) = std::env::var_os("JAVA_HOME") {
        let candidate = Path::new(&home).join("bin").join(&exe);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    let works = Command::new(name)
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success());
    works.then(|| PathBuf::from(name))
}

fn java_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("reading a test source folder") {
        let path = entry.expect("a folder entry").path();
        if path.is_dir() {
            java_files(&path, out);
        } else if path.extension().is_some_and(|x| x == "java") {
            out.push(path);
        }
    }
}

fn javac(javac: &Path, out: &Path, classpath: Option<&Path>, dir: &Path) {
    let mut sources = Vec::new();
    java_files(dir, &mut sources);
    sources.sort();
    std::fs::create_dir_all(out).expect("making a class folder");
    let mut command = Command::new(javac);
    command.args([
        "--release",
        "8",
        "-g",
        "-encoding",
        "UTF-8",
        "-nowarn",
        "-d",
    ]);
    command.arg(out);
    if let Some(classpath) = classpath {
        command.arg("-cp").arg(classpath);
    }
    let output = command.args(&sources).output().expect("running javac");
    assert!(
        output.status.success(),
        "javac failed:\n{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Compiles the stubs and the MIDlets once per test binary; `Err` says
/// why there is no JDK to do it.
fn compiled() -> &'static Result<PathBuf, String> {
    static CLASSES: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    CLASSES.get_or_init(|| {
        let javac_path = tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
        let java = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("screens_java");
        let binary = std::env::current_exe()
            .ok()
            .and_then(|exe| exe.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .unwrap_or_else(|| "screens".to_owned());
        let root = scratch(&format!("screens-java-{binary}"));
        let stubs = root.join("stubs");
        let classes = root.join("classes");
        javac(&javac_path, &stubs, None, &java.join("stubs"));
        javac(&javac_path, &classes, Some(&stubs), &java.join("midlets"));
        Ok(classes)
    })
}

/// The suite's JAR in a fresh folder named `name`, or `None` (after
/// saying why) when no JDK is installed and Java is not required.
pub fn suite(name: &str) -> Option<PathBuf> {
    let classes = match compiled() {
        Ok(classes) => classes,
        Err(why) => {
            assert!(
                std::env::var("FASTEMU_REQUIRE_JAVA").as_deref() != Ok("1"),
                "Java is required but unusable: {why}"
            );
            eprintln!("skipping: {why}");
            return None;
        }
    };
    let mut entries = vec![(
        "META-INF/MANIFEST.MF".to_owned(),
        MANIFEST.as_bytes().to_vec(),
    )];
    let mut files: Vec<PathBuf> = std::fs::read_dir(classes.join("s"))
        .expect("the compiled test classes")
        .map(|e| e.expect("an entry").path())
        .collect();
    files.sort();
    for file in files {
        let name = format!("s/{}", file.file_name().unwrap().to_string_lossy());
        entries.push((name, std::fs::read(&file).expect("a class file")));
    }
    let dir = scratch(name);
    let path = dir.join("screens.jar");
    std::fs::write(&path, zip(&entries, true)).expect("writing the JAR");
    Some(path)
}
