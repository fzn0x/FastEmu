//! The class memory budget: how much class data one machine may hold.
//!
//! A class stays in the table for as long as its machine runs, and a game
//! chooses which classes load and how large they are: a small JAR can hold
//! classes that decompress to tens of megabytes of tables, and one class
//! can be extended by thousands of others that each copy its method table.
//! None of it is on the Java heap, so [`VmLimits::max_class_bytes`] bounds
//! it all together. Loading a class past the limit throws
//! `OutOfMemoryError`.
//!
//! The size of a class is an estimate, worked out from its definition
//! before anything of it is built, so a class that does not fit costs
//! almost nothing to refuse. Only classes a game can make count: those
//! read from class files, and the array classes of any name a game asks
//! for. The classes of the library are fixed and not counted.
//!
//! [`VmLimits::max_class_bytes`]: crate::VmLimits::max_class_bytes

use std::collections::HashSet;
use std::mem::size_of;
use std::sync::{Arc, OnceLock};

use super::link::{ClassDef, MethodSrc};
use super::table::{ClassTable, LoadError};
use super::{Bytecode, Class, ClassId, ConstValue, Field, Method};
use crate::classfile::{Constant, access};
use crate::interp::Resolved;
use crate::value::Value;

/// A class: its `Class`, the `Arc` around it, its name, and its slots in
/// the table's lists and maps.
const CLASS: usize = 512;
/// The counts of an `Arc`.
const ARC_HEADER: usize = 2 * size_of::<usize>();
/// A constant pool entry and the slot that caches what it resolves to.
const POOL_ENTRY: usize = size_of::<Constant>() + size_of::<OnceLock<Resolved>>();
/// A declared field.
const FIELD: usize = size_of::<Field>();
/// A static field's value, or an instance field's slot in a template.
const SLOT: usize = size_of::<Value>();
/// A declared method: its `Method` in an `Arc`, and its entry in the
/// table of selectors, which keeps its own copy of the name and the
/// descriptor (counted by length).
const METHOD: usize = size_of::<Method>() + ARC_HEADER + 96;
/// What a method with code adds: its `Bytecode`, with the room for the
/// decoded code.
const BYTECODE: usize = size_of::<Bytecode>();
/// A byte of code: the byte itself, and the 16 bytes of the instruction
/// and the 4 of its offset that decoding makes of it. Real code takes two
/// or three bytes to an instruction, so this counts about twice what it
/// takes. Methods decode on their first call; they are all counted at link
/// time, so that no hook is needed in the interpreter.
const CODE_BYTE: usize = 1 + 16 + 4;
/// An exception handler, as the class file has it and as decoded.
const HANDLER: usize = 8 + 16;
/// A line number entry.
const LINE: usize = 4;
/// A class id in a list of superclasses or interfaces.
const CLASS_ID: usize = size_of::<ClassId>();
/// An entry of a method table: the pointer in the vtable, and the entry of
/// the map from selectors to it.
const VTABLE_ENTRY: usize = size_of::<usize>() + 24;

/// The error of a class that does not fit.
fn limit_reached() -> LoadError {
    LoadError::new("java/lang/OutOfMemoryError", "class memory limit reached")
}

/// What the classes of one machine have taken of its limit.
pub(super) struct ClassBudget {
    /// The estimated bytes of class data held.
    pub(super) used: usize,
    max: usize,
}

impl ClassBudget {
    pub(super) fn new(max: usize) -> Self {
        Self { used: 0, max }
    }

    /// Takes `bytes` of the limit, or throws `OutOfMemoryError` when they
    /// do not fit. Nothing is taken then, so a smaller class can still
    /// load.
    pub(super) fn charge(&mut self, bytes: usize) -> Result<(), LoadError> {
        match self.used.checked_add(bytes) {
            Some(total) if total <= self.max => {
                self.used = total;
                Ok(())
            }
            _ => Err(limit_reached()),
        }
    }
}

/// Adds up sizes without overflowing.
#[derive(Default)]
struct Sum(usize);

impl Sum {
    /// Adds `count` things of `each` bytes.
    fn add(&mut self, count: usize, each: usize) {
        self.0 = self.0.saturating_add(count.saturating_mul(each));
    }
}

