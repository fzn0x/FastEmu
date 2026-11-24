//! Checks the vendor classes against the API inventory in
//! `docs/compat/api/nokia.md` and `vendors.md`: on a profile of each
//! maker every class of its APIs links, and every integer constant the
//! inventory gives a value for has that value. Tier 1 and 2 constants
//! must exist; tier 3 ones are checked when FastEmu has them.

use std::path::Path;
use std::sync::Arc;

use fastemu_core::{DeviceProfile, default_profile};
use fastemu_jvm::{Ctx, MemorySource, Value, Vm, VmConfig};

/// One constant row of an inventory field table.
struct Constant {
    class: String,
    tier: u8,
    name: String,
    desc: String,
    value: i64,
}

/// The text between the first pair of backticks in `cell`.
fn code(cell: &str) -> &str {
    cell.split('`').nth(1).unwrap_or("")
}

/// The integer constants of an inventory file.
fn constants(text: &str) -> Vec<Constant> {
    let mut found = Vec::new();
    let mut class = String::new();
    let mut fields = false;
    for line in text.lines() {
        if line.starts_with("`public ") {
            let words: Vec<&str> = code(line).split_whitespace().collect();
            if let Some(i) = words
                .iter()
                .position(|w| *w == "class" || *w == "interface")
            {
                class = words.get(i + 1).unwrap_or(&"").replace('.', "/");
            }
            continue;
        }
        if line.starts_with("| T |") {
            fields = line.starts_with("| T | Field |");
            continue;
        }
        if !fields || !line.starts_with("| ") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        let (Some(tier), Some(name), Some(desc), Some(value)) =
            (cells.get(1), cells.get(2), cells.get(3), cells.get(4))
        else {
            continue;
        };
        let (Ok(tier), Ok(value)) = (tier.parse(), code(value).parse()) else {
            continue;
        };
        found.push(Constant {
            class: class.clone(),
            tier,
            name: code(name).to_owned(),
            desc: code(desc).to_owned(),
            value,
        });
    }
    found
}

fn as_i64(value: Value) -> Option<i64> {
    match value {
        Value::Int(v) => Some(v.into()),
        Value::Long(v) => Some(v),
        _ => None,
    }
}

/// The problems with the classes and constants of `profile`'s maker.
fn check(ctx: &mut Ctx<'_>, names: &[String], inventory: &[Constant]) -> Vec<String> {
    let mut problems = Vec::new();
    for name in names {
        if ctx.find_class(name).is_err() {
            problems.push(format!("{name} does not link"));
        }
    }
    for c in inventory.iter().filter(|c| names.contains(&c.class)) {
        match ctx.static_field_id(&c.class, &c.name, &c.desc) {
            Ok(field) => {
                let got = ctx.get_static(field).ok().and_then(as_i64);
                if got != Some(c.value) {
                    problems.push(format!(
                        "{}.{} is {got:?}, not {}",
                        c.class, c.name, c.value
                    ));
                }
            }
            Err(_) if c.tier <= 2 => problems.push(format!("{}.{} is missing", c.class, c.name)),
            Err(_) => {}
        }
    }
    problems
}

#[test]
fn vendor_classes_link_with_the_inventory_constants() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/compat/api");
    let mut inventory = Vec::new();
    for file in ["nokia.md", "vendors.md"] {
        let text = std::fs::read_to_string(dir.join(file)).expect("reading the inventory");
        inventory.extend(constants(&text));
    }
    assert!(inventory.len() > 100, "the inventory parsed to too little");
    let mut problems = Vec::new();
    for id in ["nokia-1", "siemens-1", "samsung-1", "motorola-1", "lg-1"] {
        let profile = DeviceProfile {
            id: id.to_owned(),
            ..default_profile()
        };
        let vendor = super::Vendor::of(&profile);
        let mut own = super::classes(vendor);
        if vendor.has_nokia_ui() {
            own.extend(crate::nokia::classes());
        }
        let names: Vec<String> = own.iter().map(|c| c.name().to_owned()).collect();
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            crate::classes(&profile),
        );
        vm.attach("test", |ctx| {
            problems.extend(check(ctx, &names, &inventory));
        });
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
