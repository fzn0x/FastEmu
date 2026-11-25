//! Linking: turning a class file or a native class definition into a
//! runtime class with its field layout, vtable and interface map.

use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use super::table::{ClassTable, LoadError, MAX_HIERARCHY_DEPTH, too_deep};
use super::{
    Bytecode, Class, ClassId, ClassKind, ConstValue, Field, Method, MethodBody, PayloadFactory,
    RtPool, package_of,
};
use crate::classfile::{ClassFile, CodeAttr, Constant, ConstantPool, access};
use crate::descriptor::{Kind, MethodDescriptor, is_field_descriptor, kind_of};
use crate::native::{NativeClass, NativeFn};
use crate::value::Value;

/// A class before linking, from either source.
pub(super) struct ClassDef {
    pub(super) name: Arc<str>,
    pub(super) access: u16,
    pub(super) super_name: Option<Arc<str>>,
    pub(super) interfaces: Vec<Arc<str>>,
    pub(super) fields: Vec<FieldDef>,
    pub(super) methods: Vec<MethodDef>,
    pub(super) source_file: Option<Arc<str>>,
    pub(super) payload: Option<PayloadFactory>,
    /// The constant pool of a class file, and whether `ACC_SUPER` is set.
    pub(super) pool: Option<(ConstantPool, bool)>,
}

pub(super) struct FieldDef {
    pub(super) name: Arc<str>,
    pub(super) desc: Arc<str>,
    pub(super) access: u16,
    pub(super) constant: Option<ConstValue>,
}

pub(super) enum MethodSrc {
    Code(CodeAttr),
    Native(NativeFn),
    Abstract,
    Unlinked,
}

pub(super) struct MethodDef {
    pub(super) name: Arc<str>,
    pub(super) desc: Arc<str>,
    pub(super) access: u16,
    pub(super) src: MethodSrc,
}

fn format_error(class: &str, what: impl std::fmt::Display) -> LoadError {
    LoadError::new("java/lang/ClassFormatError", format!("{class}: {what}"))
}

pub(super) fn from_native(native: NativeClass) -> Result<ClassDef, LoadError> {
    let name: Arc<str> = native.name.into();
    let fields = native
        .fields
        .into_iter()
        .map(|f| {
            if !is_field_descriptor(&f.desc) {
                return Err(format_error(
                    &name,
                    format!("bad descriptor of field {}", f.name),
                ));
            }
            Ok(FieldDef {
                name: f.name.into(),
                desc: f.desc.into(),
                access: f.access,
                constant: f.constant,
            })
        })
        .collect::<Result<_, _>>()?;
    let methods = native
        .methods
        .into_iter()
        .map(|m| MethodDef {
            name: m.name.into(),
            desc: m.desc.into(),
            access: m.access,
            src: match m.body {
                Some(f) => MethodSrc::Native(f),
                None => MethodSrc::Abstract,
            },
        })
        .collect();
    Ok(ClassDef {
        super_name: native.super_name.map(Into::into),
        interfaces: native.interfaces.into_iter().map(Into::into).collect(),
        access: native.access,
        fields,
        methods,
        source_file: None,
        payload: native.payload,
        pool: None,
        name,
    })
}

pub(super) fn from_class_file(file: ClassFile) -> Result<ClassDef, LoadError> {
    let cp = &file.constant_pool;
    let mut fields = Vec::with_capacity(file.fields.len());
    for f in file.fields {
        let constant = match f.constant_value {
            Some(index) if f.access & access::STATIC != 0 => {
                Some(constant_value(cp, index, &f.descriptor).ok_or_else(|| {
                    format_error(&file.name, format!("bad constant value of {}", f.name))
                })?)
            }
            _ => None,
        };
        fields.push(FieldDef {
            name: f.name,
            desc: f.descriptor,
            access: f.access,
            constant,
        });
    }
    let mut methods = Vec::with_capacity(file.methods.len());
    for m in file.methods {
        let src = if m.access & access::NATIVE != 0 {
            MethodSrc::Unlinked
        } else if m.access & access::ABSTRACT != 0 {
            MethodSrc::Abstract
        } else {
            match m.code {
                Some(code) => MethodSrc::Code(code),
                None => {
                    return Err(format_error(
                        &file.name,
                        format!("method {}{} has no code", m.name, m.descriptor),
                    ));
                }
            }
        };
        methods.push(MethodDef {
            name: m.name,
            desc: m.descriptor,
            access: m.access,
            src,
        });
    }
    let super_semantics = file.access & access::SUPER != 0;
    Ok(ClassDef {
        name: file.name,
        access: file.access,
        super_name: file.super_name,
        interfaces: file.interfaces,
        fields,
        methods,
        source_file: file.source_file,
        payload: None,
        pool: Some((file.constant_pool, super_semantics)),
    })
}

