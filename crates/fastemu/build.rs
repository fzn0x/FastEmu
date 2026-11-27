//! Gives `fastemu.exe` its icon and version information on Windows.
//!
//! The icon is `packaging/icons/fastemu.ico`, which `cargo xtask icons`
//! renders from `assets/icon.svg`. GPUI loads the icon resource numbered 1
//! for its windows, so the same icon shows in Explorer, on the title bar and
//! on the taskbar. The version information, which Explorer's Properties
//! show, comes from Cargo.toml. Other systems need neither: Linux and macOS
//! take their icons from the packages the release workflow builds.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows::embed_resources();
    }
}

#[cfg(windows)]
mod windows {
    use std::env;
    use std::fs;
    use std::path::PathBuf;

    pub fn embed_resources() {
        let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("set by Cargo"));
        let icon = manifest_dir.join("../../packaging/icons/fastemu.ico");
        println!("cargo:rerun-if-changed={}", icon.display());
        let icon = icon
            .canonicalize()
            .unwrap_or_else(|e| panic!("{}: {e}", icon.display()));

        let version = env::var("CARGO_PKG_VERSION").expect("set by Cargo");
        let number = |name: &str| -> u16 {
            env::var(name)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0)
        };
        let numeric = format!(
            "{},{},{},0",
            number("CARGO_PKG_VERSION_MAJOR"),
            number("CARGO_PKG_VERSION_MINOR"),
            number("CARGO_PKG_VERSION_PATCH"),
        );
        let rc = format!(
            r#"#include <winver.h>

1 ICON "{icon}"

1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEFLAGSMASK VS_FFI_FILEFLAGSMASK
FILEFLAGS 0
FILEOS VOS_NT_WINDOWS32
FILETYPE VFT_APP
FILESUBTYPE VFT2_UNKNOWN
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904B0"
        BEGIN
            VALUE "CompanyName", "The FastEmu contributors"
            VALUE "FileDescription", "FastEmu"
            VALUE "FileVersion", "{version}"
            VALUE "InternalName", "fastemu"
            VALUE "LegalCopyright", "The FastEmu contributors, GPL-3.0-or-later"
            VALUE "OriginalFilename", "fastemu.exe"
            VALUE "ProductName", "FastEmu"
            VALUE "ProductVersion", "{version}"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x409, 1200
    END
END
"#,
            // The resource compiler reads backslashes in strings as escapes.
            icon = rc_path(&icon),
        );
        let out_dir = PathBuf::from(env::var_os("OUT_DIR").expect("set by Cargo"));
        // fastemu-console.exe ships as fastemu.com, the console program
        // beside fastemu.exe: the same icon and version, its own names.
        let console_rc = rc
            .replace(
                r#""FileDescription", "FastEmu""#,
                r#""FileDescription", "FastEmu command line""#,
            )
            .replace(
                r#""InternalName", "fastemu""#,
                r#""InternalName", "fastemu-console""#,
            )
            .replace(
                r#""OriginalFilename", "fastemu.exe""#,
                r#""OriginalFilename", "fastemu.com""#,
            );
        for (bin, script) in [("fastemu", rc), ("fastemu-console", console_rc)] {
            let out = out_dir.join(format!("{bin}.rc"));
            fs::write(&out, script).unwrap_or_else(|e| panic!("{}: {e}", out.display()));
            embed_resource::compile_for(&out, [bin], embed_resource::NONE)
                .manifest_required()
                .unwrap_or_else(|e| {
                    panic!("{e}: install the Windows SDK, whose rc.exe builds the icon resource")
                });
        }
    }

    /// A path as a resource script string holds it: forward slashes, without
    /// the `\\?\` prefix `canonicalize` adds.
    fn rc_path(path: &std::path::Path) -> String {
        let text = path.display().to_string();
        let text = text.strip_prefix(r"\\?\").unwrap_or(&text);
        text.replace('\\', "/")
    }
}

#[cfg(not(windows))]
mod windows {
    /// Building for Windows from another system: there is no resource
    /// compiler to call, so the program has no icon.
    pub fn embed_resources() {
        println!(
            "cargo:warning=fastemu.exe is built without its icon: build on Windows to embed it"
        );
    }
}
