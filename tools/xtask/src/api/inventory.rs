//! The API inventory in `docs/compat/api/*.md`, parsed: every class with
//! its tier, and every field, constructor and method with its tier,
//! descriptor, flags and constant value.
//!
//! The files have two layouts. Most give each class a declaration line,
//! a `Tier N.` line, and member tables whose header names the kind
//! (`| T | Field | Type | Value | ...`, `| T | Constructor | Descriptor |
//! ...`, `| T | Method | Descriptor | ...`). `jsr-later.md` puts the tier
//! on the declaration line and lists members in one table (`| T | Member
//! | Descriptor | Flags |`), and lists some packages only as classes, in
//! tables of `| Class | Kind | Tier | Members | ... |` rows.

use std::fs;
use std::path::Path;

/// What a member is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Field,
    Constructor,
    Method,
}

/// One row of a member table.
#[derive(Clone, Debug)]
pub struct Member {
    pub kind: Kind,
    pub tier: u8,
    /// `<init>` for constructors.
    pub name: String,
    pub desc: String,
    /// The Flags column: `static`, `protected`, `abstract`, `final`...
    pub flags: String,
    /// The Value column of a field, as written.
    pub value: String,
    /// Listed under "Added by a later edition", which the file's own
    /// member counts leave out.
    pub later: bool,
}

impl Member {
    pub fn has_flag(&self, flag: &str) -> bool {
        self.flags.split_whitespace().any(|f| f == flag)
    }
}

/// One class of the inventory.
#[derive(Clone, Debug)]
pub struct Class {
    /// The internal name, such as `javax/microedition/lcdui/Canvas`.
    pub name: String,
    /// The inventory file it is in, such as `midp-lcdui.md`.
    pub file: String,
    pub tier: u8,
    pub interface: bool,
    pub is_abstract: bool,
    pub is_final: bool,
    /// The superclass and interfaces the declaration names.
    pub supers: Vec<String>,
    pub members: Vec<Member>,
    /// For a class listed without its members, how many the file says it
    /// has.
    pub unlisted_members: Option<usize>,
}

impl Class {
    /// The package, with dots: `javax.microedition.lcdui`.
    pub fn package(&self) -> String {
        self.name
            .rsplit_once('/')
            .map_or_else(String::new, |(p, _)| p.replace('/', "."))
    }

    /// The members the file counts for this class, listed or not; members
    /// of a later edition are not counted.
    pub fn member_count(&self) -> usize {
        self.unlisted_members
            .unwrap_or_else(|| self.members.iter().filter(|m| !m.later).count())
    }
}

/// The text between the first pair of backticks in `cell`.
fn code(cell: &str) -> &str {
    cell.split('`').nth(1).unwrap_or("")
}

fn internal(binary: &str) -> String {
    binary.replace('.', "/")
}

/// The number after `Tier ` in `text`, if any.
fn tier_in(text: &str) -> Option<u8> {
    let rest = text.split("Tier ").nth(1)?;
    rest.chars().next()?.to_digit(10).map(|d| d as u8)
}

fn declaration(line: &str, file: &str) -> Option<Class> {
    let decl = code(line);
    let words: Vec<&str> = decl.split_whitespace().collect();
    let interface = words.contains(&"interface");
    let mut iter = words
        .iter()
        .skip_while(|w| !matches!(**w, "class" | "interface"));
    iter.next()?;
    let name = internal(iter.next()?);
    let supers = iter
        .filter(|w| !matches!(**w, "extends" | "implements"))
        .map(|w| internal(w.trim_end_matches(',')))
        .collect();
    // jsr-later.md puts the tier after the declaration's closing backtick.
    let after = line.splitn(3, '`').nth(2).unwrap_or("");
    Some(Class {
        name,
        file: file.to_owned(),
        tier: tier_in(after).unwrap_or(0),
        interface,
        is_abstract: interface || words.contains(&"abstract"),
        is_final: words.contains(&"final"),
        supers,
        members: Vec::new(),
        unlisted_members: None,
    })
}

fn cells(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_owned())
        .collect()
}