fn constant_value(cp: &ConstantPool, index: u16, desc: &str) -> Option<ConstValue> {
    Some(ConstValue::Value(match (cp.get(index)?, kind_of(desc)) {
        (Constant::Integer(v), Kind::Int) => Value::Int(*v),
        (Constant::Long(v), Kind::Long) => Value::Long(*v),
        (Constant::Float(v), Kind::Float) => Value::Float(*v),
        (Constant::Double(v), Kind::Double) => Value::Double(*v),
        (Constant::String(_), Kind::Ref) => {
            return Some(ConstValue::String(cp.string(index).ok()?.into()));
        }
        _ => return None,
    }))
}

/// Whether `m`, declared in class `package`, overrides `sup` (JVMS 5.4.5).
fn overrides(m_package: &str, sup: &Method) -> bool {
    if sup.is_private() || sup.is_static() {
        return false;
    }
    sup.access & (access::PUBLIC | access::PROTECTED) != 0
        || package_of(&sup.class_name) == m_package
}

impl ClassTable {
    /// Links `def`, loading its superclass and interfaces first.
    pub(super) fn define(&mut self, def: ClassDef) -> Result<ClassId, LoadError> {
        let is_interface = def.access & access::INTERFACE != 0;
        let superclass = match &def.super_name {
            Some(name) => {
                let id = self.load(name)?;
                let sup = self.get(id);
                if sup.is_interface() || sup.is_array() {
                    return Err(LoadError::new(
                        "java/lang/IncompatibleClassChangeError",
                        format!("class {} has interface {} as super class", def.name, name),
                    ));
                }
                Some(Arc::clone(sup))
            }
            None if &*def.name == "java/lang/Object" => None,
            None => return Err(format_error(&def.name, "no superclass")),
        };
        // The classes on the longest path above this one. A class loaded
        // after its whole chain has no nesting for the load check to
        // count, but initializing it still recurses once per superclass,
        // and a chain of interfaces loaded one at a time makes each one's
        // list of interfaces longer.
        let mut above = superclass.as_ref().map_or(0, |s| usize::from(s.height));
        if above >= MAX_HIERARCHY_DEPTH {
            return Err(too_deep(&def.name));
        }
        let mut interfaces = Vec::with_capacity(def.interfaces.len());
        for name in &def.interfaces {
            let id = self.load(name)?;
            if !self.get(id).is_interface() {
                return Err(LoadError::new(
                    "java/lang/IncompatibleClassChangeError",
                    format!(
                        "class {} can not implement {name}, because it is not an interface",
                        def.name
                    ),
                ));
            }
            above = above.max(usize::from(self.get(id).height));
            if above >= MAX_HIERARCHY_DEPTH {
                return Err(too_deep(&def.name));
            }
            interfaces.push(id);
        }
        let height = above as u16 + 1;
        let id = self.next_id();
        let mut all_interfaces: Vec<ClassId> = superclass
            .as_ref()
            .map(|s| s.all_interfaces.to_vec())
            .unwrap_or_default();
        for &iface in &interfaces {
            all_interfaces.push(iface);
            all_interfaces.extend(self.get(iface).all_interfaces.iter().copied());
        }
        all_interfaces.sort_unstable();
        all_interfaces.dedup();
        let supers: Box<[ClassId]> = match &superclass {
            Some(s) => s.supers.iter().copied().chain([id]).collect(),
            None => Box::new([id]),
        };

        // Everything that can fail comes before anything is added to the
        // table, so a class that does not link leaves nothing behind: no
        // static storage, no selectors, no share of the class memory.
        let mut descriptors = Vec::with_capacity(def.methods.len());
        for m in &def.methods {
            let Some(parsed) = MethodDescriptor::parse(&m.desc) else {
                return Err(format_error(
                    &def.name,
                    format!("bad descriptor of method {}{}", m.name, m.desc),
                ));
            };
            if matches!(m.src, MethodSrc::Code(_)) && def.pool.is_none() {
                return Err(format_error(&def.name, "bytecode without a constant pool"));
            }
            descriptors.push(parsed);
        }
        // The classes a game can make count against its class memory; the
        // library's, which are fixed, do not.
        if def.pool.is_some() {
            let bytes =
                self.class_file_bytes(&def, superclass.as_deref(), &all_interfaces, supers.len());
            self.budget.charge(bytes)?;
        }

        // Fields: instance slots after the inherited ones, statics in the
        // table's static storage.
        let mut template: Vec<Value> = superclass
            .as_ref()
            .map(|s| s.template.to_vec())
            .unwrap_or_default();
        let mut fields = Vec::with_capacity(def.fields.len());
        for f in def.fields {
            let kind = kind_of(&f.desc);
            let slot = if f.access & access::STATIC != 0 {
                self.statics.push(kind.default_value());
                self.statics.len() - 1
            } else {
                template.push(kind.default_value());
                template.len() - 1
            };
            fields.push(Field {
                name: f.name,
                desc: f.desc,
                access: f.access,
                kind,
                slot: slot as u32,
                constant: f.constant,
            });
        }

        let pool = def.pool.map(|(cp, super_semantics)| {
            let cache = (0..cp.len()).map(|_| OnceLock::new()).collect();
            Arc::new(RtPool {
                class: id,
                cp,
                cache,
                super_semantics,
            })
        });

        let mut methods = Vec::with_capacity(def.methods.len());
        for (m, parsed) in def.methods.into_iter().zip(descriptors) {
            let is_static = m.access & access::STATIC != 0;
            let arg_slots = parsed.param_slots() + usize::from(!is_static);
            let body = match m.src {
                MethodSrc::Code(attr) => {
                    let pool = pool.as_ref().expect("a method with code has a pool");
                    MethodBody::Bytecode(Box::new(Bytecode {
                        attr,
                        pool: Arc::clone(pool),
                        decoded: OnceLock::new(),
                    }))
                }
                MethodSrc::Native(f) => MethodBody::Native(f),
                MethodSrc::Abstract => MethodBody::Abstract,
                MethodSrc::Unlinked => MethodBody::Unlinked,
            };
            methods.push(Arc::new(Method {
                class: id,
                class_name: Arc::clone(&def.name),
                selector: self.selector(&m.name, &m.desc),
                name: m.name,
                desc: m.desc,
                access: m.access,
                arg_slots: arg_slots as u16,
                ret: parsed.ret_kind(),
                body,
                source_file: def.source_file.clone(),
            }));
        }

        let (vtable, vmap) = if is_interface {
            (Vec::new(), HashMap::new())
        } else {
            self.build_vtable(&def.name, superclass.as_deref(), &methods, &all_interfaces)
        };

        let class = Class {
            id,
            access: def.access,
            kind: if is_interface {
                ClassKind::Interface
            } else {
                ClassKind::Plain
            },
            superclass: superclass.as_ref().map(|s| s.id),
            interfaces: interfaces.into(),
            all_interfaces: all_interfaces.into(),
            supers,
            height,
            fields: fields.into(),
            template: template.into(),
            methods: methods.into(),
            vtable: vtable.into(),
            vmap,
            payload: def
                .payload
                .or_else(|| superclass.as_ref().and_then(|s| s.payload.clone())),
            name: def.name,
            array_type: OnceLock::new(),
        };
        Ok(self.insert(class, false))
    }