/// Whether a method with these flags and this name gets an entry in the
/// method table of its class (the test `build_vtable` makes).
fn is_virtual(flags: u16, name: &str) -> bool {
    flags & (access::STATIC | access::PRIVATE) == 0 && !name.starts_with('<')
}

impl ClassTable {
    /// Bytes charged for the array class `name`, which copies the method
    /// table of its superclass `object`.
    pub(super) fn array_class_bytes(name: &str, object: &Class) -> usize {
        let mut sum = Sum::default();
        sum.add(1, CLASS + name.len());
        sum.add(2, CLASS_ID);
        sum.add(object.vtable.len(), VTABLE_ENTRY);
        sum.0
    }

    /// The most bytes the class `def` holds once linked, given the
    /// classes it extends and implements (`all_interfaces`, sorted) and
    /// the number of its superclasses with itself (`supers`).
    ///
    /// The method table is counted at its longest: the superclass's, a
    /// slot for every method of the class that a subclass could override,
    /// and every method of the interfaces the superclass does not
    /// implement already. A method that overrides one shares its slot, so
    /// the number is a little over.
    pub(super) fn class_file_bytes(
        &self,
        def: &ClassDef,
        superclass: Option<&Class>,
        all_interfaces: &[ClassId],
        supers: usize,
    ) -> usize {
        let mut sum = Sum::default();
        sum.add(1, CLASS + def.name.len());
        if let Some((pool, _)) = &def.pool {
            sum.add(pool.len(), POOL_ENTRY);
            sum.add(1, pool.utf8_bytes());
        }

        let (mut statics, mut instance) = (0, 0);
        // A `ConstantValue` string is a copy of its text in UTF-16, and
        // fields of one constant share it.
        let mut strings = HashSet::new();
        for f in &def.fields {
            if f.access & access::STATIC != 0 {
                statics += 1;
            } else {
                instance += 1;
            }
            if let Some(ConstValue::String(text)) = &f.constant
                && strings.insert(Arc::as_ptr(text).cast::<u16>())
            {
                sum.add(1, ARC_HEADER);
                sum.add(text.len(), size_of::<u16>());
            }
        }
        sum.add(def.fields.len(), FIELD);
        sum.add(statics, SLOT);
        // A subclass copies the instance fields of its superclass.
        sum.add(superclass.map_or(0, |s| s.template.len()) + instance, SLOT);

        let mut overridable = 0;
        for m in &def.methods {
            sum.add(1, METHOD + m.name.len() + m.desc.len());
            if let MethodSrc::Code(code) = &m.src {
                sum.add(1, BYTECODE);
                sum.add(code.code.len(), CODE_BYTE);
                sum.add(code.handlers.len(), HANDLER);
                sum.add(code.lines.len(), LINE);
            }
            if is_virtual(m.access, &m.name) {
                overridable += 1;
            }
        }

        sum.add(
            supers + all_interfaces.len() + def.interfaces.len(),
            CLASS_ID,
        );
        if def.access & access::INTERFACE == 0 {
            let inherited = |iface: &ClassId| {
                superclass.is_some_and(|s| s.all_interfaces.binary_search(iface).is_ok())
            };
            let from_interfaces: usize = all_interfaces
                .iter()
                .filter(|iface| !inherited(iface))
                .map(|&iface| self.get(iface).methods.len())
                .fold(0, usize::saturating_add);
            sum.add(superclass.map_or(0, |s| s.vtable.len()), VTABLE_ENTRY);
            sum.add(overridable, VTABLE_ENTRY);
            sum.add(from_interfaces, VTABLE_ENTRY);
        }
        sum.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classfile::{CodeAttr, ConstantPool, ExceptionHandler, Utf8};
    use crate::native::NativeClass;
    use crate::source::MemorySource;

    use super::super::link::{FieldDef, MethodDef};

    #[test]
    fn budget_takes_what_fits_and_refuses_the_rest() {
        let mut budget = ClassBudget::new(1000);
        budget.charge(600).unwrap();
        let err = budget.charge(401).unwrap_err();
        assert_eq!(err.exception, "java/lang/OutOfMemoryError");
        assert_eq!(err.message, "class memory limit reached");
        assert_eq!(err.missing, None);
        // A refusal takes nothing, so what still fits loads.
        assert_eq!(budget.used, 600);
        budget.charge(400).unwrap();
        assert_eq!(budget.used, 1000);
        assert!(budget.charge(1).is_err());
        assert!(ClassBudget::new(10).charge(usize::MAX).is_err());
    }

    #[test]
    fn the_model_follows_the_size_of_what_it_counts() {
        // A pool entry with its cache slot is about 100 bytes, as measured
        // on a pool of 65535 entries (6.3 MiB).
        assert!((96..=128).contains(&POOL_ENTRY), "{POOL_ENTRY}");
        assert_eq!(size_of::<Field>(), FIELD);
        assert_eq!(size_of::<Value>(), SLOT);
        assert!(size_of::<Class>() + ARC_HEADER + 64 <= CLASS);
    }

    fn table() -> ClassTable {
        let natives = vec![
            NativeClass::new("java/lang/Object"),
            NativeClass::interface("a/I")
                .abstract_method("one", "()V")
                .abstract_method("two", "()V")
                .abstract_method("three", "()V"),
            NativeClass::new("a/Impl").implements("a/I"),
        ];
        ClassTable::new(Arc::new(MemorySource::new()), natives, usize::MAX)
    }

    /// A class `a/A` of a class file, with nothing in it.
    fn def() -> ClassDef {
        ClassDef {
            name: "a/A".into(),
            access: access::PUBLIC | access::SUPER,
            super_name: Some("java/lang/Object".into()),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            source_file: None,
            payload: None,
            pool: Some((ConstantPool::default(), true)),
        }
    }

    fn method(name: &str, flags: u16, src: MethodSrc) -> MethodDef {
        MethodDef {
            name: name.into(),
            desc: "()V".into(),
            access: flags,
            src,
        }
    }

    fn code(len: usize, handlers: usize, lines: usize) -> MethodSrc {
        let handler = ExceptionHandler {
            start_pc: 0,
            end_pc: 0,
            handler_pc: 0,
            catch_type: 0,
        };
        MethodSrc::Code(CodeAttr {
            max_stack: 0,
            max_locals: 1,
            code: vec![0; len],
            handlers: vec![handler; handlers],
            lines: vec![(0, 0); lines],
        })
    }

    fn pool(entries: Vec<Constant>) -> (ConstantPool, bool) {
        (ConstantPool::from_entries(entries), true)
    }

    /// What `def` takes with `Object` above it.
    fn cost(t: &mut ClassTable, def: &ClassDef) -> usize {
        let object = t.load("java/lang/Object").unwrap();
        let object = Arc::clone(t.get(object));
        t.class_file_bytes(def, Some(&object), &[], 2)
    }

    #[test]
    fn a_class_without_members_costs_its_base_and_its_name() {
        let mut t = table();
        assert_eq!(cost(&mut t, &def()), CLASS + 3 + 2 * CLASS_ID);
        let mut long = def();
        long.name = "a".repeat(200).into();
        assert_eq!(cost(&mut t, &long), CLASS + 200 + 2 * CLASS_ID);
    }

    #[test]
    fn the_pool_methods_and_fields_each_add_their_part() {
        let mut t = table();
        let base = cost(&mut t, &def());

        let mut with_pool = def();
        with_pool.pool = Some(pool(vec![Constant::Integer(1); 1000]));
        assert_eq!(cost(&mut t, &with_pool) - base, 1000 * POOL_ENTRY);

        // The text of a name counts, with the counts of its `Arc`.
        let text = Constant::Utf8(Utf8 {
            text: "x".repeat(5000).into(),
            exact: None,
        });
        let mut with_text = def();
        with_text.pool = Some(pool(vec![Constant::Unused, text]));
        assert_eq!(
            cost(&mut t, &with_text) - base,
            2 * POOL_ENTRY + ARC_HEADER + 5000
        );

        // An abstract method, then a private one with 65535 bytes of code,
        // 3 handlers and 5 line numbers. Only the first has a slot in the
        // method table.
        let mut with_methods = def();
        let public = access::PUBLIC | access::ABSTRACT;
        with_methods
            .methods
            .push(method("m", public, MethodSrc::Abstract));
        let abstract_only = cost(&mut t, &with_methods) - base;
        assert_eq!(abstract_only, METHOD + 1 + 3 + VTABLE_ENTRY);
        with_methods
            .methods
            .push(method("n", access::PRIVATE, code(65535, 3, 5)));
        let coded = cost(&mut t, &with_methods) - base - abstract_only;
        assert_eq!(
            coded,
            METHOD + 1 + 3 + BYTECODE + 65535 * CODE_BYTE + 3 * HANDLER + 5 * LINE
        );
        // A method of 64 KiB of code is over a megabyte: the decoded
        // code is the larger part.
        assert!(coded > 1 << 20, "{coded}");

        let mut with_fields = def();
        for (i, flags) in [access::STATIC, 0, 0].into_iter().enumerate() {
            with_fields.fields.push(FieldDef {
                name: format!("f{i}").into(),
                desc: "I".into(),
                access: flags,
                constant: None,
            });
        }
        // The static value of one, and the slots of two in the template.
        assert_eq!(cost(&mut t, &with_fields) - base, 3 * FIELD + 3 * SLOT);
    }

    #[test]
    fn a_string_constant_counts_once_however_many_fields_name_it() {
        let mut t = table();
        let base = cost(&mut t, &def());
        let text: Arc<[u16]> = vec![0; 30_000].into();
        let mut shared = def();
        for i in 0..50 {
            shared.fields.push(FieldDef {
                name: format!("f{i}").into(),
                desc: "Ljava/lang/String;".into(),
                access: access::STATIC | access::FINAL,
                constant: Some(ConstValue::String(Arc::clone(&text))),
            });
        }
        let fields = 50 * (FIELD + SLOT);
        assert_eq!(
            cost(&mut t, &shared) - base,
            fields + ARC_HEADER + 2 * 30_000
        );
    }

    #[test]
    fn the_method_table_counts_the_interface_methods_the_superclass_lacks() {
        let mut t = table();
        let sup = t.load("a/Impl").unwrap();
        let sup = Arc::clone(t.get(sup));
        let object = t.load("java/lang/Object").unwrap();
        let object = Arc::clone(t.get(object));
        // `a/Impl` has the three methods of `a/I` in its method table: a
        // class that extends it copies them, and its list of interfaces.
        let sub = t.class_file_bytes(&def(), Some(&sup), &sup.all_interfaces, 3);
        assert_eq!(sub, CLASS + 3 + 4 * CLASS_ID + 3 * VTABLE_ENTRY);
        // A class that has the interface but not the superclass's takes the
        // methods as new ones: the same cost, from a different term.
        let direct = t.class_file_bytes(&def(), Some(&object), &sup.all_interfaces, 2);
        assert_eq!(direct, CLASS + 3 + 3 * CLASS_ID + 3 * VTABLE_ENTRY);
    }

    #[test]
    fn a_longer_list_of_interfaces_costs_more_by_its_length() {
        let mut t = table();
        let iface = t.load("a/I").unwrap();
        let object = t.load("java/lang/Object").unwrap();
        let object = Arc::clone(t.get(object));
        // An interface has no method table to grow.
        let mut def = def();
        def.access = access::PUBLIC | access::INTERFACE | access::ABSTRACT;
        let none = t.class_file_bytes(&def, Some(&object), &[], 2);
        let few = t.class_file_bytes(&def, Some(&object), &[iface], 2);
        let many = t.class_file_bytes(&def, Some(&object), &[iface; 11], 2);
        assert_eq!(few, none + CLASS_ID);
        assert_eq!(many - few, 10 * CLASS_ID);
    }

    #[test]
    fn a_class_of_the_library_is_not_charged() {
        // Only a class read from a class file has a pool, and only those
        // are charged: a native class loads with no room at all.
        let natives = vec![NativeClass::new("java/lang/Object")];
        let mut t = ClassTable::new(Arc::new(MemorySource::new()), natives, 0);
        t.load("java/lang/Object").unwrap();
        assert_eq!(t.budget.used, 0);
    }
}
