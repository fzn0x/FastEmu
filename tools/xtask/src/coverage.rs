//! `cargo xtask coverage`: how much of the API inventory FastEmu
//! registers, written to `docs/compat/coverage.md`.
//!
//! Every class and member of `docs/compat/api/*.md` is looked up in the
//! classes the runtime registers ([`crate::api::runtime`]) the way a game's
//! reference to it would link: fields and methods through superclasses and
//! interfaces, constructors in the class itself. A member that links but
//! differs from the inventory (its `static` flag, its access, a constant's
//! value, a class that is abstract or final where the spec's is not) is
//! listed as a difference.

use std::fmt::Write as _;
use std::path::Path;

use fastemu_jvm::classfile::access;

use crate::api::inventory::{self, Inventory, Kind, Member};
use crate::api::runtime::{Constant, Runtime};

/// Present and total counts for one tier.
#[derive(Clone, Copy, Default)]
struct Count {
    present: usize,
    total: usize,
}

impl Count {
    fn add(&mut self, present: bool) {
        self.total += 1;
        self.present += usize::from(present);
    }

    fn text(self) -> String {
        if self.total == 0 {
            "none".to_owned()
        } else {
            format!("{} of {}", self.present, self.total)
        }
    }
}

/// Counts of classes and members per tier, index 0 for tier 1.
#[derive(Clone, Copy, Default)]
struct Tally {
    classes: [Count; 3],
    members: [Count; 3],
}

impl Tally {
    fn merge(&mut self, other: &Tally) {
        for t in 0..3 {
            self.classes[t].present += other.classes[t].present;
            self.classes[t].total += other.classes[t].total;
            self.members[t].present += other.members[t].present;
            self.members[t].total += other.members[t].total;
        }
    }
}

fn tier_index(tier: u8) -> usize {
    usize::from(tier.clamp(1, 3) - 1)
}

/// One row of a package's class table.
struct ClassRow {
    simple: String,
    tier: u8,
    present: bool,
    members: [Count; 2],
}

/// What one package of the inventory comes to.
#[derive(Default)]
struct Package {
    /// The inventory files its classes are in.
    files: Vec<String>,
    tally: Tally,
    rows: Vec<ClassRow>,
    /// Missing tier 1 and 2 members of classes FastEmu has.
    missing: Vec<String>,
}

/// The whole comparison.
struct Report {
    /// Packages in the order the inventory first names them.
    packages: Vec<(String, Package)>,
    differences: Vec<String>,
}

/// The text between the first pair of backticks.
fn code(cell: &str) -> &str {
    cell.split('`').nth(1).unwrap_or("")
}

/// Java string literal text without its quotes, unescaped.
fn unescape(literal: &str) -> Option<Vec<u16>> {
    let inner = literal.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = Vec::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buf = [0; 2];
            out.extend_from_slice(c.encode_utf16(&mut buf));
            continue;
        }
        match chars.next()? {
            'n' => out.push(0x0A),
            'r' => out.push(0x0D),
            't' => out.push(0x09),
            'u' => {
                let hex: String = chars.by_ref().take(4).collect();
                out.push(u16::from_str_radix(&hex, 16).ok()?);
            }
            other => {
                let mut buf = [0; 2];
                out.extend_from_slice(other.encode_utf16(&mut buf));
            }
        }
    }
    Some(out)
}

/// The constant the inventory gives for a field of type `desc`, if its
/// Value column states one.
fn expected_constant(desc: &str, value: &str) -> Option<Constant> {
    if value.is_empty()
        || value.starts_with("(not a constant)")
        || value.contains("not in the sources")
    {
        return None;
    }
    let first = code(value);
    let bits = value
        .split("bits `")
        .nth(1)
        .and_then(|rest| rest.split('`').next())
        .and_then(|hex| u64::from_str_radix(hex.trim_start_matches("0x"), 16).ok());
    match desc {
        "I" | "S" | "B" => first.parse().ok().map(Constant::Int),
        "Z" => match first {
            "true" => Some(Constant::Int(1)),
            "false" => Some(Constant::Int(0)),
            _ => None,
        },
        "J" => first.trim_end_matches('L').parse().ok().map(Constant::Long),
        "C" => value
            .split('(')
            .nth(1)
            .and_then(|rest| rest.split(')').next())
            .and_then(|n| n.parse().ok())
            .map(Constant::Int),
        "F" => bits.map(|b| Constant::Float(f32::from_bits(b as u32))),
        "D" => bits.map(|b| Constant::Double(f64::from_bits(b))),
        "Ljava/lang/String;" => unescape(first).map(Constant::String),
        _ => None,
    }
}

