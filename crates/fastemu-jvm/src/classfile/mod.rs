//! Class file parsing (JVMS chapter 4).
//!
//! The parser accepts versions 45.3 (javac 1.1) to 52.0 (Java 8), keeps the
//! attributes the virtual machine uses (`Code`, `LineNumberTable`,
//! `ConstantValue`, `SourceFile`) and skips the rest, including the
//! `StackMap` and `StackMapTable` attributes of preverified classes.

mod constant;
mod reader;

use std::collections::HashSet;
use std::fmt;
use std::sync::Arc;

pub use constant::{
    Constant, ConstantPool, MemberRef, Utf8, decode_modified_utf8, encode_modified_utf8,
};
use reader::Reader;

/// Access and property flags (JVMS tables 4.1-B, 4.5-A and 4.6-A).
pub mod access {
    pub const PUBLIC: u16 = 0x0001;
    pub const PRIVATE: u16 = 0x0002;
    pub const PROTECTED: u16 = 0x0004;
    pub const STATIC: u16 = 0x0008;
    pub const FINAL: u16 = 0x0010;
    /// `ACC_SUPER` on classes, `ACC_SYNCHRONIZED` on methods.
    pub const SUPER: u16 = 0x0020;
    pub const SYNCHRONIZED: u16 = 0x0020;
    pub const VOLATILE: u16 = 0x0040;
    pub const BRIDGE: u16 = 0x0040;
    pub const TRANSIENT: u16 = 0x0080;
    pub const VARARGS: u16 = 0x0080;
    pub const NATIVE: u16 = 0x0100;
    pub const INTERFACE: u16 = 0x0200;
    pub const ABSTRACT: u16 = 0x0400;
    pub const STRICT: u16 = 0x0800;
    pub const SYNTHETIC: u16 = 0x1000;
    pub const ANNOTATION: u16 = 0x2000;
    pub const ENUM: u16 = 0x4000;
}

/// The oldest and newest class file major versions FastEmu runs.
pub const MIN_MAJOR: u16 = 45;
pub const MAX_MAJOR: u16 = 52;

/// Why a class file could not be read. The virtual machine reports it as
/// a `ClassFormatError`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FormatError(pub String);

impl FormatError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for FormatError {}

/// One entry of a method's exception table, in bytecode offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExceptionHandler {
    pub start_pc: u16,
    pub end_pc: u16,
    pub handler_pc: u16,
    /// A `CONSTANT_Class` index, or 0 to catch everything (`finally`).
    pub catch_type: u16,
}

/// A method's `Code` attribute.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CodeAttr {
    pub max_stack: u16,
    pub max_locals: u16,
    pub code: Vec<u8>,
    pub handlers: Vec<ExceptionHandler>,
    /// `(start_pc, line)` pairs from every `LineNumberTable`.
    pub lines: Vec<(u16, u16)>,
}

