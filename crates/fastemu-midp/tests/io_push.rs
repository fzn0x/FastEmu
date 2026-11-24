//! PushRegistry through a Java program (tests/io_java/PushTest.java).

mod io_support;

use std::sync::Arc;

use fastemu_core::{MidletInfo, NetworkPolicy, SuiteInfo};
use fastemu_midp::env::InstanceEnv;

#[test]
fn push_registrations_are_checked_and_never_fire() {
    let Some(classes) = io_support::classes() else {
        return;
    };
    let output = io_support::run_with(classes, "PushTest", &[], NetworkPolicy::Deny, |ctx| {
        let suite = SuiteInfo {
            midlets: vec![MidletInfo {
                name: "The game".into(),
                icon_path: None,
                class_name: "Game".into(),
            }],
            ..SuiteInfo::default()
        };
        ctx.extension_mut::<InstanceEnv>()
            .expect("the runner installs an environment")
            .suite = Arc::new(suite);
    });
    let expected = "registered
list 0 0
midlet null filter null
unregister false
alarm 0 0
bad invalid connection nocolon
other Other is not a MIDlet of this suite
null the MIDlet name is null
done";
    io_support::assert_output(&output, expected);
}