fn same_constant(a: &Constant, b: &Constant) -> bool {
    match (a, b) {
        (Constant::Float(x), Constant::Float(y)) => x.to_bits() == y.to_bits(),
        (Constant::Double(x), Constant::Double(y)) => x.to_bits() == y.to_bits(),
        _ => a == b,
    }
}

fn access_word(flags: u16) -> &'static str {
    if flags & access::PUBLIC != 0 {
        "public"
    } else if flags & access::PROTECTED != 0 {
        "protected"
    } else if flags & access::PRIVATE != 0 {
        "private"
    } else {
        "package-private"
    }
}

/// Whether `m` links in FastEmu, and how it differs from the inventory.
fn check_member(
    runtime: &Runtime,
    class: &inventory::Class,
    m: &Member,
    differences: &mut Vec<String>,
) -> bool {
    let what = match m.kind {
        Kind::Field => format!("`{}.{}:{}`", class.name, m.name, m.desc),
        _ => format!("`{}.{}{}`", class.name, m.name, m.desc),
    };
    let mut differ = |text: String| differences.push(format!("{what} (tier {}): {text}", m.tier));
    let want_static = m.has_flag("static");
    let want_access = if m.has_flag("protected") {
        "protected"
    } else {
        "public"
    };
    let flags = match m.kind {
        Kind::Field => {
            let Some(field) = runtime.resolve_field(&class.name, &m.name, &m.desc) else {
                return false;
            };
            if let Some(expected) = expected_constant(&m.desc, &m.value) {
                match &field.constant {
                    Some(c) if same_constant(c, &expected) => {}
                    Some(c) => differ(format!(
                        "the constant is {c:?}, the inventory says {expected:?}"
                    )),
                    None => differ(format!("not a constant, the inventory says {expected:?}")),
                }
            }
            field.flags
        }
        Kind::Constructor => {
            let Some(ctor) = runtime
                .get(&class.name)
                .and_then(|c| c.method("<init>", &m.desc))
            else {
                return false;
            };
            ctor.flags
        }
        Kind::Method => {
            let Some(method) = runtime.resolve_method(&class.name, &m.name, &m.desc) else {
                return false;
            };
            let is_abstract = method.flags & access::ABSTRACT != 0;
            let want_abstract = class.interface || m.has_flag("abstract");
            if is_abstract && !want_abstract {
                differ("abstract, the inventory says it is not".to_owned());
            }
            if method.flags & access::FINAL != 0 && !m.has_flag("final") {
                differ(
                    "final, so a game cannot override it; the inventory says it is not".to_owned(),
                );
            }
            method.flags
        }
    };
    if (flags & access::STATIC != 0) != want_static {
        let (is, says) = if want_static {
            ("not static", "static")
        } else {
            ("static", "not static")
        };
        differ(format!("{is}, the inventory says {says}"));
    }
    let is_access = access_word(flags);
    if is_access != want_access && !(class.interface && is_access == "public") {
        differ(format!("{is_access}, the inventory says {want_access}"));
    }
    true
}

/// How a class FastEmu has differs from its inventory declaration.
fn check_class(runtime: &Runtime, class: &inventory::Class, differences: &mut Vec<String>) {
    let Some(have) = runtime.get(&class.name) else {
        return;
    };
    let mut differ = |text: String| {
        differences.push(format!("`{}` (tier {}): {text}", class.name, class.tier));
    };
    if have.is_interface() != class.interface {
        differ(if class.interface {
            "a class, the inventory says an interface".to_owned()
        } else {
            "an interface, the inventory says a class".to_owned()
        });
    } else if class.unlisted_members.is_none() {
        // A class listed without its members has no declaration to
        // compare with.
        if !class.interface && have.is_abstract() && !class.is_abstract {
            differ("abstract, so a game cannot make one; the inventory says it is not".to_owned());
        }
        if have.flags & access::FINAL != 0 && !class.is_final {
            differ("final, so a game cannot extend it; the inventory says it is not".to_owned());
        }
    }
    for parent in &class.supers {
        if !runtime.is_subtype(&class.name, parent) {
            differ(format!("not a `{parent}`"));
        }
    }
}

