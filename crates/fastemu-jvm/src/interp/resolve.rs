//! Lazy constant pool resolution with per-class caches.
//!
//! The first execution of an instruction that names a class, field, method
//! or string resolves the constant pool entry and caches the result in the
//! class's [`RtPool`]; later executions read the cache. A failed
//! resolution is not cached, so it throws again each time, as the JVM
//! specification requires.

use std::sync::atomic::AtomicU64;
use std::sync::{Arc, OnceLock, Weak};

use crate::class::{ClassId, Method, RtPool, Selector};
use crate::classfile::Constant;
use crate::ctx::{Ctx, JResult};
use crate::descriptor::Kind;
use crate::lang;
use crate::value::ObjRef;

/// What a constant pool entry resolved to.
pub(crate) enum Resolved {
    Class(ClassId),
    String(ObjRef),
    Field(FieldRes),
    Method(MethodRes),
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum FieldRes {
    Static {
        class: ClassId,
        index: u32,
        wide: bool,
    },
    Instance {
        slot: u32,
        wide: bool,
    },
}

/// A resolved method reference and what dispatch needs from it.
pub(crate) struct MethodRes {
    /// The resolved method; weak so constant pools do not keep methods,
    /// and through them other pools, alive in a cycle.
    pub(crate) method: Weak<Method>,
    pub(crate) class: ClassId,
    pub(crate) selector: Selector,
    pub(crate) arg_slots: u16,
    pub(crate) is_static: bool,
    /// The vtable index in the referenced class, for `invokevirtual`.
    pub(crate) vindex: Option<u32>,
    /// Private and final methods need no dispatch.
    pub(crate) direct: bool,
    /// The method `invokespecial` selects, once computed.
    pub(crate) special: OnceLock<Weak<Method>>,
    /// The last receiver class of `invokeinterface` (plus one, high half)
    /// and the vtable index it dispatched to (low half).
    pub(crate) icache: AtomicU64,
}

impl MethodRes {
    #[inline]
    pub(crate) fn target(&self) -> Arc<Method> {
        self.method
            .upgrade()
            .expect("methods live as long as their class table")
    }
}

impl Ctx<'_> {
    /// Resolves class entry `index` of `pool`.
    pub(crate) fn resolve_class(&mut self, pool: &RtPool, index: u16) -> JResult<ClassId> {
        if let Some(Resolved::Class(id)) = pool.cache[index as usize].get() {
            return Ok(*id);
        }
        let name = match pool.cp.class_name(index) {
            Ok(n) => n,
            Err(e) => return Err(self.new_exception("java/lang/VerifyError", Some(&e.0))),
        };
        let id = self.find_class(name)?;
        let _ = pool.cache[index as usize].set(Resolved::Class(id));
        Ok(id)
    }

    /// Resolves string entry `index` to its interned string.
    pub(crate) fn resolve_string(&mut self, pool: &RtPool, index: u16) -> JResult<ObjRef> {
        if let Some(Resolved::String(s)) = pool.cache[index as usize].get() {
            return Ok(*s);
        }
        let chars = match pool.cp.string(index) {
            Ok(c) => c,
            Err(e) => return Err(self.new_exception("java/lang/VerifyError", Some(&e.0))),
        };
        let s = lang::string::intern(&mut self.state, &chars);
        let _ = pool.cache[index as usize].set(Resolved::String(s));
        Ok(s)
    }

    /// Resolves field entry `index`.
    pub(crate) fn resolve_field_ref(&mut self, pool: &RtPool, index: u16) -> JResult<FieldRes> {
        if let Some(Resolved::Field(f)) = pool.cache[index as usize].get() {
            return Ok(*f);
        }
        let (class_index, name, desc) = match pool.cp.get(index) {
            Some(Constant::FieldRef { class, .. }) => match pool.cp.member_ref(index) {
                Ok(m) => (*class, m.name.to_owned(), m.descriptor.to_owned()),
                Err(e) => return Err(self.new_exception("java/lang/VerifyError", Some(&e.0))),
            },
            _ => {
                let message = format!("constant pool entry {index} is not a field reference");
                return Err(self.new_exception("java/lang/VerifyError", Some(&message)));
            }
        };
        let class = self.resolve_class(pool, class_index)?;
        let found = self
            .state
            .classes
            .resolve_field(class, &name, &desc)
            .map(|(owner, f)| {
                let wide = matches!(f.kind, Kind::Long | Kind::Double);
                if f.is_static() {
                    FieldRes::Static {
                        class: owner,
                        index: f.slot,
                        wide,
                    }
                } else {
                    FieldRes::Instance { slot: f.slot, wide }
                }
            });
        let Some(res) = found else {
            let class_name = self.class_name(class).to_owned();
            return Err(self.missing_field(&class_name, &name, &desc));
        };
        let _ = pool.cache[index as usize].set(Resolved::Field(res));
        Ok(res)
    }

    /// Resolves method entry `index` (a `Methodref` or an
    /// `InterfaceMethodref`).
    pub(crate) fn resolve_method_ref<'p>(
        &mut self,
        pool: &'p RtPool,
        index: u16,
    ) -> JResult<&'p MethodRes> {
        if let Some(Resolved::Method(m)) = pool.cache[index as usize].get() {
            return Ok(m);
        }
        let (class_index, interface) = match pool.cp.get(index) {
            Some(Constant::MethodRef { class, .. }) => (*class, false),
            Some(Constant::InterfaceMethodRef { class, .. }) => (*class, true),
            _ => {
                let message = format!("constant pool entry {index} is not a method reference");
                return Err(self.new_exception("java/lang/VerifyError", Some(&message)));
            }
        };
        let (name, desc) = match pool.cp.member_ref(index) {
            Ok(m) => (m.name.to_owned(), m.descriptor.to_owned()),
            Err(e) => return Err(self.new_exception("java/lang/VerifyError", Some(&e.0))),
        };
        let class = self.resolve_class(pool, class_index)?;
        let is_interface = self.state.classes.get(class).is_interface();
        if is_interface != interface {
            let message = format!(
                "{} {}",
                if interface {
                    "Found class"
                } else {
                    "Found interface"
                },
                self.class_name(class).replace('/', ".")
            );
            return Err(
                self.new_exception("java/lang/IncompatibleClassChangeError", Some(&message))
            );
        }
        let method = if interface {
            self.state
                .classes
                .resolve_interface_method(class, &name, &desc)
        } else {
            self.state.classes.resolve_method(class, &name, &desc)
        };
        let Some(method) = method else {
            let class_name = self.class_name(class).to_owned();
            return Err(self.missing_method(&class_name, &name, &desc));
        };
        let referenced = self.state.classes.get(class);
        let vindex = referenced.vmap.get(&method.selector).copied();
        let direct = method.is_private()
            || method.access & crate::classfile::access::FINAL != 0
            || &*method.name == "<init>";
        let res = MethodRes {
            method: Arc::downgrade(&method),
            class: method.class,
            selector: method.selector,
            arg_slots: method.arg_slots,
            is_static: method.is_static(),
            vindex,
            direct,
            special: OnceLock::new(),
            icache: AtomicU64::new(0),
        };
        let _ = pool.cache[index as usize].set(Resolved::Method(res));
        match pool.cache[index as usize].get() {
            Some(Resolved::Method(m)) => Ok(m),
            _ => unreachable!("set above or by another thread with the same entry"),
        }
    }
}
