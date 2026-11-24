//! Objects, strings, arrays, fields, payloads and classes, for natives.

use fastemu_core::LogLevel;

use super::{Ctx, JResult, Throw};
use crate::class::{ArrayType, ClassId, ClassKind, LoadError};
use crate::heap::{Body, Heap, HeapObject, Payload};
use crate::lang;
use crate::value::{ObjRef, Value};
use crate::vm::{Diagnostic, VmState};

mod sealed {
    use crate::heap::Body;

    pub trait Sealed: Sized {
        fn slice(body: &Body) -> Option<&[Self]>;
        fn slice_mut(body: &mut Body) -> Option<&mut [Self]>;
    }
}

/// An array element type natives can view arrays as: `i8` for `boolean[]`
/// and `byte[]`, `u16` for `char[]`, `i16`, `i32`, `i64`, `f32`, `f64`,
/// and `Option<ObjRef>` for arrays of references.
///
/// References read from an `Option<ObjRef>` slice are not rooted; pass
/// any you keep across a call into Java or a wait to [`Ctx::root`].
pub trait ArrayElem: sealed::Sealed + Copy {}

macro_rules! array_elem {
    ($t:ty, $variant:ident) => {
        impl sealed::Sealed for $t {
            fn slice(body: &Body) -> Option<&[Self]> {
                match body {
                    Body::$variant(a) => Some(a),
                    _ => None,
                }
            }
            fn slice_mut(body: &mut Body) -> Option<&mut [Self]> {
                match body {
                    Body::$variant(a) => Some(a),
                    _ => None,
                }
            }
        }
        impl ArrayElem for $t {}
    };
}

array_elem!(i8, Bytes);
array_elem!(u16, Chars);
array_elem!(i16, Shorts);
array_elem!(i32, Ints);
array_elem!(i64, Longs);
array_elem!(f32, Floats);
array_elem!(f64, Doubles);
array_elem!(Option<ObjRef>, Refs);

/// A resolved instance field, for fast repeated access from natives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FieldId {
    pub(crate) class: ClassId,
    pub(crate) slot: u32,
}

/// A resolved static field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StaticFieldId {
    pub(crate) class: ClassId,
    pub(crate) index: u32,
}

impl VmState {
    /// A new instance of `class` with default field values and its
    /// payload, without running a constructor.
    pub(crate) fn alloc_instance(&mut self, class: ClassId) -> ObjRef {
        let c = self.classes.get(class);
        let obj = HeapObject::new(
            class,
            Body::Fields(c.template.clone()),
            c.payload.as_ref().map(|make| make()),
        );
        self.heap.alloc(obj)
    }

    /// A new array of `len` primitives.
    pub(crate) fn alloc_prim_array(&mut self, t: ArrayType, len: usize) -> ObjRef {
        let class = self.classes.prim_array(t);
        let body = match t {
            ArrayType::Boolean | ArrayType::Byte => Body::Bytes(vec![0; len].into()),
            ArrayType::Char => Body::Chars(vec![0; len].into()),
            ArrayType::Short => Body::Shorts(vec![0; len].into()),
            ArrayType::Int => Body::Ints(vec![0; len].into()),
            ArrayType::Long => Body::Longs(vec![0; len].into()),
            ArrayType::Float => Body::Floats(vec![0.0; len].into()),
            ArrayType::Double => Body::Doubles(vec![0.0; len].into()),
        };
        self.alloc_body(class, body)
    }

    /// A new array of `len` null references whose array class is `class`.
    pub(crate) fn alloc_ref_array(&mut self, array_class: ClassId, len: usize) -> ObjRef {
        self.alloc_body(array_class, Body::Refs(vec![None; len].into()))
    }

    pub(crate) fn alloc_body(&mut self, class: ClassId, body: Body) -> ObjRef {
        self.heap.alloc(HeapObject::new(class, body, None))
    }

    /// Reports a diagnostic once and logs it.
    pub(crate) fn report(&mut self, shared: &crate::vm::VmShared, d: Diagnostic) {
        if self.reported.insert(d.clone()) {
            shared.log(LogLevel::Warn, d.to_string());
            if let Some(sink) = &shared.config.diagnostics {
                sink(&d);
            }
        }
    }
}

