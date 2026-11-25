//! Builds what a bench runs: the benchmark MIDlet, packed three ways, and
//! MicroEmulator's wrapper, MeBench. The Java sources are part of this
//! program, so a release build of the xtask runs anywhere a JDK is.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::api::stubs::{self, javac_options, jdk_tool};

/// The benchmark MIDlet's sources, by their path under the source folder.
const MIDLET: [(&str, &str); 2] = [
    (
        "fastemu/bench/Bench.java",
        include_str!("../../bench/midlet/fastemu/bench/Bench.java"),
    ),
    (
        "fastemu/bench/Kernels.java",
        include_str!("../../bench/midlet/fastemu/bench/Kernels.java"),
    ),
];

/// MicroEmulator's wrapper.
const MEBENCH: (&str, &str) = (
    "fastemu/bench/MeBench.java",
    include_str!("../../bench/microemu/fastemu/bench/MeBench.java"),
);

/// A stand-in of `tools/xtask/bench/microemu-api`, by its path there.
macro_rules! api_file {
    ($path:literal) => {
        (
            $path,
            include_str!(concat!("../../bench/microemu-api/", $path)),
        )
    };
}

/// Signature-only stand-ins for the MicroEmulator classes MeBench names,
/// so it compiles where MicroEmulator is not installed.
const MICROEMU_API: [(&str, &str); 26] = [
    api_file!("javax/microedition/lcdui/Displayable.java"),
    api_file!("javax/microedition/lcdui/Image.java"),
    api_file!("javax/microedition/midlet/MIDlet.java"),
    api_file!("org/microemu/DisplayAccess.java"),
    api_file!("org/microemu/DisplayComponent.java"),
    api_file!("org/microemu/EmulatorContext.java"),
    api_file!("org/microemu/MIDletAccess.java"),
    api_file!("org/microemu/MIDletBridge.java"),
    api_file!("org/microemu/app/Common.java"),
    api_file!("org/microemu/app/launcher/Launcher.java"),
    api_file!("org/microemu/app/ui/DisplayRepaintListener.java"),
    api_file!("org/microemu/app/ui/noui/NoUiDisplayComponent.java"),
    api_file!("org/microemu/app/util/DeviceEntry.java"),
    api_file!("org/microemu/device/DeviceDisplay.java"),
    api_file!("org/microemu/device/FontManager.java"),
    api_file!("org/microemu/device/InputMethod.java"),
    api_file!("org/microemu/device/MutableImage.java"),
    api_file!("org/microemu/device/impl/DeviceImpl.java"),
    api_file!("org/microemu/device/j2se/J2SEDevice.java"),
    api_file!("org/microemu/device/j2se/J2SEDeviceDisplay.java"),
    api_file!("org/microemu/device/j2se/J2SEFontManager.java"),
    api_file!("org/microemu/device/j2se/J2SEInputMethod.java"),
    api_file!("org/microemu/device/ui/EventDispatcher.java"),
    api_file!("org/microemu/log/Logger.java"),
    api_file!("org/microemu/log/LoggerAppender.java"),
    api_file!("org/microemu/log/LoggingEvent.java"),
];

/// The jars of the MicroEmulator release the bench puts on the class
/// path, under its folder: the emulator, and its sound, Nokia and Siemens
/// APIs. Never `lib/cldcapi*.jar` or `lib/midpapi20.jar`: those are only
/// for compiling MIDlets.
pub const MICROEMU_JARS: [&str; 4] = [
    "microemulator.jar",
    "lib/microemu-jsr-135.jar",
    "lib/microemu-nokiaui.jar",
    "lib/microemu-siemensapi.jar",
];

/// The three packings of the MIDlet: the kernels at full size, the paced
/// scene, and the kernels at the smallest size, for tests.
const JARS: [(&str, &[(&str, &str)]); 3] = [
    (
        "bench.jar",
        &[
            ("Bench-Mode", "kernels"),
            ("Bench-Scale", "100"),
            ("Bench-Rounds", "5"),
            ("Bench-Round-Ms", "500"),
            ("Bench-Linger-Ms", "3000"),
        ],
    ),
    ("scene.jar", &[("Bench-Mode", "scene")]),
    (
        "check.jar",
        &[
            ("Bench-Mode", "kernels"),
            ("Bench-Scale", "1"),
            ("Bench-Rounds", "2"),
            ("Bench-Round-Ms", "0"),
            ("Bench-Linger-Ms", "0"),
        ],
    ),
];

/// What [`build`] made.
#[derive(Clone, Debug)]
pub struct Built {
    pub bench_jar: PathBuf,
    pub scene_jar: PathBuf,
    pub check_jar: PathBuf,
    /// The folder of MeBench's classes.
    pub mebench: PathBuf,
    /// Whether MeBench was compiled against the real microemulator.jar,
    /// not the stand-ins.
    pub mebench_real: bool,
}

