//! Runtime classes, fields and methods, and the table that loads them.
//!
//! Classes from bytecode and classes implemented in Rust share one table
//! and one representation, so a game class can extend a native class and
//! override its methods, and native code reaches game overrides through
//! the same virtual dispatch the interpreter uses.

mod link;
mod table;

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::classfile::{CodeAttr, ConstantPool, access};
use crate::descriptor::Kind;
use crate::heap::Payload;
use crate::interp::{Code, Resolved};
use crate::native::NativeFn;
use crate::value::Value;

pub(crate) use table::{
    ClassTable, InitState, LoadError, MAX_CLASS_NAME, MAX_LOADABLE_NAME, short,
};

/// Identifies a loaded class within one virtual machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClassId(pub(crate) u32);

impl ClassId {
    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

/// A method name and descriptor pair, interned per virtual machine. Two
/// methods with one selector override each other.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Selector(pub(crate) u32);

/// The element type of an array.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ArrayType {
    Boolean,
    Byte,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
}

impl ArrayType {
    /// The type `newarray`'s operand names (JVMS table 6.5.newarray-A).
    pub fn from_atype(atype: u8) -> Option<Self> {
        Some(match atype {
            4 => ArrayType::Boolean,
            5 => ArrayType::Char,
            6 => ArrayType::Float,
            7 => ArrayType::Double,
            8 => ArrayType::Byte,
            9 => ArrayType::Short,
            10 => ArrayType::Int,
            11 => ArrayType::Long,
            _ => return None,
        })
    }

    /// The descriptor character, such as `I` for `int`.
    pub fn descriptor(self) -> char {
        match self {
            ArrayType::Boolean => 'Z',
            ArrayType::Byte => 'B',
            ArrayType::Char => 'C',
            ArrayType::Short => 'S',
            ArrayType::Int => 'I',
            ArrayType::Long => 'J',
            ArrayType::Float => 'F',
            ArrayType::Double => 'D',
        }
    }

    pub(crate) fn from_descriptor(c: u8) -> Option<Self> {
        Some(match c {
            b'Z' => ArrayType::Boolean,
            b'B' => ArrayType::Byte,
            b'C' => ArrayType::Char,
            b'S' => ArrayType::Short,
            b'I' => ArrayType::Int,
            b'J' => ArrayType::Long,
            b'F' => ArrayType::Float,
            b'D' => ArrayType::Double,
            _ => return None,
        })
    }

    /// Bytes per element, as a phone stores them.
    pub(crate) fn elem_bytes(self) -> usize {
        match self {
            ArrayType::Boolean | ArrayType::Byte => 1,
            ArrayType::Char | ArrayType::Short => 2,
            ArrayType::Int | ArrayType::Float => 4,
            ArrayType::Long | ArrayType::Double => 8,
        }
    }
}

/// What sort of class this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ClassKind {
    /// An ordinary or abstract class.
    Plain,
    Interface,
    /// An array of primitives.
    PrimArray(ArrayType),
    /// An array of references; the element class (itself an array class
    /// for multi-dimensional arrays).
    RefArray(ClassId),
}

/// A static field's value from a `ConstantValue` attribute, applied when
/// the class initializes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ConstValue {
    Value(Value),
    String(Arc<[u16]>),
}

/// A declared field.
#[derive(Debug)]
pub(crate) struct Field {
    pub(crate) name: Arc<str>,
    pub(crate) desc: Arc<str>,
    pub(crate) access: u16,
    pub(crate) kind: Kind,
    /// The instance slot, or for static fields the index in the virtual
    /// machine's static storage.
    pub(crate) slot: u32,
    pub(crate) constant: Option<ConstValue>,
}

impl Field {
    pub(crate) fn is_static(&self) -> bool {
        self.access & access::STATIC != 0
    }
}

/// Makes the payload of a new object of a native class.
pub type PayloadFactory = Arc<dyn Fn() -> Box<dyn Payload> + Send + Sync>;

/// A class's runtime constant pool: the parsed pool and a cache of what
/// each entry resolved to.
pub(crate) struct RtPool {
    pub(crate) class: ClassId,
    pub(crate) cp: ConstantPool,
    pub(crate) cache: Box<[OnceLock<Resolved>]>,
    /// `ACC_SUPER`: `invokespecial` selects from the superclass.
    pub(crate) super_semantics: bool,
}

/// A method's bytecode, decoded on first call.
pub(crate) struct Bytecode {
    pub(crate) attr: CodeAttr,
    pub(crate) pool: Arc<RtPool>,
    pub(crate) decoded: OnceLock<Result<Code, String>>,
}

pub(crate) enum MethodBody {
    Bytecode(Box<Bytecode>),
    Native(NativeFn),
    Abstract,
    /// A `native` method of a class file, which FastEmu cannot run.
    Unlinked,
}

