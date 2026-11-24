//! The constant pool (JVMS 4.4) and modified UTF-8.

use std::sync::Arc;

use super::FormatError;
use super::reader::Reader;

/// A `CONSTANT_Utf8` entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Utf8 {
    /// The text. Unpaired surrogates, which a Rust string cannot hold,
    /// become U+FFFD here; `exact` then keeps the real UTF-16.
    pub text: Arc<str>,
    /// The exact UTF-16 when `text` is not a faithful copy.
    pub exact: Option<Box<[u16]>>,
}

impl Utf8 {
    /// The entry as UTF-16, exactly as the class file spells it.
    pub fn utf16(&self) -> Vec<u16> {
        match &self.exact {
            Some(exact) => exact.to_vec(),
            None => self.text.encode_utf16().collect(),
        }
    }
}

/// One constant pool entry.
#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    /// Index 0, and the slot after a `Long` or `Double`.
    Unused,
    Utf8(Utf8),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class(u16),
    String(u16),
    FieldRef {
        class: u16,
        name_and_type: u16,
    },
    MethodRef {
        class: u16,
        name_and_type: u16,
    },
    InterfaceMethodRef {
        class: u16,
        name_and_type: u16,
    },
    NameAndType {
        name: u16,
        descriptor: u16,
    },
    MethodHandle {
        kind: u8,
        reference: u16,
    },
    MethodType(u16),
    /// `CONSTANT_Dynamic` and `CONSTANT_InvokeDynamic`.
    Dynamic {
        bootstrap: u16,
        name_and_type: u16,
    },
}

/// A class file's constant pool.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ConstantPool {
    entries: Vec<Constant>,
}

/// A field or method reference with its names looked up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberRef<'a> {
    pub class: &'a str,
    pub name: &'a str,
    pub descriptor: &'a str,
}

impl ConstantPool {
    pub(crate) fn parse(r: &mut Reader<'_>) -> Result<Self, FormatError> {
        let count = r.u16()? as usize;
        let mut entries = Vec::with_capacity(count);
        entries.push(Constant::Unused);
        while entries.len() < count {
            let tag = r.u8()?;
            let entry = match tag {
                1 => {
                    let len = r.u16()? as usize;
                    Constant::Utf8(decode_modified_utf8(r.bytes(len)?))
                }
                3 => Constant::Integer(r.u32()? as i32),
                4 => Constant::Float(f32::from_bits(r.u32()?)),
                5 => Constant::Long(r.u64()? as i64),
                6 => Constant::Double(f64::from_bits(r.u64()?)),
                7 => Constant::Class(r.u16()?),
                8 => Constant::String(r.u16()?),
                9 => Constant::FieldRef {
                    class: r.u16()?,
                    name_and_type: r.u16()?,
                },
                10 => Constant::MethodRef {
                    class: r.u16()?,
                    name_and_type: r.u16()?,
                },
                11 => Constant::InterfaceMethodRef {
                    class: r.u16()?,
                    name_and_type: r.u16()?,
                },
                12 => Constant::NameAndType {
                    name: r.u16()?,
                    descriptor: r.u16()?,
                },
                15 => Constant::MethodHandle {
                    kind: r.u8()?,
                    reference: r.u16()?,
                },
                16 => Constant::MethodType(r.u16()?),
                17 | 18 => Constant::Dynamic {
                    bootstrap: r.u16()?,
                    name_and_type: r.u16()?,
                },
                // Module and Package entries only appear in module-info.
                19 | 20 => Constant::Class(r.u16()?),
                _ => {
                    return Err(FormatError::new(format!(
                        "unknown constant pool tag {tag} at index {}",
                        entries.len()
                    )));
                }
            };
            let wide = matches!(entry, Constant::Long(_) | Constant::Double(_));
            entries.push(entry);
            if wide {
                entries.push(Constant::Unused);
            }
        }
        if entries.len() > count {
            return Err(FormatError::new(
                "a long or double constant overruns the pool",
            ));
        }
        Ok(Self { entries })
    }

    /// A pool from entries, index 0 first (unused).
    pub fn from_entries(entries: Vec<Constant>) -> Self {
        Self { entries }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.len() <= 1
    }

    pub fn get(&self, index: u16) -> Option<&Constant> {
        self.entries.get(index as usize)
    }

    fn bad(index: u16, what: &str) -> FormatError {
        FormatError::new(format!("constant pool entry {index} is not {what}"))
    }

    pub fn utf8_entry(&self, index: u16) -> Result<&Utf8, FormatError> {
        match self.get(index) {
            Some(Constant::Utf8(u)) => Ok(u),
            _ => Err(Self::bad(index, "a UTF-8 string")),
        }
    }

    pub fn utf8(&self, index: u16) -> Result<&str, FormatError> {
        Ok(&self.utf8_entry(index)?.text)
    }

    pub fn utf8_arc(&self, index: u16) -> Result<Arc<str>, FormatError> {
        Ok(Arc::clone(&self.utf8_entry(index)?.text))
    }

