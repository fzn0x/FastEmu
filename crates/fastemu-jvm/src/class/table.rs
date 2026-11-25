//! The class table: loading by name, array classes, member resolution and
//! subtype checks.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::budget::ClassBudget;
use super::{ArrayType, Class, ClassId, ClassKind, Field, Method, Selector};
use crate::classfile::{ClassFile, access};
use crate::native::NativeClass;
use crate::source::ClassSource;
use crate::thread::ThreadId;
use crate::value::{ObjRef, Value};

/// Where a class is in its initialization (JVMS 5.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InitState {
    /// Loaded and linked, not yet initialized.
    Linked,
    /// A thread is running the static initializer.
    Running(ThreadId),
    Done,
    /// The static initializer threw; using the class throws
    /// `NoClassDefFoundError`.
    Failed,
}

/// The longest class name `load` takes, in bytes. javac and ProGuard
/// output stays far below it (the longest name of the test games is 57
/// bytes).
pub(crate) const MAX_CLASS_NAME: usize = 255;

/// The most dimensions of an array class (the limit of JVMS 4.3.2).
pub(crate) const MAX_ARRAY_DIMS: usize = 255;

/// The most classes on one path up a hierarchy, through superclasses and
/// interfaces, `java/lang/Object` and the class itself included. Loading
/// and initializing recurse once per class of the path, so this bounds the
/// host stack they use, and the list of interfaces of a class does not
/// grow past it for a chain. The chains of game classes of the test games
/// have 5 classes at most.
pub(crate) const MAX_HIERARCHY_DEPTH: usize = 256;

/// The longest name `load` can accept at all: an array of
/// [`MAX_ARRAY_DIMS`] dimensions of a class with a [`MAX_CLASS_NAME`] name
/// (`[[[Lname;`).
pub(crate) const MAX_LOADABLE_NAME: usize = MAX_ARRAY_DIMS + MAX_CLASS_NAME + 2;

/// The longest piece of a game chosen name that goes into an error
/// message, in characters.
const MAX_SHOWN: usize = 100;

/// `name` cut to [`MAX_SHOWN`] characters, for an error message that
/// carries a name a game chose.
pub(crate) fn short(name: &str) -> String {
    match name.char_indices().nth(MAX_SHOWN) {
        Some((end, _)) => format!("{}...", &name[..end]),
        None => name.to_owned(),
    }
}

/// The error of a class whose hierarchy has too many classes.
pub(super) fn too_deep(name: &str) -> LoadError {
    LoadError::new(
        "java/lang/ClassFormatError",
        format!(
            "{}: the class hierarchy is over {MAX_HIERARCHY_DEPTH} classes deep",
            short(name)
        ),
    )
}

/// Why a class could not be loaded, as the Java exception to throw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoadError {
    /// The internal name of the exception class, such as
    /// `java/lang/NoClassDefFoundError`.
    pub(crate) exception: &'static str,
    pub(crate) message: String,
    /// The class FastEmu could not find, for the diagnostics hook.
    pub(crate) missing: Option<String>,
}

impl LoadError {
    pub(crate) fn no_class(name: &str) -> Self {
        Self {
            exception: "java/lang/NoClassDefFoundError",
            message: short(name),
            missing: Some(name.to_owned()),
        }
    }

    pub(crate) fn new(exception: &'static str, message: impl Into<String>) -> Self {
        Self {
            exception,
            message: message.into(),
            missing: None,
        }
    }
}

pub(crate) struct ClassTable {
    classes: Vec<Arc<Class>>,
    by_name: HashMap<Arc<str>, ClassId>,
    natives: HashMap<String, NativeClass>,
    source: Arc<dyn ClassSource>,
    /// Classes being loaded on this path, to catch circular superclasses.
    loading: Vec<Arc<str>>,
    /// Selectors by method name, then descriptor.
    selectors: HashMap<Arc<str>, HashMap<Arc<str>, Selector>>,
    selector_count: u32,
    /// Static field storage of every class.
    pub(crate) statics: Vec<Value>,
    /// Initialization state, by class.
    pub(crate) init: Vec<InitState>,
    /// The `java.lang.Class` object of each class, made on first use.
    pub(crate) mirrors: Vec<Option<ObjRef>>,
    /// What the classes a game can make take of the machine's limit on
    /// class data. Classes are never unloaded, so nothing is given back.
    pub(super) budget: ClassBudget,
}

