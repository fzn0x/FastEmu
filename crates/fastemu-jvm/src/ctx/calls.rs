//! Calling Java methods from Rust.

use std::sync::Arc;

use super::{Ctx, JResult, Throw};
use crate::class::{ClassId, Method, MethodBody};
use crate::interp;
use crate::thread::Frame;
use crate::value::{ObjRef, Value};
use crate::vm::Diagnostic;

impl Ctx<'_> {
    /// Calls `method` with `args` (one value per parameter, `this` first
    /// for instance methods) and returns its result. A reference result,
    /// and an exception thrown out of it, are rooted as native locals.
    pub(crate) fn invoke(&mut self, method: &Arc<Method>, args: &[Value]) -> JResult<Value> {
        let result = self.invoke_unrooted(method, args);
        match result {
            Ok(v) => Ok(self.root_value(v)),
            Err(Throw::Exception(e)) => {
                self.root(e);
                Err(Throw::Exception(e))
            }
            Err(Throw::Stop) => Err(Throw::Stop),
        }
    }

    fn invoke_unrooted(&mut self, method: &Arc<Method>, args: &[Value]) -> JResult<Value> {
        let top = self.thread.top();
        let slots = args
            .iter()
            .map(|a| if a.is_wide() { 2 } else { 1 })
            .sum::<usize>();
        if slots != method.arg_slots as usize {
            let message = format!(
                "{} takes {} argument slots, a native passed {slots}",
                method.describe(),
                method.arg_slots
            );
            return Err(self.new_exception("java/lang/InternalError", Some(&message)));
        }
        self.thread.reserve(top + slots);
        let mut i = top;
        for &a in args {
            self.thread.slots[i] = a;
            i += 1;
            if a.is_wide() {
                self.thread.slots[i] = Value::Void;
                i += 1;
            }
        }
        if self.thread.native_depth >= self.thread.max_native_depth {
            return Err(self.new_exception("java/lang/StackOverflowError", None));
        }
        match &method.body {
            MethodBody::Bytecode(_) => {
                let base = self.thread.frames.len();
                if let Err(fault) = interp::push_frame(self, method, top) {
                    return Err(fault.into_throw(self));
                }
                self.thread.native_depth += 1;
                let result = match self.enter_synchronized(method, args) {
                    Ok(()) => interp::run(self, base),
                    Err(e) => {
                        self.thread.frames.truncate(base);
                        Err(e)
                    }
                };
                self.thread.native_depth -= 1;
                result
            }
            MethodBody::Native(f) => {
                let f = Arc::clone(f);
                self.call_native(method, &f, top, args)
            }
            MethodBody::Abstract => {
                let name = method.describe().replace('/', ".");
                Err(self.new_exception("java/lang/AbstractMethodError", Some(&name)))
            }
            MethodBody::Unlinked => {
                let name = method.describe().replace('/', ".");
                Err(self.new_exception("java/lang/UnsatisfiedLinkError", Some(&name)))
            }
        }
    }

    /// Takes the monitor of a synchronized method's `this` (or class) for
    /// the frame just pushed.
    fn enter_synchronized(&mut self, method: &Arc<Method>, args: &[Value]) -> JResult<()> {
        if !method.is_synchronized() {
            return Ok(());
        }
        let obj = if method.is_static() {
            crate::lang::class::mirror(&mut self.state, method.class)
        } else {
            match args.first().and_then(|a| a.obj()) {
                Some(obj) => obj,
                None => return Err(self.new_exception("java/lang/NullPointerException", None)),
            }
        };
        self.monitor_enter(obj)?;
        if let Some(frame) = self.thread.frames.last_mut() {
            frame.sync = Some(obj);
        }
        Ok(())
    }

    /// Runs native method `method` whose arguments already sit in the slots
    /// from `bp`, with a native frame around it for stack traces, rooting
    /// and synchronization.
    pub(crate) fn call_native(
        &mut self,
        method: &Arc<Method>,
        f: &crate::native::NativeFn,
        bp: usize,
        args: &[Value],
    ) -> JResult<Value> {
        let mark = self.thread.locals.len();
        self.thread.frames.push(Frame {
            method: Arc::clone(method),
            pc: 0,
            bp: bp as u32,
            sp: (bp + method.arg_slots as usize) as u32,
            sync: None,
            native: true,
        });
        let result = match self.enter_synchronized(method, args) {
            Ok(()) => f(self, args),
            Err(e) => Err(e),
        };
        let frame = self
            .thread
            .frames
            .pop()
            .expect("the native frame pushed above");
        if let Some(obj) = frame.sync {
            let _ = self.monitor_exit(obj);
        }
        self.thread.locals.truncate(mark);
        result
    }

    fn lookup_virtual(&mut self, obj: ObjRef, name: &str, desc: &str) -> JResult<Arc<Method>> {
        let class = self.class_of(obj);
        let found = self
            .state
            .classes
            .find_selector(name, desc)
            .and_then(|sel| self.state.classes.get(class).virtual_method(sel))
            .cloned();
        match found {
            Some(m) => Ok(m),
            None => {
                let class = self.class_name(class).to_owned();
                Err(self.missing_method(&class, name, desc))
            }
        }
    }

    pub(crate) fn missing_method(&mut self, class: &str, name: &str, desc: &str) -> Throw {
        self.state.report(
            self.shared,
            Diagnostic::MissingMethod {
                class: class.to_owned(),
                name: name.to_owned(),
                desc: desc.to_owned(),
            },
        );
        let message = format!("{}.{name}{desc}", class.replace('/', "."));
        self.new_exception("java/lang/NoSuchMethodError", Some(&message))
    }

    fn with_this(this: ObjRef, args: &[Value]) -> smallvec::SmallVec<[Value; 8]> {
        let mut all = smallvec::SmallVec::with_capacity(args.len() + 1);
        all.push(Value::from(this));
        all.extend_from_slice(args);
        all
    }

    /// Calls instance method `name` with descriptor `desc` on `obj` through
    /// virtual dispatch, so a game's override runs. `args` excludes `this`.
    pub fn call_virtual(
        &mut self,
        obj: ObjRef,
        name: &str,
        desc: &str,
        args: &[Value],
    ) -> JResult<Value> {
        let method = self.lookup_virtual(obj, name, desc)?;
        self.invoke(&method, &Self::with_this(obj, args))
    }

    /// Calls the static method `name` of class `class` (an internal name),
    /// initializing the class first.
    pub fn call_static(
        &mut self,
        class: &str,
        name: &str,
        desc: &str,
        args: &[Value],
    ) -> JResult<Value> {
        let cid = self.find_class(class)?;
        self.call_static_in(cid, name, desc, args)
    }

    /// Calls the static method `name` of `class`.
    pub fn call_static_in(
        &mut self,
        class: ClassId,
        name: &str,
        desc: &str,
        args: &[Value],
    ) -> JResult<Value> {
        let method = self
            .state
            .classes
            .resolve_method(class, name, desc)
            .filter(|m| m.is_static());
        let Some(method) = method else {
            let class = self.class_name(class).to_owned();
            return Err(self.missing_method(&class, name, desc));
        };
        self.initialize(method.class)?;
        self.invoke(&method, args)
    }

    /// Calls the instance method `name` that class `class` declares or
    /// inherits, without virtual dispatch: `super.name(...)` and
    /// constructors. `args` excludes `this`.
    pub fn call_special(
        &mut self,
        class: &str,
        obj: ObjRef,
        name: &str,
        desc: &str,
        args: &[Value],
    ) -> JResult<Value> {
        let cid = self.find_class(class)?;
        let method = self
            .state
            .classes
            .resolve_method(cid, name, desc)
            .filter(|m| !m.is_static());
        let Some(method) = method else {
            return Err(self.missing_method(class, name, desc));
        };
        self.invoke(&method, &Self::with_this(obj, args))
    }

    /// Creates an instance of `class` (an internal name) and runs its
    /// constructor with descriptor `desc` (such as `(I)V`).
    pub fn construct(&mut self, class: &str, desc: &str, args: &[Value]) -> JResult<ObjRef> {
        let cid = self.find_class(class)?;
        self.construct_in(cid, desc, args)
    }

    /// Creates an instance of `class` and runs its constructor `desc`.
    pub fn construct_in(&mut self, class: ClassId, desc: &str, args: &[Value]) -> JResult<ObjRef> {
        let obj = self.new_object(class)?;
        let ctor = self
            .state
            .classes
            .get(class)
            .declared_method("<init>", desc)
            .cloned();
        let Some(ctor) = ctor else {
            let name = self.class_name(class).to_owned();
            return Err(self.missing_method(&name, "<init>", desc));
        };
        self.invoke(&ctor, &Self::with_this(obj, args))?;
        Ok(obj)
    }
}
