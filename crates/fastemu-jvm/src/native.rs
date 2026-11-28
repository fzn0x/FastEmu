//! Defining Java classes in Rust.
//!
//! A [`NativeClass`] describes a class the way a class file would (name,
//! superclass, interfaces, fields, methods) but with Rust functions as the
//! method bodies. The virtual machine links it on first use, like any
//! other class, so bytecode can extend it, call it and override it.
//!
//! A built class can also be read back ([`NativeClass::fields`],
//! [`NativeClass::methods`] and the rest), so tools can list the API the
//! runtime offers without starting a virtual machine.

use std::sync::Arc;

use crate::class::{ConstValue, PayloadFactory};
use crate::classfile::access;
use crate::ctx::{Ctx, JResult};
use crate::heap::Payload;
use crate::value::Value;

/// The body of a native method.
///
/// It gets the context of the calling thread and the arguments, one
/// [`Value`] per parameter (a `long` is one value here, not two slots),
/// `this` first for instance methods. It returns the result
/// ([`Value::Void`] for `void` methods) or the exception to throw.
pub type NativeFn = Arc<dyn Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync>;

/// A field of a [`NativeClass`], as [`NativeClass::fields`] describes it.
pub struct NativeField {
    pub(crate) name: String,
    pub(crate) desc: String,
    pub(crate) access: u16,
    pub(crate) constant: Option<ConstValue>,
}

impl NativeField {
    /// The field's name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The field's descriptor, such as `I` or `Ljava/lang/String;`.
    pub fn descriptor(&self) -> &str {
        &self.desc
    }

    /// The access flags (see [`crate::classfile::access`]).
    pub fn flags(&self) -> u16 {
        self.access
    }

    /// The compile-time constant, for a field a `ConstantValue` attribute
    /// would initialize.
    pub fn constant(&self) -> Option<FieldConstant<'_>> {
        self.constant.as_ref().map(|c| match c {
            ConstValue::Value(v) => FieldConstant::Value(*v),
            ConstValue::String(s) => FieldConstant::String(s),
        })
    }
}

/// The value of a constant field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FieldConstant<'a> {
    /// A primitive: [`Value::Int`] for `int`, `short`, `char`, `byte` and
    /// `boolean`, or a `Long`, `Float` or `Double`.
    Value(Value),
    /// A `String`, as UTF-16.
    String(&'a [u16]),
}

/// A method or constructor of a [`NativeClass`], as
/// [`NativeClass::methods`] describes it.
pub struct NativeMethod {
    pub(crate) name: String,
    pub(crate) desc: String,
    pub(crate) access: u16,
    /// `None` for abstract methods.
    pub(crate) body: Option<NativeFn>,
    /// The method does nothing and returns nothing (see
    /// [`NativeClass::empty_method`]).
    pub(crate) empty: bool,
}

impl NativeMethod {
    /// The method's name: `<init>` for a constructor, `<clinit>` for the
    /// static initializer.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The method's descriptor, such as `(II)V`.
    pub fn descriptor(&self) -> &str {
        &self.desc
    }

    /// The access flags (see [`crate::classfile::access`]); abstract
    /// methods have `ABSTRACT`.
    pub fn flags(&self) -> u16 {
        self.access
    }
}

/// A Java class implemented in Rust.
///
/// ```
/// use fastemu_jvm::{NativeClass, Value};
///
/// let counter = NativeClass::new("demo/Counter")
///     .field("count", "I")
///     .method("<init>", "()V", |_ctx, _args| Ok(Value::Void))
///     .static_method("twice", "(I)I", |_ctx, args| Ok(Value::Int(args[0].int() * 2)));
/// assert_eq!(counter.name(), "demo/Counter");
/// ```
pub struct NativeClass {
    pub(crate) name: String,
    pub(crate) super_name: Option<String>,
    pub(crate) interfaces: Vec<String>,
    pub(crate) access: u16,
    pub(crate) fields: Vec<NativeField>,
    pub(crate) methods: Vec<NativeMethod>,
    pub(crate) payload: Option<PayloadFactory>,
}

impl NativeClass {
    /// A public class named `name` (an internal name such as
    /// `javax/microedition/lcdui/Canvas`) extending `java/lang/Object`.
    pub fn new(name: &str) -> Self {
        Self {
            super_name: (name != "java/lang/Object").then(|| "java/lang/Object".to_owned()),
            name: name.to_owned(),
            interfaces: Vec::new(),
            access: access::PUBLIC | access::SUPER,
            fields: Vec::new(),
            methods: Vec::new(),
            payload: None,
        }
    }