impl ClassTable {
    /// A table that loads classes from `source` and `natives`, and lets
    /// the classes of a game take `max_class_bytes` of class data (see
    /// [`VmLimits::max_class_bytes`](crate::VmLimits::max_class_bytes)).
    pub(crate) fn new(
        source: Arc<dyn ClassSource>,
        natives: Vec<NativeClass>,
        max_class_bytes: usize,
    ) -> Self {
        Self {
            classes: Vec::new(),
            by_name: HashMap::new(),
            natives: natives.into_iter().map(|n| (n.name.clone(), n)).collect(),
            source,
            loading: Vec::new(),
            selectors: HashMap::new(),
            selector_count: 0,
            statics: Vec::new(),
            init: Vec::new(),
            mirrors: Vec::new(),
            budget: ClassBudget::new(max_class_bytes),
        }
    }

    #[inline]
    pub(crate) fn get(&self, id: ClassId) -> &Arc<Class> {
        &self.classes[id.index()]
    }

    pub(crate) fn source(&self) -> &Arc<dyn ClassSource> {
        &self.source
    }

    /// The class named `name` if it is already loaded.
    pub(crate) fn lookup(&self, name: &str) -> Option<ClassId> {
        self.by_name.get(name).copied()
    }

    /// The interned selector of `name` and `desc`.
    pub(crate) fn selector(&mut self, name: &str, desc: &str) -> Selector {
        if let Some(sel) = self.find_selector(name, desc) {
            return sel;
        }
        let sel = Selector(self.selector_count);
        self.selector_count += 1;
        self.selectors
            .entry(name.into())
            .or_default()
            .insert(desc.into(), sel);
        sel
    }

    /// The selector of `name` and `desc` if any method has it.
    pub(crate) fn find_selector(&self, name: &str, desc: &str) -> Option<Selector> {
        self.selectors.get(name)?.get(desc).copied()
    }

    /// Loads, links and returns the class named `name` (an internal name
    /// such as `java/lang/String`, or an array descriptor such as `[I`).
    ///
    /// A name over [`MAX_CLASS_NAME`] bytes, an array of over
    /// [`MAX_ARRAY_DIMS`] dimensions and a hierarchy of over
    /// [`MAX_HIERARCHY_DEPTH`] classes do not load: names come from games
    /// and from files, and a load recurses once per class of a hierarchy.
    pub(crate) fn load(&mut self, name: &str) -> Result<ClassId, LoadError> {
        if let Some(id) = self.lookup(name) {
            return Ok(id);
        }
        if name.starts_with('[') {
            return self.load_array(name);
        }
        if name.len() > MAX_CLASS_NAME {
            return Err(LoadError::new(
                "java/lang/NoClassDefFoundError",
                short(name),
            ));
        }
        if self.loading.iter().any(|n| &**n == name) {
            return Err(LoadError::new("java/lang/ClassCircularityError", name));
        }
        // Each class being loaded here waits for its superclass or one of
        // its interfaces, so this is the depth of the recursion.
        if self.loading.len() >= MAX_HIERARCHY_DEPTH {
            return Err(too_deep(name));
        }
        let def = if let Some(native) = self.natives.remove(name) {
            super::link::from_native(native)?
        } else {
            let Some(bytes) = self.source.class_bytes(name) else {
                return Err(LoadError::no_class(name));
            };
            let file = ClassFile::parse(&bytes).map_err(|e| {
                LoadError::new("java/lang/ClassFormatError", format!("{name}: {e}"))
            })?;
            if &*file.name != name {
                return Err(LoadError::new(
                    "java/lang/NoClassDefFoundError",
                    format!("{name} (wrong name: {})", short(&file.name)),
                ));
            }
            super::link::from_class_file(file)?
        };
        self.loading.push(Arc::clone(&def.name));
        let result = self.define(def);
        self.loading.pop();
        result
    }

    /// Adds a finished class to the table.
    pub(super) fn insert(&mut self, class: Class, initialized: bool) -> ClassId {
        let id = class.id;
        debug_assert_eq!(id.index(), self.classes.len());
        self.by_name.insert(Arc::clone(&class.name), id);
        self.classes.push(Arc::new(class));
        self.init.push(if initialized {
            InitState::Done
        } else {
            InitState::Linked
        });
        self.mirrors.push(None);
        id
    }