fn compare(runtime: &Runtime, inventory: &Inventory) -> Report {
    let mut packages: Vec<(String, Package)> = Vec::new();
    let mut differences = Vec::new();
    for class in &inventory.classes {
        let name = class.package();
        let index = match packages.iter().position(|(p, _)| *p == name) {
            Some(i) => i,
            None => {
                packages.push((name, Package::default()));
                packages.len() - 1
            }
        };
        let package = &mut packages[index].1;
        if !package.files.contains(&class.file) {
            package.files.push(class.file.clone());
        }
        let present = runtime.get(&class.name).is_some();
        package.tally.classes[tier_index(class.tier)].add(present);
        check_class(runtime, class, &mut differences);
        let mut members = [Count::default(); 2];
        for m in &class.members {
            let linked = present && check_member(runtime, class, m, &mut differences);
            package.tally.members[tier_index(m.tier)].add(linked);
            if m.tier <= 2 {
                members[tier_index(m.tier)].add(linked);
                if present && !linked {
                    let shown = match m.kind {
                        Kind::Field => format!("{}:{}", m.name, m.desc),
                        _ => format!("{}{}", m.name, m.desc),
                    };
                    let simple = class.name.rsplit('/').next().unwrap_or(&class.name);
                    package
                        .missing
                        .push(format!("tier {}: `{simple}.{shown}`", m.tier));
                }
            }
        }
        if class.tier <= 2 || members.iter().any(|c| c.total > 0) {
            package.rows.push(ClassRow {
                simple: class
                    .name
                    .rsplit('/')
                    .next()
                    .unwrap_or(&class.name)
                    .to_owned(),
                tier: class.tier,
                present,
                members,
            });
        }
    }
    Report {
        packages,
        differences,
    }
}

const INTRO: &str = "\
# API coverage

How much of the [API inventory](api/README.md) FastEmu registers today.
This page is written by `cargo xtask coverage`; run it again after
changing the runtime's classes or the inventory, and commit the result.

A class counts as present when the runtime registers it. A member counts
as present when a game's reference to it links: a field or method
through the class, its superclasses and its interfaces, as the JVM
resolves it, and a constructor in the class itself. The classes of
`jsr-later.md` that the inventory lists without their members (PIM,
Bluetooth, Location, SVG) count as classes only. Tiers are the
inventory's: 1 for what most games need, 2 for what many need, 3 for the
rest.
";

fn render(report: &Report) -> String {
    let mut out = String::from(INTRO);
    let mut total = Tally::default();
    out.push_str("\n## Summary\n\n");
    out.push_str("| Package | Tier 1 classes | Tier 1 members | Tier 2 classes | Tier 2 members | Tier 3 classes | Tier 3 members |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for (name, package) in &report.packages {
        let t = &package.tally;
        total.merge(t);
        let _ = writeln!(
            out,
            "| [`{name}`](#{}) | {} | {} | {} | {} | {} | {} |",
            anchor(name),
            t.classes[0].text(),
            t.members[0].text(),
            t.classes[1].text(),
            t.members[1].text(),
            t.classes[2].text(),
            t.members[2].text()
        );
    }
    let _ = writeln!(
        out,
        "| All | {} | {} | {} | {} | {} | {} |",
        total.classes[0].text(),
        total.members[0].text(),
        total.classes[1].text(),
        total.members[1].text(),
        total.classes[2].text(),
        total.members[2].text()
    );
    out.push_str("\n## Differences\n\n");
    out.push_str(
        "Classes and members FastEmu has that differ from the inventory. An\n\
         override that is protected in a game and public here links and runs;\n\
         the other kinds can break a game. The stubs that test MIDlets compile\n\
         against (`cargo xtask stubs`) follow FastEmu, so a test MIDlet declares\n\
         such an override public.\n\n",
    );
    if report.differences.is_empty() {
        out.push_str("None.\n");
    }
    for d in &report.differences {
        let _ = writeln!(out, "- {d}");
    }
    out.push_str("\n## By package\n");
    for (name, package) in &report.packages {
        let _ = writeln!(out, "\n### {name}\n");
        let files: Vec<String> = package
            .files
            .iter()
            .map(|f| format!("[{f}](api/{f})"))
            .collect();
        let t = &package.tally;
        let _ = writeln!(
            out,
            "From {}. Tier 1: {} classes, {} members. Tier 2: {} classes, {} members. \
             Tier 3: {} classes, {} members.",
            files.join(", "),
            t.classes[0].text(),
            t.members[0].text(),
            t.classes[1].text(),
            t.members[1].text(),
            t.classes[2].text(),
            t.members[2].text()
        );
        if package.rows.is_empty() {
            continue;
        }
        out.push_str("\n| Class | Tier | In FastEmu | Tier 1 members | Tier 2 members |\n");
        out.push_str("|---|---|---|---|---|\n");
        for row in &package.rows {
            let _ = writeln!(
                out,
                "| `{}` | {} | {} | {} | {} |",
                row.simple,
                row.tier,
                if row.present { "yes" } else { "missing" },
                row.members[0].text(),
                row.members[1].text()
            );
        }
        if !package.missing.is_empty() {
            out.push_str("\nMissing tier 1 and 2 members of the classes FastEmu has:\n\n");
            for m in &package.missing {
                let _ = writeln!(out, "- {m}");
            }
        }
    }
    out
}