impl CodeAttr {
    /// The source line of the instruction at bytecode offset `pc`.
    pub fn line_at(&self, pc: u32) -> Option<u16> {
        self.lines
            .iter()
            .filter(|&&(start, _)| u32::from(start) <= pc)
            .max_by_key(|&&(start, _)| start)
            .map(|&(_, line)| line)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldInfo {
    pub access: u16,
    pub name: Arc<str>,
    pub descriptor: Arc<str>,
    /// The `ConstantValue` attribute's constant pool index.
    pub constant_value: Option<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MethodInfo {
    pub access: u16,
    pub name: Arc<str>,
    pub descriptor: Arc<str>,
    /// `None` for abstract and native methods.
    pub code: Option<CodeAttr>,
}

/// A parsed class file.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassFile {
    pub minor_version: u16,
    pub major_version: u16,
    pub constant_pool: ConstantPool,
    pub access: u16,
    /// The internal name, such as `com/example/Game`.
    pub name: Arc<str>,
    /// `None` only for `java/lang/Object`.
    pub super_name: Option<Arc<str>>,
    pub interfaces: Vec<Arc<str>>,
    pub fields: Vec<FieldInfo>,
    pub methods: Vec<MethodInfo>,
    pub source_file: Option<Arc<str>>,
}

impl ClassFile {
    /// Parses a class file.
    pub fn parse(bytes: &[u8]) -> Result<Self, FormatError> {
        let mut r = Reader::new(bytes);
        if r.u32()? != 0xCAFE_BABE {
            return Err(FormatError::new("not a class file (bad magic number)"));
        }
        let minor_version = r.u16()?;
        let major_version = r.u16()?;
        if !(MIN_MAJOR..=MAX_MAJOR).contains(&major_version) {
            return Err(FormatError::new(format!(
                "unsupported class file version {major_version}.{minor_version}"
            )));
        }
        let cp = ConstantPool::parse(&mut r)?;
        let access = r.u16()?;
        let name = cp.class_name_arc(r.u16()?)?;
        let super_index = r.u16()?;
        let super_name = if super_index == 0 {
            None
        } else {
            Some(cp.class_name_arc(super_index)?)
        };
        if super_name.is_none() && &*name != "java/lang/Object" {
            return Err(FormatError::new(format!("{name} has no superclass")));
        }
        let interface_count = r.u16()?;
        let mut interfaces = Vec::with_capacity(interface_count as usize);
        for _ in 0..interface_count {
            interfaces.push(cp.class_name_arc(r.u16()?)?);
        }
        let field_count = r.u16()?;
        let mut fields = Vec::with_capacity(field_count as usize);
        // The pool entries whose text was found to be a field descriptor.
        // Fields share descriptors, and one can take 65535 bytes, so a
        // descriptor is walked once however many fields name it.
        let mut sound = HashSet::new();
        for _ in 0..field_count {
            fields.push(parse_field(&mut r, &cp, &mut sound)?);
        }
        let method_count = r.u16()?;
        let mut methods = Vec::with_capacity(method_count as usize);
        // The pool entries whose descriptors were found sound. Methods
        // share descriptors, and one can name 65532 parameters, so a
        // descriptor is walked once however many methods name it.
        let mut checked = HashSet::new();
        for _ in 0..method_count {
            methods.push(parse_method(&mut r, &cp, &mut checked)?);
        }
        let mut source_file = None;
        let attr_count = r.u16()?;
        for _ in 0..attr_count {
            let (attr_name, body) = attribute(&mut r, &cp)?;
            if attr_name == "SourceFile" && body.len() == 2 {
                let index = u16::from_be_bytes([body[0], body[1]]);
                source_file = cp.utf8(index).ok().map(Arc::from);
            }
        }
        // Trailing bytes after the last attribute are tolerated: some
        // obfuscators pad class files.
        Ok(Self {
            minor_version,
            major_version,
            constant_pool: cp,
            access,
            name,
            super_name,
            interfaces,
            fields,
            methods,
            source_file,
        })
    }

    pub fn is_interface(&self) -> bool {
        self.access & access::INTERFACE != 0
    }
}

fn attribute<'b, 'c>(
    r: &mut Reader<'b>,
    cp: &'c ConstantPool,
) -> Result<(&'c str, &'b [u8]), FormatError> {
    let name = cp.utf8(r.u16()?)?;
    let len = r.u32()? as usize;
    Ok((name, r.bytes(len)?))
}

fn parse_field(
    r: &mut Reader<'_>,
    cp: &ConstantPool,
    sound: &mut HashSet<u16>,
) -> Result<FieldInfo, FormatError> {
    let access = r.u16()?;
    let name = cp.utf8_arc(r.u16()?)?;
    let descriptor_index = r.u16()?;
    let descriptor = cp.utf8_arc(descriptor_index)?;
    if !sound.contains(&descriptor_index) {
        if !crate::descriptor::is_field_descriptor(&descriptor) {
            return Err(FormatError::new(format!(
                "field {name} has a bad descriptor {descriptor}"
            )));
        }
        sound.insert(descriptor_index);
    }
    let mut constant_value = None;
    for _ in 0..r.u16()? {
        let (attr_name, body) = attribute(r, cp)?;
        if attr_name == "ConstantValue" && body.len() == 2 {
            constant_value = Some(u16::from_be_bytes([body[0], body[1]]));
        }
    }
    Ok(FieldInfo {
        access,
        name,
        descriptor,
        constant_value,
    })
}

fn parse_method(
    r: &mut Reader<'_>,
    cp: &ConstantPool,
    checked: &mut HashSet<u16>,
) -> Result<MethodInfo, FormatError> {
    let access = r.u16()?;
    let name = cp.utf8_arc(r.u16()?)?;
    let descriptor_index = r.u16()?;
    let descriptor = cp.utf8_arc(descriptor_index)?;
    if !checked.contains(&descriptor_index) {
        if crate::descriptor::method_shape(&descriptor).is_none() {
            return Err(FormatError::new(format!(
                "method {name} has a bad descriptor {descriptor}"
            )));
        }
        checked.insert(descriptor_index);
    }
    let mut code = None;
    for _ in 0..r.u16()? {
        let (attr_name, body) = attribute(r, cp)?;
        if attr_name == "Code" {
            code = Some(parse_code(body, cp).map_err(|e| {
                FormatError::new(format!("in the code of {name}{descriptor}: {e}"))
            })?);
        }
    }
    Ok(MethodInfo {
        access,
        name,
        descriptor,
        code,
    })
}

fn parse_code(body: &[u8], cp: &ConstantPool) -> Result<CodeAttr, FormatError> {
    let mut r = Reader::new(body);
    let max_stack = r.u16()?;
    let max_locals = r.u16()?;
    let code_len = r.u32()? as usize;
    if code_len == 0 || code_len > 65535 {
        return Err(FormatError::new(format!("code length {code_len}")));
    }
    let code = r.bytes(code_len)?.to_vec();
    let handler_count = r.u16()?;
    let mut handlers = Vec::with_capacity(handler_count as usize);
    for _ in 0..handler_count {
        handlers.push(ExceptionHandler {
            start_pc: r.u16()?,
            end_pc: r.u16()?,
            handler_pc: r.u16()?,
            catch_type: r.u16()?,
        });
    }
    // The class memory counts the 4 bytes of each line number, and the
    // list stays for as long as the class: it must not keep the spare room
    // of a list that doubled as it grew. Code can have many tables, so
    // the list grows as usual and is cut to its length once.
    let mut lines = Vec::new();
    for _ in 0..r.u16()? {
        let (attr_name, attr) = attribute(&mut r, cp)?;
        if attr_name == "LineNumberTable" {
            let mut lr = Reader::new(attr);
            let count = lr.u16()?;
            lines.reserve(usize::from(count));
            for _ in 0..count {
                lines.push((lr.u16()?, lr.u16()?));
            }
        }
    }
    lines.shrink_to_fit();
    Ok(CodeAttr {
        max_stack,
        max_locals,
        code,
        handlers,
        lines,
    })
}

/// A class file `T` of abstract methods named `m`: `per_descriptor` of them
/// for each of `descriptors`, which are one pool entry each. For the tests
/// of what a class file with many methods costs to read and to link.
#[cfg(test)]
pub(crate) fn class_of_methods(descriptors: &[&str], per_descriptor: usize) -> Vec<u8> {
    let mut b = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 49];
    let utf8 = |b: &mut Vec<u8>, s: &str| {
        b.push(1);
        b.extend((s.len() as u16).to_be_bytes());
        b.extend(s.as_bytes());
    };
    b.extend((6 + descriptors.len() as u16).to_be_bytes());
    utf8(&mut b, "T"); // 1
    b.extend([7, 0, 1]); // 2: class T
    utf8(&mut b, "java/lang/Object"); // 3
    b.extend([7, 0, 3]); // 4: class Object
    utf8(&mut b, "m"); // 5
    for descriptor in descriptors {
        utf8(&mut b, descriptor); // 6 and on
    }
    b.extend(0x0421u16.to_be_bytes()); // public, super, abstract
    b.extend(2u16.to_be_bytes());
    b.extend(4u16.to_be_bytes());
    b.extend(0u16.to_be_bytes()); // interfaces
    b.extend(0u16.to_be_bytes()); // fields
    b.extend(((descriptors.len() * per_descriptor) as u16).to_be_bytes());
    for i in 0..descriptors.len() {
        for _ in 0..per_descriptor {
            b.extend(0x0401u16.to_be_bytes()); // public abstract
            b.extend(5u16.to_be_bytes());
            b.extend((6 + i as u16).to_be_bytes());
            b.extend(0u16.to_be_bytes()); // attributes
        }
    }
    b.extend(0u16.to_be_bytes()); // attributes
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::descriptor::{take_shapes, take_walks};

    /// A hand-assembled `public class T { static int f() { return 7; } }`
    /// with a `SourceFile` and a `LineNumberTable`.
    fn tiny_class(major: u16, minor: u16) -> Vec<u8> {
        class_with_lines(major, minor, &[1])
    }

    /// [`tiny_class`] with a `LineNumberTable` of each length of `tables`
    /// in the code of `f`, every entry line 3 from offset 0.
    fn class_with_lines(major: u16, minor: u16, tables: &[u16]) -> Vec<u8> {
        let mut b = vec![0xCA, 0xFE, 0xBA, 0xBE];
        b.extend(minor.to_be_bytes());
        b.extend(major.to_be_bytes());
        let utf8 = |b: &mut Vec<u8>, s: &str| {
            b.push(1);
            b.extend((s.len() as u16).to_be_bytes());
            b.extend(s.as_bytes());
        };
        b.extend(10u16.to_be_bytes()); // 9 entries
        utf8(&mut b, "T"); // 1
        b.extend([7, 0, 1]); // 2: class T
        utf8(&mut b, "java/lang/Object"); // 3
        b.extend([7, 0, 3]); // 4: class Object
        utf8(&mut b, "f"); // 5
        utf8(&mut b, "()I"); // 6
        utf8(&mut b, "Code"); // 7
        utf8(&mut b, "LineNumberTable"); // 8
        utf8(&mut b, "SourceFile"); // 9
        b.extend(0x0021u16.to_be_bytes());
        b.extend(2u16.to_be_bytes());
        b.extend(4u16.to_be_bytes());
        b.extend(0u16.to_be_bytes()); // interfaces
        b.extend(0u16.to_be_bytes()); // fields
        b.extend(1u16.to_be_bytes()); // methods
        b.extend(0x0008u16.to_be_bytes());
        b.extend(5u16.to_be_bytes());
        b.extend(6u16.to_be_bytes());
        b.extend(1u16.to_be_bytes());
        b.extend(7u16.to_be_bytes());
        let mut line_tables = Vec::new();
        for &entries in tables {
            line_tables.extend(8u16.to_be_bytes());
            line_tables.extend((2 + 4 * u32::from(entries)).to_be_bytes());
            line_tables.extend(entries.to_be_bytes());
            for _ in 0..entries {
                line_tables.extend(0u16.to_be_bytes());
                line_tables.extend(3u16.to_be_bytes());
            }
        }
        let code = [0x10, 7, 0xAC]; // bipush 7; ireturn
        let code_attr_len = 2 + 2 + 4 + code.len() + 2 + 2 + line_tables.len();
        b.extend((code_attr_len as u32).to_be_bytes());
        b.extend(1u16.to_be_bytes());
        b.extend(0u16.to_be_bytes());
        b.extend((code.len() as u32).to_be_bytes());
        b.extend(code);
        b.extend(0u16.to_be_bytes());
        b.extend((tables.len() as u16).to_be_bytes());
        b.extend(line_tables);
        b.extend(1u16.to_be_bytes());
        b.extend(9u16.to_be_bytes());
        b.extend(2u32.to_be_bytes());
        b.extend(1u16.to_be_bytes());
        b
    }

    #[test]
    fn parses_a_small_class() {
        let class = ClassFile::parse(&tiny_class(45, 3)).unwrap();
        assert_eq!(&*class.name, "T");
        assert_eq!(class.super_name.as_deref(), Some("java/lang/Object"));
        assert_eq!(class.source_file.as_deref(), Some("T"));
        let method = &class.methods[0];
        assert_eq!((&*method.name, &*method.descriptor), ("f", "()I"));
        let code = method.code.as_ref().unwrap();
        assert_eq!(code.code, [0x10, 7, 0xAC]);
        assert_eq!(code.line_at(2), Some(3));
    }

    #[test]
    fn line_numbers_take_no_more_room_than_they_hold() {
        // The class memory counts 4 bytes a line number. Lines added one at
        // a time leave a list room for twice as many: 65537 lines from two
        // tables kept room for 131072.
        let class = ClassFile::parse(&class_with_lines(49, 0, &[65535, 2])).unwrap();
        let code = class.methods[0].code.as_ref().unwrap();
        assert_eq!((code.lines.len(), code.lines.capacity()), (65537, 65537));
        assert_eq!(code.line_at(2), Some(3));
    }

    #[test]
    fn methods_that_share_a_descriptor_walk_it_once() {
        // 3000 methods of 9000 parameters each: walked for every method,
        // that is 27 million parameters. A class file of 65535 methods
        // that name a descriptor of 65532 parameters took six minutes.
        let wide = format!("({})V", "I".repeat(9000));
        let other = format!("({})I", "J".repeat(9000));
        let bytes = class_of_methods(&[&wide, &other], 1500);
        take_shapes();
        let class = ClassFile::parse(&bytes).unwrap();
        assert_eq!(class.methods.len(), 3000);
        assert_eq!(take_shapes(), 2, "one walk for each descriptor");
    }

    /// The start of a class file `T` that extends `Object`, with the
    /// pool entries 1 to 4 and then the text entries `texts`, from 5 on.
    fn class_head(texts: &[&str]) -> Vec<u8> {
        let mut b = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 49];
        let utf8 = |b: &mut Vec<u8>, s: &str| {
            b.push(1);
            b.extend((s.len() as u16).to_be_bytes());
            b.extend(s.as_bytes());
        };
        b.extend((5 + texts.len() as u16).to_be_bytes());
        utf8(&mut b, "T"); // 1
        b.extend([7, 0, 1]); // 2: class T
        utf8(&mut b, "java/lang/Object"); // 3
        b.extend([7, 0, 3]); // 4: class Object
        for text in texts {
            utf8(&mut b, text); // 5 and on
        }
        b
    }