fn io(path: &Path, e: std::io::Error) -> String {
    format!("{}: {e}", path.display())
}

/// Writes `files` under `dir`, after clearing it.
fn write_sources(dir: &Path, files: &[(&str, &str)]) -> Result<Vec<PathBuf>, String> {
    let _ = fs::remove_dir_all(dir);
    let mut paths = Vec::new();
    for (name, text) in files {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| io(parent, e))?;
        }
        fs::write(&path, text).map_err(|e| io(&path, e))?;
        paths.push(path);
    }
    Ok(paths)
}

fn fresh(dir: &Path) -> Result<(), String> {
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).map_err(|e| io(dir, e))
}

/// The manifest of one packing.
fn manifest(properties: &[(&str, &str)]) -> String {
    let mut text = String::from(
        "Manifest-Version: 1.0\nMIDlet-Name: FastEmu Bench\nMIDlet-Vendor: FastEmu\n\
         MIDlet-Version: 1.0\nMIDlet-1: FastEmu Bench, , fastemu.bench.Bench\n\
         MicroEdition-Configuration: CLDC-1.1\nMicroEdition-Profile: MIDP-2.0\n",
    );
    for (key, value) in properties {
        let _ = writeln!(text, "{key}: {value}");
    }
    text
}

/// Builds the stubs, the MIDlet's three jars and MeBench under `out`.
/// MeBench compiles against `microemu`'s microemulator.jar when given,
/// into `out/mebench-real`, else against the stand-ins, into
/// `out/mebench`. `root` is the repository, for `docs/compat/api`.
pub fn build(out: &Path, root: &Path, microemu: Option<&Path>) -> Result<Built, String> {
    let javac = jdk_tool("javac").ok_or("no javac in JAVA_HOME or on the PATH")?;
    let jar_tool = jdk_tool("jar").ok_or("no jar tool in JAVA_HOME or on the PATH")?;
    fs::create_dir_all(out).map_err(|e| io(out, e))?;

    // The MIDlet, against exactly the classes FastEmu has.
    let stubs = stubs::build(&out.join("stubs"), &root.join("docs/compat/api"))?;
    let midlet = out.join("midlet");
    let sources = write_sources(&midlet.join("src"), &MIDLET)?;
    let classes = midlet.join("classes");
    fresh(&classes)?;
    let mut command = Command::new(&javac);
    command
        .args(javac_options(&stubs.jar))
        .arg("-d")
        .arg(&classes)
        .args(&sources);
    stubs::run(&mut command, "javac on the benchmark MIDlet")?;
    for (name, properties) in JARS {
        let mf = midlet.join(format!("{name}.mf"));
        fs::write(&mf, manifest(properties)).map_err(|e| io(&mf, e))?;
        let jar = out.join(name);
        let _ = fs::remove_file(&jar);
        let mut command = Command::new(&jar_tool);
        command
            .arg("cfm")
            .arg(&jar)
            .arg(&mf)
            .arg("-C")
            .arg(&classes)
            .arg(".");
        stubs::run(&mut command, &format!("jar for {name}"))?;
    }

    // MeBench, a plain Java 8 program beside MicroEmulator.
    let me_src = write_sources(&out.join("mebench-src"), &[MEBENCH])?;
    let mut command = Command::new(&javac);
    command.args([
        "--release",
        "8",
        "-g",
        "-encoding",
        "UTF-8",
        "-nowarn",
        "-implicit:none",
    ]);
    let (mebench, real) = if let Some(dir) = microemu {
        let jar = dir.join("microemulator.jar");
        if !jar.is_file() {
            return Err(format!(
                "{} is missing: --microemu names the unpacked microemulator-2.0.4 folder",
                jar.display()
            ));
        }
        command.arg("-cp").arg(jar);
        (out.join("mebench-real"), true)
    } else {
        let api = out.join("microemu-api");
        write_sources(&api, &MICROEMU_API)?;
        command.arg("-sourcepath").arg(api);
        (out.join("mebench"), false)
    };
    fresh(&mebench)?;
    command.arg("-d").arg(&mebench).args(&me_src);
    stubs::run(&mut command, "javac on MeBench")?;
    Ok(Built {
        bench_jar: out.join("bench.jar"),
        scene_jar: out.join("scene.jar"),
        check_jar: out.join("check.jar"),
        mebench,
        mebench_real: real,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifests_name_the_midlet_and_its_settings() {
        let text = manifest(JARS[2].1);
        assert!(text.contains("MIDlet-1: FastEmu Bench, , fastemu.bench.Bench\n"));
        assert!(text.contains("Bench-Scale: 1\n"));
        assert!(text.ends_with('\n'));
        // Every stand-in says it holds no MicroEmulator code.
        for (name, text) in MICROEMU_API {
            assert!(text.contains("It holds no MicroEmulator code"), "{name}");
        }
    }
}