    /// The internal name a `CONSTANT_Class` entry names (`java/lang/String`
    /// or an array descriptor such as `[I`).
    pub fn class_name(&self, index: u16) -> Result<&str, FormatError> {
        match self.get(index) {
            Some(Constant::Class(name)) => self.utf8(*name),
            _ => Err(Self::bad(index, "a class")),
        }
    }

    pub fn name_and_type(&self, index: u16) -> Result<(&str, &str), FormatError> {
        match self.get(index) {
            Some(Constant::NameAndType { name, descriptor }) => {
                Ok((self.utf8(*name)?, self.utf8(*descriptor)?))
            }
            _ => Err(Self::bad(index, "a name and type")),
        }
    }

    /// A `Fieldref`, `Methodref` or `InterfaceMethodref` entry.
    pub fn member_ref(&self, index: u16) -> Result<MemberRef<'_>, FormatError> {
        match self.get(index) {
            Some(
                Constant::FieldRef {
                    class,
                    name_and_type,
                }
                | Constant::MethodRef {
                    class,
                    name_and_type,
                }
                | Constant::InterfaceMethodRef {
                    class,
                    name_and_type,
                },
            ) => {
                let (name, descriptor) = self.name_and_type(*name_and_type)?;
                Ok(MemberRef {
                    class: self.class_name(*class)?,
                    name,
                    descriptor,
                })
            }
            _ => Err(Self::bad(index, "a field or method reference")),
        }
    }

    /// The UTF-16 text of a `CONSTANT_String` entry.
    pub fn string(&self, index: u16) -> Result<Vec<u16>, FormatError> {
        match self.get(index) {
            Some(Constant::String(utf8)) => Ok(self.utf8_entry(*utf8)?.utf16()),
            _ => Err(Self::bad(index, "a string")),
        }
    }
}

/// Decodes the modified UTF-8 of class files (JVMS 4.4.7): U+0000 as two
/// bytes, characters outside the BMP as two encoded surrogates. Malformed
/// sequences become U+FFFD rather than failing, so a sloppy obfuscator's
/// output still loads.
pub fn decode_modified_utf8(bytes: &[u8]) -> Utf8 {
    let mut units = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let cont = |b: Option<&u8>| {
        b.filter(|&&b| b & 0xC0 == 0x80)
            .map(|&b| u16::from(b & 0x3F))
    };
    while i < bytes.len() {
        let b = bytes[i];
        if b < 0x80 {
            units.push(u16::from(b));
            i += 1;
        } else if b & 0xE0 == 0xC0 {
            match cont(bytes.get(i + 1)) {
                Some(c1) => {
                    units.push((u16::from(b & 0x1F) << 6) | c1);
                    i += 2;
                }
                None => {
                    units.push(0xFFFD);
                    i += 1;
                }
            }
        } else if b & 0xF0 == 0xE0 {
            match (cont(bytes.get(i + 1)), cont(bytes.get(i + 2))) {
                (Some(c1), Some(c2)) => {
                    units.push((u16::from(b & 0x0F) << 12) | (c1 << 6) | c2);
                    i += 3;
                }
                _ => {
                    units.push(0xFFFD);
                    i += 1;
                }
            }
        } else {
            units.push(0xFFFD);
            i += 1;
        }
    }
    match String::from_utf16(&units) {
        Ok(text) => Utf8 {
            text: text.into(),
            exact: None,
        },
        Err(_) => Utf8 {
            text: String::from_utf16_lossy(&units).into(),
            exact: Some(units.into()),
        },
    }
}

/// Encodes UTF-16 as modified UTF-8, the inverse of
/// [`decode_modified_utf8`].
pub fn encode_modified_utf8(units: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(units.len());
    for &u in units {
        match u {
            0x0001..=0x007F => out.push(u as u8),
            0x0000 | 0x0080..=0x07FF => {
                out.push(0xC0 | (u >> 6) as u8);
                out.push(0x80 | (u & 0x3F) as u8);
            }
            _ => {
                out.push(0xE0 | (u >> 12) as u8);
                out.push(0x80 | ((u >> 6) & 0x3F) as u8);
                out.push(0x80 | (u & 0x3F) as u8);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modified_utf8_round_trips() {
        let units: Vec<u16> = "a\u{0}\u{e9}\u{20ac}\u{1F600}".encode_utf16().collect();
        let bytes = encode_modified_utf8(&units);
        assert!(!bytes.contains(&0), "U+0000 never encodes as a zero byte");
        let decoded = decode_modified_utf8(&bytes);
        assert_eq!(decoded.utf16(), units);
        assert!(decoded.exact.is_none());
        assert_eq!(&*decoded.text, "a\u{0}\u{e9}\u{20ac}\u{1F600}");
    }

    #[test]
    fn lone_surrogates_keep_their_exact_text() {
        let units = [0x41, 0xD800, 0x42];
        let decoded = decode_modified_utf8(&encode_modified_utf8(&units));
        assert_eq!(decoded.utf16(), units);
        assert_eq!(&*decoded.text, "A\u{FFFD}B");
    }

    #[test]
    fn malformed_bytes_become_replacements() {
        let decoded = decode_modified_utf8(&[0x41, 0xC3, 0xFF]);
        assert_eq!(&*decoded.text, "A\u{FFFD}\u{FFFD}");
    }
}