    pub(super) fn next_id(&self) -> ClassId {
        ClassId(self.classes.len() as u32)
    }

    /// Loads the array class `name` and the array classes inside it. The
    /// classes are made from the innermost out in a loop, so the stack a
    /// name needs does not grow with its dimensions.
    fn load_array(&mut self, name: &str) -> Result<ClassId, LoadError> {
        let dims = name.bytes().take_while(|&b| b == b'[').count();
        if dims > MAX_ARRAY_DIMS {
            return Err(LoadError::new(
                "java/lang/ClassFormatError",
                "array type has too many dimensions",
            ));
        }
        if name.len() > MAX_LOADABLE_NAME {
            return Err(LoadError::new(
                "java/lang/NoClassDefFoundError",
                short(name),
            ));
        }
        let kind = match &name.as_bytes()[dims..] {
            [c] => ClassKind::PrimArray(
                ArrayType::from_descriptor(*c).ok_or_else(|| LoadError::no_class(name))?,
            ),
            [b'L', inner @ .., b';'] if !inner.is_empty() => {
                let inner = std::str::from_utf8(inner).expect("a slice of a str at ASCII bounds");
                ClassKind::RefArray(self.load(inner)?)
            }
            _ => return Err(LoadError::no_class(name)),
        };
        let object = self.load("java/lang/Object")?;
        let object = Arc::clone(self.get(object));
        // The array of the innermost dimension is `name[dims - 1..]`, and
        // each dimension out from it adds one `[` in front.
        let mut class = self.make_array(&name[dims - 1..], kind, &object);
        for start in (0..dims - 1).rev() {
            class = self.make_array(&name[start..], ClassKind::RefArray(class), &object);
        }
        Ok(class)
    }

    /// The array class `name` of `kind`, made if it is not there yet.
    fn make_array(&mut self, name: &str, kind: ClassKind, object: &Arc<Class>) -> ClassId {
        if let Some(id) = self.lookup(name) {
            return id;
        }
        let id = self.next_id();
        let class = Class {
            id,
            name: name.into(),
            access: access::PUBLIC | access::FINAL | access::ABSTRACT,
            kind,
            superclass: Some(object.id),
            interfaces: Box::new([]),
            all_interfaces: Box::new([]),
            supers: Box::new([object.id, id]),
            height: 2,
            fields: Box::new([]),
            template: Box::new([]),
            methods: Box::new([]),
            vtable: object.vtable.clone(),
            vmap: object.vmap.clone(),
            payload: None,
            array_type: std::sync::OnceLock::new(),
        };
        self.insert(class, true)
    }

    /// The class of arrays whose elements are `component`.
    pub(crate) fn array_of(&mut self, component: ClassId) -> Result<ClassId, LoadError> {
        let c = self.get(component);
        let name = if c.is_array() {
            format!("[{}", c.name)
        } else {
            format!("[L{};", c.name)
        };
        self.load(&name)
    }

    pub(crate) fn prim_array(&mut self, t: ArrayType) -> ClassId {
        let name = format!("[{}", t.descriptor());
        self.load(&name)
            .expect("primitive array classes always load once Object has")
    }

    /// The method a reference to `name` and `desc` in class `class`
    /// resolves to (JVMS 5.4.3.3): the class and its superclasses first,
    /// then its superinterfaces.
    pub(crate) fn resolve_method(
        &self,
        class: ClassId,
        name: &str,
        desc: &str,
    ) -> Option<Arc<Method>> {
        let mut cur = Some(class);
        while let Some(id) = cur {
            let c = self.get(id);
            if let Some(m) = c.declared_method(name, desc) {
                return Some(Arc::clone(m));
            }
            cur = c.superclass;
        }
        self.superinterface_method(class, name, desc)
    }

