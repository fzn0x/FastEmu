//! A MIDlet probing for the vendor APIs with `Class.forName` finds only
//! the classes of the maker its device profile imitates.

mod support;
mod vendor_support;

use fastemu_core::{DeviceProfile, builtin_profiles, default_profile};
use support::Running;
use vendor_support::suite;

const NOKIA: &str = "com.nokia.mid.ui.FullCanvas com.nokia.mid.ui.DirectGraphics \
                     com.nokia.mid.ui.DirectUtils com.nokia.mid.ui.DeviceControl \
                     com.nokia.mid.sound.Sound";

const SIEMENS: &str = "com.siemens.mp.game.Light com.siemens.mp.game.Sprite \
                       com.siemens.mp.ui.Image";

/// What the probe finds on a profile of each maker, by the first word of
/// the profile id.
fn expected(id: &str) -> String {
    let maker = id.split('-').next().unwrap_or_default();
    let found = match maker {
        "generic" | "nokia" | "sonyericsson" => NOKIA,
        "siemens" => SIEMENS,
        _ => "",
    };
    format!("found: {found}").trim_end().to_owned()
}

fn probe(profile: DeviceProfile) {
    let Some(jar) = suite("probe", &["Probe"]) else {
        return;
    };
    let id = profile.id.clone();
    let mut game = Running::launch_on(&jar, 0, profile);
    let line = game.wait_log("found:");
    assert_eq!(line, expected(&id), "on {id}");
}

#[test]
fn every_builtin_profile_shows_only_its_makers_classes() {
    for profile in builtin_profiles() {
        probe(profile);
    }
}

#[test]
fn samsung_and_lg_profiles_show_their_own_classes() {
    for id in ["samsung-176x220", "lg-176x220"] {
        probe(DeviceProfile {
            id: id.to_owned(),
            ..default_profile()
        });
    }
}
