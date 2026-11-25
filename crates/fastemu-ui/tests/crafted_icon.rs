//! A suite whose icon is a crafted PNG must not stop the library scan.
//!
//! The scan runs on a background task. In the app a panic that leaves such
//! a task aborts the whole process; the test platform runs tasks where
//! they are called, so the same panic fails the test.

use std::path::Path;
use std::sync::Arc;

use fastemu_core::{Instance, LaunchError, LaunchRequest, Launcher, SuiteInfo};
use fastemu_ui::{AppConfig, AppPaths, FastEmuView, init};
use gpui::TestAppContext;

/// An indexed 1 x 1 PNG with a palette of 4 bytes, with valid checksums.
/// The `png` crate panics decoding it ("range end index 3 out of range").
fn odd_palette_png() -> Vec<u8> {
    let mut out = Vec::new();
    let mut encoder = png::Encoder::new(&mut out, 1, 1);
    encoder.set_color(png::ColorType::Indexed);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_palette(vec![1, 2, 3, 4]);
    let mut writer = encoder.write_header().expect("writes");
    writer.write_image_data(&[0]).expect("writes");
    writer.finish().expect("writes");
    out
}

/// Reads every suite as one with the crafted icon.
struct CraftedIcon;

impl Launcher for CraftedIcon {
    fn inspect(&self, _suite: &Path) -> Result<SuiteInfo, LaunchError> {
        Ok(SuiteInfo {
            name: "Crafted".into(),
            icon: Some(odd_palette_png()),
            ..SuiteInfo::default()
        })
    }

    fn launch(&self, _request: LaunchRequest) -> Result<Arc<dyn Instance>, LaunchError> {
        Err(LaunchError::Unsupported("not needed".into()))
    }
}

#[gpui::test]
fn a_crafted_icon_does_not_stop_the_library_scan(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("temp dir");
    let config_dir = dir.path().join("config");
    std::fs::create_dir_all(&config_dir).expect("config dir");
    // The scan at every start reads each entry's suite again.
    std::fs::write(
        config_dir.join("library.json"),
        r#"{"entries":[{"id":1,"path":"nowhere/crafted.jar","name":"Crafted"}],"next_id":2}"#,
    )
    .expect("library");
    let config = AppConfig {
        launcher: Arc::new(CraftedIcon),
        paths: AppPaths {
            config_dir,
            data_dir: dir.path().join("data"),
            screenshot_dir: dir.path().join("shots"),
        },
        text_entry: None,
    };
    cx.update(init);
    let (_view, cx) = cx.add_window_view(|window, cx| FastEmuView::new(config, window, cx));
    cx.run_until_parked();
}