    /// Interface method resolution (JVMS 5.4.3.4): the interface, then
    /// `java/lang/Object`, then the superinterfaces.
    pub(crate) fn resolve_interface_method(
        &self,
        iface: ClassId,
        name: &str,
        desc: &str,
    ) -> Option<Arc<Method>> {
        let c = self.get(iface);
        if let Some(m) = c.declared_method(name, desc) {
            return Some(Arc::clone(m));
        }
        if let Some(object) = self.lookup("java/lang/Object")
            && let Some(m) = self.get(object).declared_method(name, desc)
            && !m.is_static()
            && m.access & access::PUBLIC != 0
        {
            return Some(Arc::clone(m));
        }
        self.superinterface_method(iface, name, desc)
    }

    /// A method of `class`'s superinterfaces, preferring one with a body.
    fn superinterface_method(&self, class: ClassId, name: &str, desc: &str) -> Option<Arc<Method>> {
        let mut found: Option<&Arc<Method>> = None;
        for &iface in &self.get(class).all_interfaces {
            if let Some(m) = self.get(iface).declared_method(name, desc)
                && !m.is_static()
                && !m.is_private()
            {
                if !m.is_abstract() {
                    return Some(Arc::clone(m));
                }
                found.get_or_insert(m);
            }
        }
        found.cloned()
    }

    /// Field resolution (JVMS 5.4.3.2): the class, its superinterfaces,
    /// then its superclass, each of them searched the same way.
    ///
    /// A class is searched once however many paths lead to it, so the
    /// search takes time linear in the size of the interface graph. A
    /// class reached a second time was searched already and without a
    /// hit, or the search would have ended there.
    pub(crate) fn resolve_field(
        &self,
        class: ClassId,
        name: &str,
        desc: &str,
    ) -> Option<(ClassId, &Field)> {
        if let Some(f) = self.get(class).declared_field(name, desc) {
            return Some((class, f));
        }
        let mut seen = HashSet::from([class]);
        let mut todo = Vec::new();
        self.push_supers(class, &mut todo);
        while let Some(id) = todo.pop() {
            if !seen.insert(id) {
                continue;
            }
            if let Some(f) = self.get(id).declared_field(name, desc) {
                return Some((id, f));
            }
            self.push_supers(id, &mut todo);
        }
        None
    }

    /// Puts the superclass of `class`, then its interfaces last to first,
    /// on `todo`, so that popping gives the interfaces in order and the
    /// superclass after all of them.
    fn push_supers(&self, class: ClassId, todo: &mut Vec<ClassId>) {
        let c = self.get(class);
        todo.extend(c.superclass);
        todo.extend(c.interfaces.iter().rev());
    }

    /// The method `invokespecial` runs (JVMS 6.5.invokespecial): a call
    /// of an inherited method through `super` selects from the current
    /// class's superclass.
    pub(crate) fn select_special(
        &self,
        current: ClassId,
        super_semantics: bool,
        resolved: &Arc<Method>,
    ) -> Arc<Method> {
        let cur = self.get(current);
        let target = self.get(resolved.class);
        if !super_semantics
            || &*resolved.name == "<init>"
            || target.is_interface()
            || resolved.class == current
            || !self.is_subclass(current, resolved.class)
        {
            return Arc::clone(resolved);
        }
        let mut next = cur.superclass;
        while let Some(id) = next {
            let c = self.get(id);
            if let Some(m) = c.declared_method(&resolved.name, &resolved.desc)
                && !m.is_static()
            {
                return Arc::clone(m);
            }
            next = c.superclass;
        }
        Arc::clone(resolved)
    }

    /// Whether `sub` is `sup` or a subclass of it (classes only).
    #[inline]
    pub(crate) fn is_subclass(&self, sub: ClassId, sup: ClassId) -> bool {
        let depth = self.get(sup).depth();
        self.get(sub).supers.get(depth) == Some(&sup)
    }

