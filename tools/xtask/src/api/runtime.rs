//! The classes the runtime registers, read back from their
//! [`NativeClass`] descriptions into plain data.
//!
//! Every instance's virtual machine gets the CLDC library of `fastemu-jvm`
//! and the classes of `fastemu_midp::classes`, the second replacing a
//! library class of the same name, as `Vm::new` does. This module builds
//! the same set without starting a machine.

use std::collections::{BTreeMap, HashSet};

use fastemu_jvm::classfile::access;
use fastemu_jvm::{FieldConstant, NativeClass, Value};

/// A field's constant value.
#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    /// A string, as UTF-16 units.
    String(Vec<u16>),
}

/// A field of a runtime class.
#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub desc: String,
    pub flags: u16,
    pub constant: Option<Constant>,
}

/// A method or constructor (`<init>`) of a runtime class.
#[derive(Clone, Debug)]
pub struct Method {
    pub name: String,
    pub desc: String,
    pub flags: u16,
}

/// A class the runtime registers.
#[derive(Clone, Debug)]
pub struct Class {
    /// The internal name, such as `javax/microedition/lcdui/Canvas`.
    pub name: String,
    pub super_name: Option<String>,
    pub interfaces: Vec<String>,
    pub flags: u16,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
}

impl Class {
    pub fn is_interface(&self) -> bool {
        self.flags & access::INTERFACE != 0
    }

    pub fn is_abstract(&self) -> bool {
        self.flags & access::ABSTRACT != 0
    }

    /// The field this class itself declares as `name` with `desc`.
    pub fn field(&self, name: &str, desc: &str) -> Option<&Field> {
        self.fields
            .iter()
            .find(|f| f.name == name && f.desc == desc)
    }

    /// The method this class itself declares as `name` with `desc`.
    pub fn method(&self, name: &str, desc: &str) -> Option<&Method> {
        self.methods
            .iter()
            .find(|m| m.name == name && m.desc == desc)
    }
}

fn constant(c: FieldConstant<'_>) -> Option<Constant> {
    match c {
        FieldConstant::Value(Value::Int(v)) => Some(Constant::Int(v)),
        FieldConstant::Value(Value::Long(v)) => Some(Constant::Long(v)),
        FieldConstant::Value(Value::Float(v)) => Some(Constant::Float(v)),
        FieldConstant::Value(Value::Double(v)) => Some(Constant::Double(v)),
        FieldConstant::String(s) => Some(Constant::String(s.to_vec())),
        FieldConstant::Value(_) => None,
    }
}

impl From<&NativeClass> for Class {
    fn from(native: &NativeClass) -> Self {
        Self {
            name: native.name().to_owned(),
            super_name: native.super_name().map(str::to_owned),
            interfaces: native.interfaces().map(str::to_owned).collect(),
            flags: native.flags(),
            fields: native
                .fields()
                .iter()
                .map(|f| Field {
                    name: f.name().to_owned(),
                    desc: f.descriptor().to_owned(),
                    flags: f.flags(),
                    constant: f.constant().and_then(constant),
                })
                .collect(),
            methods: native
                .methods()
                .iter()
                .map(|m| Method {
                    name: m.name().to_owned(),
                    desc: m.descriptor().to_owned(),
                    flags: m.flags(),
                })
                .collect(),
        }
    }
}

/// Every class an instance's virtual machine can load from the runtime,
/// by internal name.
pub struct Runtime {
    pub classes: BTreeMap<String, Class>,
}

impl Runtime {
    /// The classes of the CLDC library and of the MIDP runtime.
    pub fn load() -> Self {
        let mut natives = fastemu_jvm::library();
        natives.extend(fastemu_midp::classes());
        Self::from_natives(&natives)
    }

    /// The classes of `natives`; a later class replaces an earlier one of
    /// the same name.
    pub fn from_natives(natives: &[NativeClass]) -> Self {
        let classes = natives
            .iter()
            .map(|n| (n.name().to_owned(), Class::from(n)))
            .collect();
        Self { classes }
    }

    pub fn get(&self, name: &str) -> Option<&Class> {
        self.classes.get(name)
    }