    /// A class file `T` that names the interface `I` of `name_len` bytes
    /// `mentions` times.
    fn class_of_interfaces(name_len: usize, mentions: usize) -> Vec<u8> {
        let name = "a".repeat(name_len);
        let mut b = class_head(&[&name]);
        // The class entry for the text at 5 goes after the texts.
        b[8..10].copy_from_slice(&7u16.to_be_bytes());
        b.extend([7, 0, 5]); // 6: class `name`
        b.extend(0x0421u16.to_be_bytes());
        b.extend(2u16.to_be_bytes());
        b.extend(4u16.to_be_bytes());
        b.extend((mentions as u16).to_be_bytes());
        for _ in 0..mentions {
            b.extend(6u16.to_be_bytes());
        }
        b.extend([0; 6]); // no fields, no methods, no attributes
        b
    }

    /// A class file `T` with `per_descriptor` fields named `f` for each of
    /// `descriptors`, which are one pool entry each.
    fn class_of_fields(descriptors: &[&str], per_descriptor: usize) -> Vec<u8> {
        let mut texts = vec!["f"];
        texts.extend(descriptors);
        let mut b = class_head(&texts);
        b.extend(0x0421u16.to_be_bytes());
        b.extend(2u16.to_be_bytes());
        b.extend(4u16.to_be_bytes());
        b.extend(0u16.to_be_bytes()); // interfaces
        b.extend(((descriptors.len() * per_descriptor) as u16).to_be_bytes());
        for i in 0..descriptors.len() {
            for _ in 0..per_descriptor {
                b.extend(0x0001u16.to_be_bytes());
                b.extend(5u16.to_be_bytes());
                b.extend((6 + i as u16).to_be_bytes());
                b.extend(0u16.to_be_bytes()); // attributes
            }
        }
        b.extend([0; 4]); // no methods, no attributes
        b
    }