    /// A public interface named `name`.
    pub fn interface(name: &str) -> Self {
        let mut class = Self::new(name);
        class.access = access::PUBLIC | access::INTERFACE | access::ABSTRACT;
        class
    }

    /// The internal name of the class.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The internal name of the superclass; `None` only for
    /// `java/lang/Object`.
    pub fn super_name(&self) -> Option<&str> {
        self.super_name.as_deref()
    }

    /// The internal names of the interfaces the class implements (or, for
    /// an interface, extends), in the order they were added.
    pub fn interfaces(&self) -> impl Iterator<Item = &str> {
        self.interfaces.iter().map(String::as_str)
    }

    /// The class access flags (see [`crate::classfile::access`]).
    pub fn flags(&self) -> u16 {
        self.access
    }

    /// The fields, in the order they were added.
    pub fn fields(&self) -> &[NativeField] {
        &self.fields
    }

    /// The methods and constructors, in the order they were added, the
    /// static initializer among them.
    pub fn methods(&self) -> &[NativeMethod] {
        &self.methods
    }

    /// Sets the superclass.
    #[must_use]
    pub fn extends(mut self, super_name: &str) -> Self {
        self.super_name = Some(super_name.to_owned());
        self
    }

    /// Adds an interface the class implements.
    #[must_use]
    pub fn implements(mut self, interface: &str) -> Self {
        self.interfaces.push(interface.to_owned());
        self
    }

    /// Replaces the class access flags (see [`crate::classfile::access`]).
    #[must_use]
    pub fn access(mut self, flags: u16) -> Self {
        self.access = flags;
        self
    }

    /// Marks the class abstract: `new` on it throws `InstantiationError`.
    #[must_use]
    pub fn abstract_class(mut self) -> Self {
        self.access |= access::ABSTRACT;
        self
    }

    /// Marks the class final.
    #[must_use]
    pub fn final_class(mut self) -> Self {
        self.access |= access::FINAL;
        self
    }

    /// A public instance field.
    #[must_use]
    pub fn field(self, name: &str, desc: &str) -> Self {
        self.field_with(name, desc, access::PUBLIC)
    }

    /// An instance or static field with explicit access flags.
    #[must_use]
    pub fn field_with(mut self, name: &str, desc: &str, access: u16) -> Self {
        self.fields.push(NativeField {
            name: name.to_owned(),
            desc: desc.to_owned(),
            access,
            constant: None,
        });
        self
    }

    /// A public static field.
    #[must_use]
    pub fn static_field(self, name: &str, desc: &str) -> Self {
        self.field_with(name, desc, access::PUBLIC | access::STATIC)
    }

    /// A `public static final` field with a constant value, as a
    /// `ConstantValue` attribute would give it.
    #[must_use]
    pub fn constant(self, name: &str, desc: &str, value: Value) -> Self {
        self.constant_with(name, desc, value, access::PUBLIC)
    }

    /// A `static final` field with a constant value and the visibility in
    /// `access`, such as [`access::PROTECTED`] for the constants a class
    /// declares for its subclasses.
    #[must_use]
    pub fn constant_with(mut self, name: &str, desc: &str, value: Value, access: u16) -> Self {
        self.fields.push(NativeField {
            name: name.to_owned(),
            desc: desc.to_owned(),
            access: access | access::STATIC | access::FINAL,
            constant: Some(ConstValue::Value(value)),
        });
        self
    }

    /// A `public static final String` constant.
    #[must_use]
    pub fn string_constant(mut self, name: &str, value: &str) -> Self {
        self.fields.push(NativeField {
            name: name.to_owned(),
            desc: "Ljava/lang/String;".to_owned(),
            access: access::PUBLIC | access::STATIC | access::FINAL,
            constant: Some(ConstValue::String(value.encode_utf16().collect())),
        });
        self
    }

    /// A public instance method.
    #[must_use]
    pub fn method(
        self,
        name: &str,
        desc: &str,
        f: impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static,
    ) -> Self {
        self.method_with(name, desc, access::PUBLIC, f)
    }

    /// A public static method.
    #[must_use]
    pub fn static_method(
        self,
        name: &str,
        desc: &str,
        f: impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static,
    ) -> Self {
        self.method_with(name, desc, access::PUBLIC | access::STATIC, f)
    }