    /// The class and its superclasses, nearest first.
    pub fn superclasses<'a>(&'a self, name: &str) -> Vec<&'a Class> {
        let mut chain = Vec::new();
        let mut next = self.get(name);
        while let Some(class) = next {
            if chain.iter().any(|c: &&Class| c.name == class.name) {
                break;
            }
            chain.push(class);
            next = class.super_name.as_deref().and_then(|s| self.get(s));
        }
        chain
    }

    /// Every interface the class implements, directly or through its
    /// superclasses and superinterfaces, each once.
    pub fn all_interfaces<'a>(&'a self, name: &str) -> Vec<&'a Class> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut pending: Vec<&str> = self
            .superclasses(name)
            .iter()
            .flat_map(|c| c.interfaces.iter().map(String::as_str))
            .collect();
        while let Some(next) = pending.pop() {
            if !seen.insert(next) {
                continue;
            }
            if let Some(class) = self.get(next) {
                out.push(class);
                pending.extend(class.interfaces.iter().map(String::as_str));
            }
        }
        out
    }

    /// Whether `name` is `ancestor` or extends or implements it.
    pub fn is_subtype(&self, name: &str, ancestor: &str) -> bool {
        self.superclasses(name).iter().any(|c| c.name == ancestor)
            || self.all_interfaces(name).iter().any(|c| c.name == ancestor)
    }

    /// The field a `getfield` or `getstatic` of `class.name:desc` links
    /// to: the class, its superinterfaces, then its superclass, as JVMS
    /// 5.4.3.2 orders the search.
    pub fn resolve_field<'a>(&'a self, class: &str, name: &str, desc: &str) -> Option<&'a Field> {
        let start = self.get(class)?;
        if let Some(f) = start.field(name, desc) {
            return Some(f);
        }
        for iface in &start.interfaces {
            if let Some(f) = self.resolve_field(iface, name, desc) {
                return Some(f);
            }
        }
        start
            .super_name
            .as_deref()
            .and_then(|s| self.resolve_field(s, name, desc))
    }

    /// The method a call of `class.name desc` links to: the class and its
    /// superclasses first, then every superinterface.
    pub fn resolve_method<'a>(&'a self, class: &str, name: &str, desc: &str) -> Option<&'a Method> {
        let mut classes = self.superclasses(class);
        if self.get(class).is_some_and(Class::is_interface)
            && let Some(object) = self.get("java/lang/Object")
        {
            classes.push(object);
        }
        classes
            .into_iter()
            .chain(self.all_interfaces(class))
            .find_map(|c| c.method(name, desc))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> Runtime {
        Runtime::from_natives(&[
            NativeClass::new("java/lang/Object")
                .method("hashCode", "()I", |_, _| Ok(Value::Int(0))),
            NativeClass::new("java/lang/Throwable"),
            NativeClass::new("java/lang/Exception").extends("java/lang/Throwable"),
            NativeClass::new("java/lang/RuntimeException").extends("java/lang/Exception"),
            NativeClass::new("java/io/IOException").extends("java/lang/Exception"),
            NativeClass::new("a/Oops").extends("java/lang/RuntimeException"),
            NativeClass::interface("a/Shape")
                .constant("SIDES", "I", Value::Int(4))
                .abstract_method("area", "()I"),
            NativeClass::new("a/Square")
                .implements("a/Shape")
                .method("area", "()I", |_, _| Ok(Value::Int(1))),
            NativeClass::new("a/Big").extends("a/Square"),
        ])
    }

    #[test]
    fn members_resolve_through_the_hierarchy() {
        let rt = runtime();
        assert!(rt.resolve_method("a/Big", "area", "()I").is_some());
        assert!(rt.resolve_method("a/Big", "hashCode", "()I").is_some());
        assert!(rt.resolve_method("a/Shape", "hashCode", "()I").is_some());
        assert!(rt.resolve_method("a/Big", "perimeter", "()I").is_none());
        let sides = rt.resolve_field("a/Big", "SIDES", "I").unwrap();
        assert_eq!(sides.constant, Some(Constant::Int(4)));
    }

    #[test]
    fn subtypes_follow_classes_and_interfaces() {
        let rt = runtime();
        assert!(rt.is_subtype("a/Oops", "java/lang/Throwable"));
        assert!(!rt.is_subtype("a/Square", "java/lang/Throwable"));
        assert!(rt.is_subtype("a/Big", "a/Shape"));
    }
}