/// A member row under `header`, or `None` for a row that is not one.
fn member(header: &[String], cells: &[String]) -> Option<Member> {
    let column = |name: &str| {
        header
            .iter()
            .position(|h| h == name)
            .and_then(|i| cells.get(i).cloned())
            .unwrap_or_default()
    };
    let tier = cells.first()?.parse().ok()?;
    let flags = column("Flags");
    let method = |descriptor: &str| {
        let open = descriptor.find('(')?;
        let name = &descriptor[..open];
        Some(Member {
            kind: if name == "<init>" {
                Kind::Constructor
            } else {
                Kind::Method
            },
            tier,
            name: name.to_owned(),
            desc: descriptor[open..].to_owned(),
            flags: flags.clone(),
            value: String::new(),
            later: false,
        })
    };
    match header.get(1).map(String::as_str)? {
        "Field" => Some(Member {
            kind: Kind::Field,
            tier,
            name: code(cells.get(1)?).to_owned(),
            desc: code(&column("Type")).to_owned(),
            flags,
            value: column("Value"),
            later: false,
        }),
        "Constructor" | "Method" => method(code(&column("Descriptor"))),
        "Member" => {
            let descriptor = code(&column("Descriptor")).to_owned();
            if descriptor.contains('(') {
                return method(&descriptor);
            }
            let cell = cells.get(1)?;
            Some(Member {
                kind: Kind::Field,
                tier,
                name: code(cell).to_owned(),
                desc: descriptor,
                flags,
                value: cell
                    .split_once('=')
                    .map(|(_, v)| v.trim().to_owned())
                    .unwrap_or_default(),
                later: false,
            })
        }
        _ => None,
    }
}

/// A row of a class-only table: `` | `a.b.C` | interface | 3 | 12 | ... | ``.
fn listed_class(header: &[String], cells: &[String], file: &str) -> Option<Class> {
    // The index at the top of a file names its classes by link or by
    // simple name; only the class-only tables give qualified names.
    let first = cells.first()?;
    if !first.starts_with('`') || !code(first).contains('.') {
        return None;
    }
    let column = |name: &str| {
        header
            .iter()
            .position(|h| h == name)
            .and_then(|i| cells.get(i))
    };
    let interface = column("Kind").is_some_and(|k| k == "interface");
    Some(Class {
        name: internal(code(first)),
        file: file.to_owned(),
        tier: column("Tier")?.parse().ok()?,
        interface,
        is_abstract: interface,
        is_final: false,
        supers: Vec::new(),
        members: Vec::new(),
        unlisted_members: Some(column("Members")?.parse().ok()?),
    })
}

/// Every class of one inventory file.
pub fn parse(file: &str, text: &str) -> Vec<Class> {
    let mut classes: Vec<Class> = Vec::new();
    let mut header: Vec<String> = Vec::new();
    let mut later = false;
    for line in text.lines() {
        if line.starts_with("`public ") {
            classes.extend(declaration(line, file));
            header.clear();
            later = false;
            continue;
        }
        if !line.starts_with('|') {
            header.clear();
            if line.starts_with("Added by a later edition") {
                later = true;
            }
            if line.starts_with("Tier ")
                && let Some(class) = classes.last_mut()
                && class.tier == 0
            {
                class.tier = tier_in(line).unwrap_or(0);
            }
            continue;
        }
        let row = cells(line);
        let separator = |c: &String| !c.is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':'));
        if row.iter().all(separator) {
            continue;
        }
        if header.is_empty() {
            header = row;
            continue;
        }
        if header[0] == "Class" && header.iter().any(|h| h == "Kind") {
            if let Some(class) = listed_class(&header, &row, file)
                && !classes.iter().any(|c| c.name == class.name)
            {
                classes.push(class);
            }
            continue;
        }
        if header[0] == "T"
            && let Some(m) = member(&header, &row)
            && let Some(class) = classes.last_mut()
        {
            class.members.push(Member { later, ..m });
        }
    }
    classes
}

/// The parsed inventory.
pub struct Inventory {
    pub classes: Vec<Class>,
    /// Each file whose classes or members do not add up to the counts the
    /// README's index gives: a row the parser could not read.
    pub problems: Vec<String>,
}