    /// A method with explicit access flags. Add
    /// [`access::SYNCHRONIZED`] to have the virtual machine hold the
    /// monitor of `this` (or of the class, for static methods) around it.
    #[must_use]
    pub fn method_with(
        mut self,
        name: &str,
        desc: &str,
        access: u16,
        f: impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static,
    ) -> Self {
        self.methods.push(NativeMethod {
            name: name.to_owned(),
            desc: desc.to_owned(),
            access,
            body: Some(Arc::new(f)),
            empty: false,
        });
        self
    }

    /// A public method that does nothing and returns nothing. The
    /// interpreter calls it without a native frame: `Object.<init>`, which
    /// every constructor ends in, is one.
    #[must_use]
    pub(crate) fn empty_method(mut self, name: &str, desc: &str) -> Self {
        self.methods.push(NativeMethod {
            name: name.to_owned(),
            desc: desc.to_owned(),
            access: access::PUBLIC,
            body: Some(Arc::new(|_, _| Ok(Value::Void))),
            empty: true,
        });
        self
    }

    /// A public abstract method, for subclasses (game classes or other
    /// native classes) to implement.
    #[must_use]
    pub fn abstract_method(self, name: &str, desc: &str) -> Self {
        self.abstract_method_with(name, desc, access::PUBLIC)
    }

    /// An abstract method with explicit access flags, such as a
    /// `protected abstract` one; `ABSTRACT` is added to them.
    #[must_use]
    pub fn abstract_method_with(mut self, name: &str, desc: &str, access: u16) -> Self {
        self.methods.push(NativeMethod {
            name: name.to_owned(),
            desc: desc.to_owned(),
            access: access | access::ABSTRACT,
            body: None,
            empty: false,
        });
        self
    }

    /// The static initializer, run once when the class initializes (after
    /// its superclass and its constant fields).
    #[must_use]
    pub fn static_init(
        self,
        f: impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static,
    ) -> Self {
        self.method_with("<clinit>", "()V", access::STATIC, f)
    }

    /// Gives every new instance (of this class and its subclasses) a
    /// default payload of type `T`.
    #[must_use]
    pub fn payload<T: Payload + Default>(mut self) -> Self {
        self.payload = Some(Arc::new(|| Box::new(T::default())));
        self
    }

    /// Gives every new instance the payload `f` makes.
    #[must_use]
    pub fn payload_with(
        mut self,
        f: impl Fn() -> Box<dyn Payload> + Send + Sync + 'static,
    ) -> Self {
        self.payload = Some(Arc::new(f));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_built_class_reads_back() {
        let class = NativeClass::interface("demo/Shape")
            .implements("demo/Named")
            .constant("SIDES", "I", Value::Int(4))
            .string_constant("NAME", "sq\u{e9}")
            .abstract_method("area", "()I");
        assert_eq!(class.super_name(), Some("java/lang/Object"));
        assert_eq!(class.interfaces().collect::<Vec<_>>(), ["demo/Named"]);
        assert_ne!(class.flags() & access::INTERFACE, 0);
        let fields = class.fields();
        assert_eq!(fields[0].name(), "SIDES");
        assert_eq!(fields[0].descriptor(), "I");
        assert_eq!(
            fields[0].flags(),
            access::PUBLIC | access::STATIC | access::FINAL
        );
        assert_eq!(
            fields[0].constant(),
            Some(FieldConstant::Value(Value::Int(4)))
        );
        let name: Vec<u16> = "sq\u{e9}".encode_utf16().collect();
        assert_eq!(fields[1].constant(), Some(FieldConstant::String(&name)));
        let method = &class.methods()[0];
        assert_eq!((method.name(), method.descriptor()), ("area", "()I"));
        assert_eq!(method.flags(), access::PUBLIC | access::ABSTRACT);
    }

    #[test]
    fn object_has_no_superclass() {
        let object = NativeClass::new("java/lang/Object")
            .field_with("lock", "I", access::PRIVATE)
            .static_init(|_, _| Ok(Value::Void));
        assert_eq!(object.super_name(), None);
        assert_eq!(object.fields()[0].constant(), None);
        assert_eq!(object.fields()[0].flags(), access::PRIVATE);
        assert_eq!(object.methods()[0].name(), "<clinit>");
        assert_eq!(object.methods()[0].flags(), access::STATIC);
    }
}