/// A declared method.
pub(crate) struct Method {
    pub(crate) class: ClassId,
    pub(crate) class_name: Arc<str>,
    pub(crate) name: Arc<str>,
    pub(crate) desc: Arc<str>,
    pub(crate) access: u16,
    pub(crate) selector: Selector,
    /// Argument slots, `this` included.
    pub(crate) arg_slots: u16,
    pub(crate) ret: Kind,
    pub(crate) body: MethodBody,
    pub(crate) source_file: Option<Arc<str>>,
}

impl Method {
    pub(crate) fn is_static(&self) -> bool {
        self.access & access::STATIC != 0
    }

    pub(crate) fn is_synchronized(&self) -> bool {
        self.access & access::SYNCHRONIZED != 0
    }

    pub(crate) fn is_private(&self) -> bool {
        self.access & access::PRIVATE != 0
    }

    pub(crate) fn is_abstract(&self) -> bool {
        matches!(self.body, MethodBody::Abstract)
    }

    /// The decoded bytecode, decoding it on the first call.
    pub(crate) fn code(&self) -> Option<Result<&Code, &str>> {
        let MethodBody::Bytecode(bc) = &self.body else {
            return None;
        };
        let decoded = bc
            .decoded
            .get_or_init(|| crate::interp::decode(&bc.attr, &bc.pool.cp, self.arg_slots));
        Some(match decoded {
            Ok(code) => Ok(code),
            Err(e) => Err(e.as_str()),
        })
    }

    /// The runtime constant pool of the method's class.
    pub(crate) fn pool(&self) -> Option<&Arc<RtPool>> {
        match &self.body {
            MethodBody::Bytecode(bc) => Some(&bc.pool),
            _ => None,
        }
    }

    /// The source line of instruction `insn`, for stack traces.
    pub(crate) fn line_of(&self, insn: usize) -> Option<u16> {
        let MethodBody::Bytecode(bc) = &self.body else {
            return None;
        };
        let pc = match bc.decoded.get() {
            Some(Ok(code)) => *code.pcs.get(insn)?,
            _ => return None,
        };
        bc.attr.line_at(pc)
    }

    /// `Class.name(desc)` as diagnostics and errors print it.
    pub(crate) fn describe(&self) -> String {
        format!("{}.{}{}", self.class_name, self.name, self.desc)
    }
}

/// A loaded and linked class.
pub(crate) struct Class {
    pub(crate) id: ClassId,
    pub(crate) name: Arc<str>,
    pub(crate) access: u16,
    pub(crate) kind: ClassKind,
    pub(crate) superclass: Option<ClassId>,
    pub(crate) interfaces: Box<[ClassId]>,
    /// Every interface this class implements, directly or inherited,
    /// sorted.
    pub(crate) all_interfaces: Box<[ClassId]>,
    /// The superclass chain from `java/lang/Object` to this class.
    pub(crate) supers: Box<[ClassId]>,
    /// The most classes on any path from this class up to
    /// `java/lang/Object` through superclasses and interfaces, both ends
    /// included.
    pub(crate) height: u16,
    pub(crate) fields: Box<[Field]>,
    /// The starting values of every instance field, inherited ones first.
    pub(crate) template: Box<[Value]>,
    pub(crate) methods: Box<[Arc<Method>]>,
    pub(crate) vtable: Box<[Arc<Method>]>,
    pub(crate) vmap: HashMap<Selector, u32>,
    pub(crate) payload: Option<PayloadFactory>,
    /// The class of arrays of this class, once something asked for it.
    pub(crate) array_type: OnceLock<ClassId>,
}

impl Class {
    pub(crate) fn is_interface(&self) -> bool {
        self.kind == ClassKind::Interface
    }

    pub(crate) fn is_array(&self) -> bool {
        matches!(self.kind, ClassKind::PrimArray(_) | ClassKind::RefArray(_))
    }

    pub(crate) fn is_abstract(&self) -> bool {
        self.access & access::ABSTRACT != 0
    }

    pub(crate) fn depth(&self) -> usize {
        self.supers.len() - 1
    }

    /// The method this class declares with `name` and `desc`.
    pub(crate) fn declared_method(&self, name: &str, desc: &str) -> Option<&Arc<Method>> {
        self.methods
            .iter()
            .find(|m| &*m.name == name && &*m.desc == desc)
    }

    pub(crate) fn declared_field(&self, name: &str, desc: &str) -> Option<&Field> {
        self.fields
            .iter()
            .find(|f| &*f.name == name && &*f.desc == desc)
    }

    /// The vtable entry for `selector`, the target of a virtual or
    /// interface call on an instance of this class.
    #[inline]
    pub(crate) fn virtual_method(&self, selector: Selector) -> Option<&Arc<Method>> {
        self.vmap
            .get(&selector)
            .map(|&index| &self.vtable[index as usize])
    }

    /// The package part of the name, for package-private access.
    pub(crate) fn package(&self) -> &str {
        package_of(&self.name)
    }
}

pub(crate) fn package_of(name: &str) -> &str {
    name.rfind('/').map_or("", |i| &name[..i])
}