impl Inventory {
    /// Reads every file of the index in `api_dir`'s `README.md`, in its
    /// order, and checks each against the index's counts.
    pub fn load(api_dir: &Path) -> Result<Self, String> {
        let read = |name: &str| {
            let path = api_dir.join(name);
            fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))
        };
        let index = read("README.md")?;
        let mut classes = Vec::new();
        let mut problems = Vec::new();
        for file in index_files(&index) {
            let parsed = parse(&file, &read(&file)?);
            if let Some((want_classes, want_members)) = indexed_counts(&index, &file) {
                let members: usize = parsed.iter().map(Class::member_count).sum();
                if (parsed.len(), members) != (want_classes, want_members) {
                    problems.push(format!(
                        "{file}: read {} classes and {members} members, the index says \
                         {want_classes} and {want_members}",
                        parsed.len()
                    ));
                }
            }
            classes.extend(parsed);
        }
        Ok(Self { classes, problems })
    }
}

/// The files the README's index lists, in order: the first link of every
/// row of its Files table.
pub fn index_files(index: &str) -> Vec<String> {
    index
        .lines()
        .filter(|l| l.starts_with("| ["))
        .filter_map(|l| {
            let target = l.split("](").nth(1)?.split(')').next()?;
            target
                .to_ascii_lowercase()
                .ends_with(".md")
                .then(|| target.to_owned())
        })
        .collect()
}

/// The class and member counts the README's index gives for `file`, when
/// it gives them.
pub fn indexed_counts(index: &str, file: &str) -> Option<(usize, usize)> {
    let row = index
        .lines()
        .find(|l| l.starts_with(&format!("| [{file}]")))?;
    let numbers: Vec<usize> = row
        .split('|')
        .filter_map(|cell| cell.trim().parse().ok())
        .collect();
    match numbers[..] {
        [classes, members] => Some((classes, members)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn api_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/compat/api")
    }

    #[test]
    fn both_layouts_parse() {
        let text = "\
`public abstract class javax.microedition.lcdui.game.GameCanvas extends javax.microedition.lcdui.Canvas`

Tier 1. Since MIDP 2.0.

| T | Field | Type | Value | Flags | Since | Notes |
|---|---|---|---|---|---|---|
| 2 | `FIRE_PRESSED` | `I` | `256` (`0x100`) | static | 2.0 |  |

| T | Method | Descriptor | Flags | Since | Throws | Notes |
|---|---|---|---|---|---|---|
| 1 | `void paint(Graphics g)` | `paint(Ljavax/microedition/lcdui/Graphics;)V` |  | 2.0 | `NullPointerException` if g is null; `IOException` |  |

`public class a.b.Track extends a.b.Object3D`. Tier 3. [Javadoc](x).

| T | Member | Descriptor | Flags |
|---|---|---|---|
| 3 | `ALPHA` = `256` (`0x100`) | `I` | static |
| 3 | `(int property)` | `<init>(I)V` |  |

| Class | Kind | Tier | Members | Javadoc |
|---|---|---|---|---|
| `a.b.List` | interface | 3 | 8 | [page](x) |
";
        let classes = parse("x.md", text);
        assert_eq!(classes.len(), 3);
        let game = &classes[0];
        assert_eq!(game.name, "javax/microedition/lcdui/game/GameCanvas");
        assert_eq!((game.tier, game.is_abstract), (1, true));
        assert_eq!(game.supers, ["javax/microedition/lcdui/Canvas"]);
        let fire = &game.members[0];
        assert_eq!(
            (fire.kind, fire.tier, fire.name.as_str()),
            (Kind::Field, 2, "FIRE_PRESSED")
        );
        assert_eq!(code(&fire.value), "256");
        let paint = &game.members[1];
        assert_eq!(paint.desc, "(Ljavax/microedition/lcdui/Graphics;)V");
        let track = &classes[1];
        assert_eq!(track.tier, 3);
        assert_eq!(code(&track.members[0].value), "256");
        assert_eq!(track.members[1].kind, Kind::Constructor);
        assert_eq!(classes[2].name, "a/b/List");
        assert_eq!(classes[2].member_count(), 8);
        assert!(classes[2].interface);
    }

    /// The parser reads every class and member of the real inventory: its
    /// counts match the ones the README's index gives.
    #[test]
    fn the_inventory_parses_to_the_indexed_counts() {
        let inventory = Inventory::load(&api_dir()).unwrap();
        assert!(inventory.problems.is_empty(), "{:?}", inventory.problems);
        let files: Vec<&str> = inventory.classes.iter().map(|c| c.file.as_str()).collect();
        assert!(files.contains(&"jsr-later.md") && files.contains(&"vendors.md"));
        for class in &inventory.classes {
            assert!((1..=3).contains(&class.tier), "{} has no tier", class.name);
        }
    }
}