/// The anchor GitHub gives a `### name` heading.
fn anchor(heading: &str) -> String {
    heading
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .collect::<String>()
        .to_ascii_lowercase()
}

/// The counts of the whole report, for the command's summary line.
fn totals(report: &Report) -> Tally {
    let mut total = Tally::default();
    for (_, package) in &report.packages {
        total.merge(&package.tally);
    }
    total
}

/// Writes the report for the repository at `root` to `out`.
pub fn run(root: &Path, out: &Path) -> Result<(), String> {
    let runtime = Runtime::load();
    let inventory = Inventory::load(&root.join("docs/compat/api"))?;
    if !inventory.problems.is_empty() {
        return Err(format!(
            "the inventory did not parse as its index says:\n{}",
            inventory.problems.join("\n")
        ));
    }
    let report = compare(&runtime, &inventory);
    std::fs::write(out, render(&report)).map_err(|e| format!("{}: {e}", out.display()))?;
    let t = totals(&report);
    for i in 0..3 {
        println!(
            "tier {}: {} classes, {} members",
            i + 1,
            t.classes[i].text(),
            t.members[i].text()
        );
    }
    println!(
        "{} differences; wrote {}",
        report.differences.len(),
        out.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastemu_jvm::{NativeClass, Value};

    fn report(natives: &[NativeClass], text: &str) -> Report {
        let runtime = Runtime::from_natives(natives);
        let inventory = Inventory {
            classes: inventory::parse("t.md", text),
            problems: Vec::new(),
        };
        compare(&runtime, &inventory)
    }

    const INVENTORY: &str = "\
`public class a.Shape extends java.lang.Object`

Tier 1.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 1 | `SIDES` | `I` | `4` | static | 1.0 |  |
| 2 | `NAME` | `Ljava/lang/String;` | `\"sq\"` | static | 1.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `int area()` | `area()I` | protected | 1.0 |  |  |
| 2 | `void grow()` | `grow()V` |  | 1.0 |  |  |
| 1 | `int hashCode()` | `hashCode()I` |  | 1.0 |  |  |

`public class a.Gone extends java.lang.Object`

Tier 2.
";

    #[test]
    fn members_are_counted_and_differences_named() {
        let natives = [
            NativeClass::new("java/lang/Object")
                .method("hashCode", "()I", |_, _| Ok(Value::Int(0))),
            NativeClass::new("a/Shape")
                .constant("SIDES", "I", Value::Int(5))
                .string_constant("NAME", "sq")
                .method("area", "()I", |_, _| Ok(Value::Int(0))),
        ];
        let report = report(&natives, INVENTORY);
        let (name, package) = &report.packages[0];
        assert_eq!(name, "a");
        assert_eq!(
            (
                package.tally.classes[0].present,
                package.tally.classes[0].total
            ),
            (1, 1)
        );
        assert_eq!(package.tally.classes[1].total, 1);
        assert_eq!(package.tally.classes[1].present, 0);
        assert_eq!(package.tally.members[0].present, 3, "hashCode is inherited");
        assert_eq!(package.tally.members[1].present, 1);
        assert_eq!(package.missing, ["tier 2: `Shape.grow()V`"]);
        assert_eq!(report.differences.len(), 2, "{:?}", report.differences);
        assert!(report.differences[0].contains("SIDES:I"));
        assert!(report.differences[1].contains("public, the inventory says protected"));
        let text = render(&report);
        assert!(
            text.contains("| `Gone` | 2 | missing | none | none |"),
            "{text}"
        );
    }

    #[test]
    fn inventory_constants_parse() {
        assert_eq!(
            expected_constant("I", "`256` (`0x100`)"),
            Some(Constant::Int(256))
        );
        assert_eq!(
            expected_constant("J", "`-9223372036854775808L` (`0x8000000000000000L`)"),
            Some(Constant::Long(i64::MIN))
        );
        assert_eq!(
            expected_constant("C", "`'\\uffff'` (65535)"),
            Some(Constant::Int(65535))
        );
        assert_eq!(
            expected_constant("F", "`NaN` (bits `0x7fc00000`)").map(|c| match c {
                Constant::Float(f) => f.to_bits(),
                _ => 0,
            }),
            Some(0x7fc0_0000)
        );
        assert_eq!(
            expected_constant("Ljava/lang/String;", "`\"device://tone\"`"),
            Some(Constant::String("device://tone".encode_utf16().collect()))
        );
        assert_eq!(expected_constant("I", "(not a constant)"), None);
    }
}
