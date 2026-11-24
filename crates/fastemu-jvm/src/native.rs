//! Defining Java classes in Rust.
//!
//! A [`NativeClass`] describes a class the way a class file would (name,
//! superclass, interfaces, fields, methods) but with Rust functions as the
//! method bodies. The virtual machine links it on first use, like any
//! other class, so bytecode can extend it, call it and override it.

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

pub(crate) struct NativeField {
    pub(crate) name: String,
    pub(crate) desc: String,
    pub(crate) access: u16,
    pub(crate) constant: Option<ConstValue>,
}

pub(crate) struct NativeMethod {
    pub(crate) name: String,
    pub(crate) desc: String,
    pub(crate) access: u16,
    /// `None` for abstract methods.
    pub(crate) body: Option<NativeFn>,
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
    pub fn constant(mut self, name: &str, desc: &str, value: Value) -> Self {
        self.fields.push(NativeField {
            name: name.to_owned(),
            desc: desc.to_owned(),
            access: access::PUBLIC | access::STATIC | access::FINAL,
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
        });
        self
    }

    /// A public abstract method, for subclasses (game classes or other
    /// native classes) to implement.
    #[must_use]
    pub fn abstract_method(mut self, name: &str, desc: &str) -> Self {
        self.methods.push(NativeMethod {
            name: name.to_owned(),
            desc: desc.to_owned(),
            access: access::PUBLIC | access::ABSTRACT,
            body: None,
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