    fn build_vtable(
        &self,
        name: &str,
        superclass: Option<&Class>,
        methods: &[Arc<Method>],
        all_interfaces: &[ClassId],
    ) -> (Vec<Arc<Method>>, HashMap<super::Selector, u32>) {
        let mut vtable: Vec<Arc<Method>> =
            superclass.map(|s| s.vtable.to_vec()).unwrap_or_default();
        let mut vmap = superclass.map(|s| s.vmap.clone()).unwrap_or_default();
        let package = package_of(name);
        for m in methods {
            if m.is_static() || m.is_private() || m.name.starts_with('<') {
                continue;
            }
            match vmap.get(&m.selector) {
                Some(&index) if overrides(package, &vtable[index as usize]) => {
                    vtable[index as usize] = Arc::clone(m);
                }
                _ => {
                    vmap.insert(m.selector, vtable.len() as u32);
                    vtable.push(Arc::clone(m));
                }
            }
        }
        // Interface methods the class neither declares nor inherits: a
        // default method supplies the body, otherwise the entry stays
        // abstract and a call throws AbstractMethodError.
        for &iface in all_interfaces {
            for m in &self.get(iface).methods {
                if m.is_static() || m.is_private() || m.name.starts_with('<') {
                    continue;
                }
                match vmap.get(&m.selector) {
                    Some(&index) => {
                        let existing = &vtable[index as usize];
                        if existing.is_abstract()
                            && !m.is_abstract()
                            && self.get(existing.class).is_interface()
                        {
                            vtable[index as usize] = Arc::clone(m);
                        }
                    }
                    None => {
                        vmap.insert(m.selector, vtable.len() as u32);
                        vtable.push(Arc::clone(m));
                    }
                }
            }
        }
        (vtable, vmap)
    }
}