    #[test]
    fn interfaces_that_name_one_class_share_its_name() {
        // A class file can name one interface 65535 times, and the name can
        // take 65535 bytes. A copy for each mention is four gigabytes.
        let bytes = class_of_interfaces(60_000, 2000);
        let class = ClassFile::parse(&bytes).unwrap();
        assert_eq!(class.interfaces.len(), 2000);
        assert_eq!(class.interfaces[0].len(), 60_000);
        assert!(
            class
                .interfaces
                .iter()
                .all(|name| Arc::ptr_eq(name, &class.interfaces[0]))
        );
    }

    #[test]
    fn fields_that_share_a_descriptor_walk_it_once() {
        // 3000 fields of a descriptor of 60000 bytes: walked for every
        // field, that is 180 million bytes. A class file of 65535 fields
        // took a second.
        let wide = format!("L{};", "a".repeat(60_000));
        let bytes = class_of_fields(&[&wide, "[[J"], 1500);
        take_walks();
        let class = ClassFile::parse(&bytes).unwrap();
        assert_eq!(class.fields.len(), 3000);
        assert_eq!(take_walks(), 2, "one walk for each descriptor");
    }

    #[test]
    fn a_bad_field_descriptor_is_refused_however_many_fields_share_it() {
        let err = ClassFile::parse(&class_of_fields(&["L;", "I"], 3)).unwrap_err();
        assert!(err.0.contains("bad descriptor"), "{}", err.0);
        assert!(ClassFile::parse(&class_of_fields(&["I", "Lx;"], 3)).is_ok());
    }

    #[test]
    fn a_bad_descriptor_is_refused_however_many_methods_share_it() {
        let bytes = class_of_methods(&["(I", "()V"], 3);
        let err = ClassFile::parse(&bytes).unwrap_err();
        assert!(err.0.contains("bad descriptor"), "{}", err.0);
        assert!(ClassFile::parse(&class_of_methods(&["()V"], 3)).is_ok());
    }

    #[test]
    fn rejects_unsupported_versions_and_garbage() {
        assert!(ClassFile::parse(&tiny_class(52, 0)).is_ok());
        let err = ClassFile::parse(&tiny_class(61, 0)).unwrap_err();
        assert_eq!(err.0, "unsupported class file version 61.0");
        assert!(ClassFile::parse(b"PK\x03\x04").is_err());
        let mut truncated = tiny_class(50, 0);
        truncated.truncate(40);
        assert_eq!(
            ClassFile::parse(&truncated).unwrap_err().0,
            "truncated class file"
        );
    }
}