impl Ctx<'_> {
    /// Turns a class loading failure into the Java error to throw,
    /// reporting a missing class to the diagnostics hook.
    pub(crate) fn load_error(&mut self, e: LoadError) -> Throw {
        if let Some(missing) = e.missing {
            self.state
                .report(self.shared, Diagnostic::MissingClass(missing));
        }
        self.new_exception(e.exception, Some(&e.message.replace('/', ".")))
    }

    /// Loads (without initializing) the class named `name`, an internal
    /// name such as `java/lang/String` or an array descriptor such as `[I`.
    /// Throws `NoClassDefFoundError` when no source has it.
    pub fn find_class(&mut self, name: &str) -> JResult<ClassId> {
        match self.state.classes.load(name) {
            Ok(id) => Ok(id),
            Err(e) => Err(self.load_error(e)),
        }
    }

    /// The internal name of `class`.
    pub fn class_name(&self, class: ClassId) -> &str {
        &self.state.classes.get(class).name
    }

    /// The runtime class of `obj`.
    pub fn class_of(&self, obj: ObjRef) -> ClassId {
        self.state.heap.class_of(obj)
    }

    /// The superclass of `class`, `None` for `java.lang.Object` and
    /// interfaces' own super, which is `Object`.
    pub fn superclass(&self, class: ClassId) -> Option<ClassId> {
        self.state.classes.get(class).superclass
    }

    /// Whether a `from` can be used where a `to` is expected.
    pub fn is_assignable(&self, from: ClassId, to: ClassId) -> bool {
        self.state.classes.is_assignable(from, to)
    }

    /// `obj instanceof class`.
    pub fn is_instance(&self, obj: ObjRef, class: ClassId) -> bool {
        self.is_assignable(self.class_of(obj), class)
    }

    /// The `java.lang.Class` object of `class`.
    pub fn class_object(&mut self, class: ClassId) -> ObjRef {
        let mirror = lang::class::mirror(&mut self.state, class);
        self.root(mirror)
    }

    /// The class a `java.lang.Class` object stands for.
    pub fn class_from_object(&self, mirror: ObjRef) -> Option<ClassId> {
        lang::class::mirrored(&self.state, mirror)
    }

    /// `v` as a non-null reference, or a `NullPointerException`.
    pub fn non_null(&mut self, v: Value) -> JResult<ObjRef> {
        match v {
            Value::Ref(Some(r)) => Ok(r),
            _ => Err(self.new_exception("java/lang/NullPointerException", None)),
        }
    }

    /// A new instance of `class`, initializing the class first, with
    /// default field values and no constructor run. Natives that need a
    /// constructor use [`Ctx::construct`].
    pub fn new_object(&mut self, class: ClassId) -> JResult<ObjRef> {
        let c = self.state.classes.get(class);
        if c.is_abstract() || c.is_interface() || c.is_array() {
            let name = c.name.replace('/', ".");
            return Err(self.new_exception("java/lang/InstantiationError", Some(&name)));
        }
        self.initialize(class)?;
        let obj = self.state.alloc_instance(class);
        Ok(self.root(obj))
    }

    /// A new `java.lang.String`. Throws `OutOfMemoryError` when its
    /// characters would not fit, as [`Ctx::check_alloc`] decides.
    pub fn new_string(&mut self, s: &str) -> JResult<ObjRef> {
        self.new_string_utf16(s.encode_utf16().collect())
    }

    /// A new `java.lang.String` from UTF-16 code units. Throws
    /// `OutOfMemoryError` when they would not fit, as [`Ctx::check_alloc`]
    /// decides. A native that builds `chars` from something it can size
    /// first checks that size before it builds them.
    pub fn new_string_utf16(&mut self, chars: Vec<u16>) -> JResult<ObjRef> {
        self.check_alloc(lang::string::string_bytes(chars.len()))?;
        let obj = lang::string::new_string(&mut self.state, chars);
        Ok(self.root(obj))
    }

    /// The canonical (interned) string with these characters.
    pub fn intern(&mut self, chars: &[u16]) -> ObjRef {
        let obj = lang::string::intern(&mut self.state, chars);
        self.root(obj)
    }

    /// The characters of the string `obj`.
    ///
    /// # Panics
    ///
    /// When `obj` is not a `java.lang.String`; a native's descriptor fixes
    /// which arguments are strings, so this is a bug in the native.
    pub fn string_utf16(&self, obj: ObjRef) -> JResult<&[u16]> {
        match lang::string::chars(&self.state, obj) {
            Some(chars) => Ok(chars),
            None => panic!(
                "a native read {} as a java.lang.String",
                self.class_name(self.class_of(obj))
            ),
        }
    }

    /// The string `obj` as Rust text (unpaired surrogates become U+FFFD).
    pub fn string(&self, obj: ObjRef) -> JResult<String> {
        Ok(String::from_utf16_lossy(self.string_utf16(obj)?))
    }

    /// The string `v` refers to, or `None` for `null`.
    pub fn opt_string(&self, v: Value) -> JResult<Option<String>> {
        match v.obj() {
            Some(obj) => self.string(obj).map(Some),
            None => Ok(None),
        }
    }

    /// Throws `OutOfMemoryError` when a native should not make an object
    /// of `bytes` bytes now: when `bytes` is more than the whole heap, or
    /// when it and what the heap counts now come to more than twice the
    /// heap.
    ///
    /// Call it before building a large host buffer (a `Vec`, a decoded
    /// image, a resource) that will become an object's data, so a request
    /// the heap could never hold fails before the host allocates it.
    /// `bytes` is what the object will count: its array size, or its
    /// payload's `heap_bytes`.
    ///
    /// The check cannot collect, because a native may hold references the
    /// collector does not see. The count may therefore include garbage,
    /// and the bound is twice the limit. A native that stays under it can
    /// still leave the heap over its limit: the interpreter then collects
    /// when the native returns, and throws `OutOfMemoryError` into the
    /// thread that ran it if the live data still does not fit.
    #[inline]
    pub fn check_alloc(&mut self, bytes: usize) -> JResult<()> {
        if self.state.heap.refuses(bytes) {
            return Err(self.out_of_memory());
        }
        Ok(())
    }

    fn check_array_len(&mut self, len: i32, elem_bytes: usize) -> JResult<usize> {
        if len < 0 {
            return Err(self.new_exception(
                "java/lang/NegativeArraySizeException",
                Some(&len.to_string()),
            ));
        }
        self.check_alloc(Heap::array_bytes(len as usize, elem_bytes))?;
        Ok(len as usize)
    }

    /// A new array of `len` zeroed primitives.
    pub fn new_array(&mut self, t: ArrayType, len: i32) -> JResult<ObjRef> {
        let len = self.check_array_len(len, t.elem_bytes())?;
        let obj = self.state.alloc_prim_array(t, len);
        Ok(self.root(obj))
    }

    /// A new array of `len` nulls whose elements are of class `elem`.
    pub fn new_object_array(&mut self, elem: ClassId, len: i32) -> JResult<ObjRef> {
        let len = self.check_array_len(len, 4)?;
        let class = match self.state.classes.array_of(elem) {
            Ok(c) => c,
            Err(e) => return Err(self.load_error(e)),
        };
        let obj = self.state.alloc_ref_array(class, len);
        Ok(self.root(obj))
    }

    /// A new `byte[]` holding `bytes`.
    pub fn new_byte_array(&mut self, bytes: &[u8]) -> JResult<ObjRef> {
        let len = self.check_array_len(bytes.len() as i32, 1)?;
        let class = self.state.classes.prim_array(ArrayType::Byte);
        let data: Box<[i8]> = bytes.iter().map(|&b| b as i8).collect();
        debug_assert_eq!(data.len(), len);
        let obj = self.state.alloc_body(class, Body::Bytes(data));
        Ok(self.root(obj))
    }

    /// The length of array `arr`.
    pub fn array_len(&self, arr: ObjRef) -> JResult<usize> {
        match self.state.heap.get(arr).body.array_len() {
            Some(len) => Ok(len),
            None => panic!("a native treated a non-array object as an array"),
        }
    }

    /// The elements of `arr` as a slice of `T`.
    ///
    /// # Panics
    ///
    /// When `arr` is not an array of `T`; a native's descriptor fixes the
    /// array types it receives, so this is a bug in the native.
    pub fn array<T: ArrayElem>(&self, arr: ObjRef) -> JResult<&[T]> {
        match T::slice(&self.state.heap.get(arr).body) {
            Some(s) => Ok(s),
            None => panic!(
                "a native read {} as an array of {}",
                self.class_name(self.class_of(arr)),
                std::any::type_name::<T>()
            ),
        }
    }

    /// The elements of `arr` as a mutable slice of `T`.
    ///
    /// # Panics
    ///
    /// As [`Ctx::array`]. Storing a reference whose class does not fit the
    /// array's element class breaks Java's type rules; check with
    /// [`Ctx::is_instance`] first.
    pub fn array_mut<T: ArrayElem>(&mut self, arr: ObjRef) -> JResult<&mut [T]> {
        let heap = &mut self.state.heap;
        if T::slice_mut(&mut heap.get_mut(arr).body).is_none() {
            let class = self.state.heap.class_of(arr);
            panic!(
                "a native wrote {} as an array of {}",
                self.class_name(class),
                std::any::type_name::<T>()
            );
        }
        Ok(T::slice_mut(&mut self.state.heap.get_mut(arr).body).expect("checked above"))
    }

    /// The element class of array class `class`, if it is an array of
    /// references.
    pub fn array_element_class(&self, class: ClassId) -> Option<ClassId> {
        match self.state.classes.get(class).kind {
            ClassKind::RefArray(elem) => Some(elem),
            _ => None,
        }
    }

    /// Resolves the instance field `name` of type `desc` in `class` or a
    /// superclass. Throws `NoSuchFieldError` when there is none.
    pub fn field_id(&mut self, class: &str, name: &str, desc: &str) -> JResult<FieldId> {
        let cid = self.find_class(class)?;
        let found = self
            .state
            .classes
            .resolve_field(cid, name, desc)
            .filter(|(_, f)| !f.is_static())
            .map(|(owner, f)| FieldId {
                class: owner,
                slot: f.slot,
            });
        match found {
            Some(id) => Ok(id),
            None => Err(self.missing_field(class, name, desc)),
        }
    }

    /// Resolves a static field. Throws `NoSuchFieldError` when there is
    /// none.
    pub fn static_field_id(
        &mut self,
        class: &str,
        name: &str,
        desc: &str,
    ) -> JResult<StaticFieldId> {
        let cid = self.find_class(class)?;
        let found = self
            .state
            .classes
            .resolve_field(cid, name, desc)
            .filter(|(_, f)| f.is_static())
            .map(|(owner, f)| StaticFieldId {
                class: owner,
                index: f.slot,
            });
        match found {
            Some(id) => Ok(id),
            None => Err(self.missing_field(class, name, desc)),
        }
    }

    pub(crate) fn missing_field(&mut self, class: &str, name: &str, desc: &str) -> Throw {
        self.state.report(
            self.shared,
            Diagnostic::MissingField {
                class: class.to_owned(),
                name: name.to_owned(),
                desc: desc.to_owned(),
            },
        );
        self.new_exception("java/lang/NoSuchFieldError", Some(name))
    }

    /// Reads field `field` of `obj`; a reference read is rooted.
    pub fn get_field(&mut self, obj: ObjRef, field: FieldId) -> Value {
        let v = match &self.state.heap.get(obj).body {
            Body::Fields(f) => f.get(field.slot as usize).copied().unwrap_or_default(),
            _ => Value::Void,
        };
        debug_assert!(
            self.state
                .classes
                .is_assignable(self.state.heap.class_of(obj), field.class),
            "a field read on an object of the wrong class"
        );
        self.root_value(v)
    }

    /// Writes field `field` of `obj`.
    pub fn set_field(&mut self, obj: ObjRef, field: FieldId, v: Value) {
        if let Body::Fields(f) = &mut self.state.heap.get_mut(obj).body
            && let Some(slot) = f.get_mut(field.slot as usize)
        {
            *slot = v;
        }
    }

    /// Reads a static field, initializing its class first.
    pub fn get_static(&mut self, field: StaticFieldId) -> JResult<Value> {
        self.initialize(field.class)?;
        let v = self.state.classes.statics[field.index as usize];
        Ok(self.root_value(v))
    }

    /// Writes a static field, initializing its class first.
    pub fn set_static(&mut self, field: StaticFieldId, v: Value) -> JResult<()> {
        self.initialize(field.class)?;
        self.state.classes.statics[field.index as usize] = v;
        Ok(())
    }

    /// The payload of `obj`.
    ///
    /// # Panics
    ///
    /// When `obj` has no payload of type `T`: a native's class fixes what
    /// `this` is, so this is a bug in the native.
    pub fn payload<T: Payload>(&self, obj: ObjRef) -> JResult<&T> {
        match self.state.heap.get(obj).payload::<T>() {
            Some(p) => Ok(p),
            None => panic!(
                "{} has no payload of type {}",
                self.class_name(self.class_of(obj)),
                std::any::type_name::<T>()
            ),
        }
    }

    /// The payload of `obj`, mutably. Panics as [`Ctx::payload`] does.
    pub fn payload_mut<T: Payload>(&mut self, obj: ObjRef) -> JResult<&mut T> {
        assert!(
            self.state.heap.get(obj).payload::<T>().is_some(),
            "{} has no payload of type {}",
            self.class_name(self.class_of(obj)),
            std::any::type_name::<T>()
        );
        Ok(self
            .state
            .heap
            .get_mut(obj)
            .payload_mut::<T>()
            .expect("checked above"))
    }

    /// The payload of `obj` if it has one of type `T`.
    pub fn try_payload<T: Payload>(&self, obj: ObjRef) -> Option<&T> {
        self.state.heap.get(obj).payload::<T>()
    }

    /// Replaces the payload of `obj`. The heap counts the change in size
    /// at once, as it counts an allocation, so a native that fills in an
    /// object after making it (an image's pixels) brings the next
    /// collection closer.
    pub fn set_payload<T: Payload>(&mut self, obj: ObjRef, payload: T) {
        self.state.heap.set_payload(obj, Box::new(payload));
    }

    /// Counts the change in the size of `obj`'s payload since the heap
    /// last counted it, through [`Payload::heap_bytes`]. Call it after
    /// changing the payload with [`Ctx::payload_mut`] in a way that
    /// changes `heap_bytes`: a buffer that grew, a list that took an
    /// item, a name that was set. Without it the heap learns of the
    /// change only at the next collection, and a loop that changes
    /// payloads and allocates nothing else never reaches that.
    ///
    /// It is cheap, a few additions, so call it where the size changed,
    /// not on every call; a payload that grows by doubling calls it once
    /// per doubling. Checking the new size against the heap before the
    /// change is [`Ctx::check_alloc`]'s job. The thread that recounted
    /// gets `OutOfMemoryError` at the next call return if the live data
    /// no longer fits.
    pub fn recount(&mut self, obj: ObjRef) {
        self.state.heap.recount(obj);
    }

    /// A system property, as `System.getProperty` reads it.
    pub fn property(&self, key: &str) -> Option<&str> {
        self.state.properties.get(key).map(String::as_str)
    }

    /// Sets a system property.
    pub fn set_property(&mut self, key: &str, value: &str) {
        self.state
            .properties
            .insert(key.to_owned(), value.to_owned());
    }

    /// Writes a line to the log sink.
    pub fn log(&self, level: LogLevel, text: impl Into<String>) {
        self.shared.log(level, text);
    }

    /// Reports a diagnostic (once per virtual machine) and logs it.
    pub fn report(&mut self, diagnostic: Diagnostic) {
        self.state.report(self.shared, diagnostic);
    }

    /// Host data of type `T` stored in this virtual machine.
    pub fn extension<T: std::any::Any + Send>(&self) -> Option<&T> {
        self.state
            .extensions
            .get(&std::any::TypeId::of::<T>())
            .and_then(|b| b.downcast_ref())
    }

    /// Host data of type `T`, mutably.
    pub fn extension_mut<T: std::any::Any + Send>(&mut self) -> Option<&mut T> {
        self.state
            .extensions
            .get_mut(&std::any::TypeId::of::<T>())
            .and_then(|b| b.downcast_mut())
    }

    /// Stores host data of type `T`, replacing any earlier value. The VM
    /// owns it until it is dropped; it must not hold [`ObjRef`]s (use
    /// [`crate::GlobalRef`]s).
    pub fn set_extension<T: std::any::Any + Send>(&mut self, value: T) {
        self.state
            .extensions
            .insert(std::any::TypeId::of::<T>(), Box::new(value));
    }

    /// The configured heap size and the bytes in use.
    pub fn heap_usage(&self) -> (usize, usize) {
        (self.state.heap.limit(), self.state.heap.used())
    }
}