    /// Whether a value of runtime class `from` can be stored where class
    /// `to` is expected (`checkcast`, `instanceof`, `aastore`, catch
    /// clauses; JVMS 6.5.checkcast).
    pub(crate) fn is_assignable(&self, from: ClassId, to: ClassId) -> bool {
        if from == to {
            return true;
        }
        let f = self.get(from);
        let t = self.get(to);
        match t.kind {
            ClassKind::Interface => {
                if f.is_array() {
                    matches!(&*t.name, "java/lang/Cloneable" | "java/io/Serializable")
                } else {
                    f.all_interfaces.binary_search(&to).is_ok()
                }
            }
            ClassKind::Plain => {
                t.superclass.is_none() || (!f.is_array() && self.is_subclass(from, to))
            }
            ClassKind::RefArray(tc) => match f.kind {
                ClassKind::RefArray(fc) => self.is_assignable(fc, tc),
                _ => false,
            },
            ClassKind::PrimArray(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::NativeClass;
    use crate::source::MemorySource;

    fn table() -> ClassTable {
        let natives = vec![
            NativeClass::new("java/lang/Object"),
            NativeClass::interface("a/I"),
            NativeClass::interface("a/J").implements("a/I"),
            NativeClass::new("a/A")
                .implements("a/J")
                .field("x", "I")
                .static_field("s", "J"),
            NativeClass::new("a/B").extends("a/A").field("y", "D"),
            NativeClass::new("a/Loop").extends("a/Loop"),
        ];
        ClassTable::new(Arc::new(MemorySource::new()), natives, usize::MAX)
    }

    #[test]
    fn subclasses_inherit_fields_and_interfaces() {
        let mut t = table();
        let b = t.load("a/B").unwrap();
        let a = t.load("a/A").unwrap();
        let i = t.load("a/I").unwrap();
        let object = t.load("java/lang/Object").unwrap();
        assert_eq!(t.get(b).template.len(), 2);
        assert_eq!(t.get(b).template[1], Value::Double(0.0));
        assert!(t.is_assignable(b, a));
        assert!(t.is_assignable(b, i));
        assert!(t.is_assignable(b, object));
        assert!(!t.is_assignable(a, b));
        let (owner, field) = t.resolve_field(b, "x", "I").unwrap();
        assert_eq!((owner, field.slot), (a, 0));
        let (_, s) = t.resolve_field(b, "s", "J").unwrap();
        assert!(s.is_static());
        assert_eq!(t.statics[s.slot as usize], Value::Long(0));
    }

    #[test]
    fn array_classes_follow_their_components() {
        let mut t = table();
        let bs = t.load("[[La/B;").unwrap();
        let as_ = t.load("[[La/A;").unwrap();
        let ints = t.load("[I").unwrap();
        let object = t.load("java/lang/Object").unwrap();
        assert!(t.is_assignable(bs, as_));
        assert!(!t.is_assignable(as_, bs));
        assert!(t.is_assignable(ints, object));
        assert!(!t.is_assignable(ints, t.lookup("[[La/A;").unwrap()));
        let b = t.load("a/B").unwrap();
        let array = t.array_of(b).unwrap();
        assert_eq!(&*t.get(array).name, "[La/B;");
        assert_eq!(t.prim_array(ArrayType::Int), ints);
    }

    /// A table of `natives` and `java/lang/Object`.
    fn table_of(natives: Vec<NativeClass>) -> ClassTable {
        let mut all = vec![NativeClass::new("java/lang/Object")];
        all.extend(natives);
        ClassTable::new(Arc::new(MemorySource::new()), all, usize::MAX)
    }

    #[test]
    fn field_lookup_searches_interfaces_before_the_superclass() {
        let mut t = table_of(vec![
            NativeClass::interface("a/I").static_field("x", "I"),
            NativeClass::interface("a/J").implements("a/I"),
            NativeClass::interface("a/K")
                .static_field("x", "I")
                .static_field("k", "I"),
            NativeClass::new("a/S")
                .static_field("x", "I")
                .static_field("s", "I"),
            NativeClass::new("a/C")
                .extends("a/S")
                .implements("a/J")
                .implements("a/K"),
        ]);
        let c = t.load("a/C").unwrap();
        let owner = |t: &ClassTable, name: &str| {
            let (owner, _) = t.resolve_field(c, name, "I")?;
            Some(t.get(owner).name.to_string())
        };
        // `a/J` comes first and its superinterface `a/I` before `a/K`.
        assert_eq!(owner(&t, "x").as_deref(), Some("a/I"));
        assert_eq!(owner(&t, "k").as_deref(), Some("a/K"));
        assert_eq!(owner(&t, "s").as_deref(), Some("a/S"));
        assert_eq!(owner(&t, "none"), None);
        assert_eq!(t.resolve_field(c, "x", "J").map(|_| ()), None);
    }

    #[test]
    fn names_and_arrays_over_the_limits_do_not_load() {
        let mut t = table_of(vec![NativeClass::new("a/A")]);
        let err = t.load(&"a".repeat(MAX_CLASS_NAME + 1)).unwrap_err();
        assert_eq!(err.exception, "java/lang/NoClassDefFoundError");
        assert_eq!((err.message.len(), err.missing), (103, None));
        let err = t.load("a/A").and_then(|_| t.load(&"b".repeat(100_000)));
        assert_eq!(err.unwrap_err().message.len(), 103);

        let deep = |dims: usize| format!("{}I", "[".repeat(dims));
        for dims in [MAX_ARRAY_DIMS + 1, 40_000] {
            let err = t.load(&deep(dims)).unwrap_err();
            assert_eq!(err.exception, "java/lang/ClassFormatError");
            assert_eq!(err.missing, None);
        }
        let err = t.load(&format!("[L{};", "c".repeat(MAX_CLASS_NAME + 1)));
        assert_eq!(err.unwrap_err().exception, "java/lang/NoClassDefFoundError");
        let err = t.load(&format!("{}La/A;", "[".repeat(MAX_ARRAY_DIMS + 1)));
        assert_eq!(err.unwrap_err().exception, "java/lang/ClassFormatError");

        // The limit itself loads, each dimension its own class.
        let id = t.load(&deep(MAX_ARRAY_DIMS)).unwrap();
        assert_eq!(t.get(id).name.len(), MAX_ARRAY_DIMS + 1);
        for dims in 1..=MAX_ARRAY_DIMS {
            assert!(t.lookup(&deep(dims)).is_some(), "{dims} dimensions");
        }
        let a = t
            .load(&format!("{}La/A;", "[".repeat(MAX_ARRAY_DIMS)))
            .unwrap();
        let inner = t.load("[[La/A;").unwrap();
        assert_ne!(a, inner);
        assert_eq!(
            t.get(a).kind,
            ClassKind::RefArray(
                t.lookup(&format!("{}La/A;", "[".repeat(MAX_ARRAY_DIMS - 1)))
                    .unwrap()
            )
        );
        // A name with no end, and an empty one, are just not found.
        assert!(t.load("[La/A").is_err());
        assert!(t.load("[L;").is_err());
        assert!(t.load("[[[").is_err());
    }

    /// Natives `n0` to `n<len - 1>`, each extending the one before.
    fn native_chain(len: usize) -> ClassTable {
        let natives = (0..len)
            .map(|i| match i {
                0 => NativeClass::new("n0"),
                _ => NativeClass::new(&format!("n{i}")).extends(&format!("n{}", i - 1)),
            })
            .collect();
        table_of(natives)
    }

    /// Runs `f` on a thread with the stack of a virtual machine thread.
    fn on_vm_stack(f: impl FnOnce() + Send + 'static) {
        std::thread::Builder::new()
            .stack_size(16 * 1024 * 1024)
            .spawn(f)
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn a_hierarchy_over_the_limit_does_not_load_in_either_order() {
        on_vm_stack(|| {
            // The leaf first: the load of each class waits for its
            // superclass's, and nothing is left behind.
            let mut t = native_chain(400);
            let before = t.classes.len();
            let err = t.load("n399").unwrap_err();
            assert_eq!(err.exception, "java/lang/ClassFormatError");
            assert!(err.message.ends_with("classes deep"), "{}", err.message);
            assert_eq!(t.classes.len(), before);
            assert!(t.load("n100").is_ok());

            // The root first: no load nests, and the hierarchy of 256
            // classes (with Object) is the last that loads.
            let mut t = native_chain(400);
            let failed: Vec<usize> = (0..400)
                .filter(|i| t.load(&format!("n{i}")).is_err())
                .collect();
            assert_eq!(failed, (MAX_HIERARCHY_DEPTH - 1..400).collect::<Vec<_>>());
            let last = t.lookup(&format!("n{}", MAX_HIERARCHY_DEPTH - 2)).unwrap();
            assert_eq!(t.get(last).supers.len(), MAX_HIERARCHY_DEPTH);
        });
    }

    /// Natives for the interfaces `n0` to `n<len - 1>`, each extending the
    /// one before.
    fn interface_chain(len: usize) -> Vec<NativeClass> {
        (0..len)
            .map(|i| match i {
                0 => NativeClass::interface("n0"),
                _ => NativeClass::interface(&format!("n{i}")).implements(&format!("n{}", i - 1)),
            })
            .collect()
    }

    #[test]
    fn an_interface_chain_over_the_limit_does_not_load_in_either_order() {
        on_vm_stack(|| {
            let mut t = table_of(interface_chain(400));
            let err = t.load("n399").unwrap_err();
            assert_eq!(err.exception, "java/lang/ClassFormatError");
            assert!(t.load("n100").is_ok());

            // The root first nests no load, and the list of interfaces of
            // each interface would grow with the chain.
            let mut t = table_of(interface_chain(400));
            let failed: Vec<usize> = (0..400)
                .filter(|i| t.load(&format!("n{i}")).is_err())
                .collect();
            assert_eq!(failed, (MAX_HIERARCHY_DEPTH - 1..400).collect::<Vec<_>>());
            let last = t.lookup(&format!("n{}", MAX_HIERARCHY_DEPTH - 2)).unwrap();
            assert_eq!(t.get(last).all_interfaces.len(), MAX_HIERARCHY_DEPTH - 2);
        });
    }

    #[test]
    fn a_class_counts_the_depth_of_its_interfaces() {
        on_vm_stack(|| {
            let mut natives = interface_chain(255);
            natives.push(NativeClass::new("fits").implements("n253"));
            natives.push(NativeClass::new("deep").implements("n254"));
            let mut t = table_of(natives);
            // `n253` has 255 classes on its path, with Object.
            let fits = t.load("fits").unwrap();
            assert_eq!(usize::from(t.get(fits).height), MAX_HIERARCHY_DEPTH);
            let err = t.load("deep").unwrap_err();
            assert_eq!(err.exception, "java/lang/ClassFormatError");
        });
    }

    #[test]
    fn missing_and_circular_classes_fail() {
        let mut t = table();
        let err = t.load("com/nokia/mid/ui/FullCanvas").unwrap_err();
        assert_eq!(err.exception, "java/lang/NoClassDefFoundError");
        assert_eq!(err.missing.as_deref(), Some("com/nokia/mid/ui/FullCanvas"));
        let err = t.load("a/Loop").unwrap_err();
        assert_eq!(err.exception, "java/lang/ClassCircularityError");
        assert!(t.load("[Q").is_err());
    }

    /// A class file `name` with `statics` static `int` fields and the
    /// abstract method `m` with the descriptor `desc`.
    fn class_file(name: &str, statics: usize, desc: &str) -> ClassFile {
        use crate::classfile::{ConstantPool, FieldInfo, MethodInfo};
        ClassFile {
            minor_version: 0,
            major_version: 49,
            constant_pool: ConstantPool::default(),
            access: access::PUBLIC | access::SUPER,
            name: name.into(),
            super_name: Some("java/lang/Object".into()),
            interfaces: Vec::new(),
            fields: (0..statics)
                .map(|i| FieldInfo {
                    access: access::STATIC,
                    name: format!("f{i}").into(),
                    descriptor: "I".into(),
                    constant_value: None,
                })
                .collect(),
            methods: vec![MethodInfo {
                access: access::PUBLIC | access::ABSTRACT,
                name: "m".into(),
                descriptor: desc.into(),
                code: None,
            }],
            source_file: None,
        }
    }

    #[test]
    fn a_class_that_does_not_link_leaves_nothing_behind() {
        let mut t = table_of(Vec::new());
        t.load("java/lang/Object").unwrap();
        let (statics, selectors, used) = (t.statics.len(), t.selector_count, t.budget.used);
        // A game that loads a class like this in a loop would otherwise
        // add its static fields to the table's storage every time.
        for _ in 0..3 {
            let def = super::super::link::from_class_file(class_file("a/Bad", 100, "(")).unwrap();
            let err = t.define(def).unwrap_err();
            assert_eq!(err.exception, "java/lang/ClassFormatError");
        }
        assert_eq!(t.statics.len(), statics);
        assert_eq!(t.selector_count, selectors);
        assert_eq!(t.budget.used, used);
        assert!(t.lookup("a/Bad").is_none());

        let def = super::super::link::from_class_file(class_file("a/Good", 100, "()V")).unwrap();
        t.define(def).unwrap();
        assert_eq!(t.statics.len(), statics + 100);
    }
}
