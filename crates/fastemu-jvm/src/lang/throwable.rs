//! `java.lang.Throwable`, the exception and error classes of CLDC 1.1 and
//! the errors the virtual machine itself throws, and stack traces.

use std::sync::Arc;

use fastemu_core::LogLevel;

use super::{string_value, this};
use crate::class::Method;
use crate::ctx::{Ctx, JResult, Throw};
use crate::heap::{Body, Payload};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

/// One line of a stack trace.
pub(crate) struct TraceElement {
    method: Arc<Method>,
    /// The running instruction, `None` in a native method.
    insn: Option<u32>,
}

impl TraceElement {
    /// The text after `at `, as HotSpot prints it.
    pub(crate) fn describe(&self) -> String {
        let m = &self.method;
        let class = m.class_name.replace('/', ".");
        let place = match (self.insn, &m.source_file) {
            (None, _) => "Native Method".to_owned(),
            (Some(insn), Some(file)) => match m.line_of(insn as usize) {
                Some(line) => format!("{file}:{line}"),
                None => file.to_string(),
            },
            (Some(_), None) => "Unknown Source".to_owned(),
        };
        format!("{class}.{}({place})", m.name)
    }
}

/// The stack trace a throwable captured when it was made.
#[derive(Default)]
pub(crate) struct ThrowableData {
    pub(crate) trace: Vec<TraceElement>,
}

impl Payload for ThrowableData {
    fn heap_bytes(&self) -> usize {
        self.trace.len() * 16
    }
}

/// Traces longer than this are cut. CLDC has no stack traces and HotSpot
/// keeps 1024 frames, but a game that prints one never needs more than a
/// screenful, and each frame counts 16 bytes of the heap for as long as
/// the exception lives: 256 frames are 4 KiB.
const MAX_TRACE: usize = 256;

/// Records the current thread's frames in `exc`, leaving out the frames
/// that are building the throwable itself.
pub(crate) fn fill_in_stack_trace(ctx: &mut Ctx<'_>, exc: ObjRef) {
    let throwable = ctx.state.wk.throwable;
    let mut trace = Vec::new();
    let mut skipping = true;
    let methods = &*ctx.shared.methods;
    for frame in ctx.thread.frames.iter().rev() {
        let m = methods.get(frame.method);
        if skipping {
            let building = (&*m.name == "<init>"
                && ctx.state.classes.is_subclass(m.class, throwable))
                || &*m.name == "fillInStackTrace";
            if building {
                continue;
            }
            skipping = false;
        }
        trace.push(TraceElement {
            method: Arc::clone(m),
            insn: (!frame.native).then(|| frame.pc.saturating_sub(1)),
        });
        if trace.len() >= MAX_TRACE {
            break;
        }
    }
    let stored = match ctx.state.heap.get_mut(exc).payload_mut::<ThrowableData>() {
        Some(data) => {
            data.trace = trace;
            true
        }
        None => false,
    };
    if stored {
        ctx.recount(exc);
    }
}

fn set_slot(ctx: &mut Ctx<'_>, obj: ObjRef, slot: usize, v: Value) {
    if let Body::Fields(f) = &mut ctx.state.heap.get_mut(obj).body {
        f[slot] = v;
    }
}

fn get_slot(ctx: &Ctx<'_>, obj: ObjRef, slot: usize) -> Value {
    match &ctx.state.heap.get(obj).body {
        Body::Fields(f) => f[slot],
        _ => Value::NULL,
    }
}

/// `Throwable.cause` sits right after `detailMessage`.
fn cause_slot(ctx: &Ctx<'_>) -> usize {
    ctx.state.wk.message_slot + 1
}

/// Sets the cause of throwable `exc`.
pub(crate) fn set_cause(ctx: &mut Ctx<'_>, exc: ObjRef, cause: ObjRef) {
    let slot = cause_slot(ctx);
    set_slot(ctx, exc, slot, cause.into());
}

impl Ctx<'_> {
    /// A new exception of class `class` (an internal name of a class
    /// defined in Rust, such as `java/lang/IllegalArgumentException`) with
    /// `message`, its stack trace taken from this thread. Return it as
    /// `Err(...)` from a native to throw it.
    pub fn new_exception(&mut self, class: &str, message: Option<&str>) -> Throw {
        let cid = match self.state.classes.load(class) {
            Ok(c) => c,
            Err(_) => self
                .state
                .classes
                .load("java/lang/Error")
                .expect("java/lang/Error is part of the library"),
        };
        let _ = self.initialize(cid);
        let obj = self.state.alloc_instance(cid);
        self.root(obj);
        if let Some(text) = message {
            let s = super::string::new_string(&mut self.state, text.encode_utf16().collect());
            let slot = self.state.wk.message_slot;
            set_slot(self, obj, slot, s.into());
        }
        fill_in_stack_trace(self, obj);
        Throw::Exception(obj)
    }

    /// Returns `Err` with a new exception of `class`, for natives.
    pub fn throw<T>(&mut self, class: &str, message: &str) -> JResult<T> {
        Err(self.new_exception(class, Some(message)))
    }

    /// An `OutOfMemoryError`.
    pub(crate) fn out_of_memory(&mut self) -> Throw {
        self.new_exception("java/lang/OutOfMemoryError", Some("Java heap space"))
    }

    /// `toString()` of throwable `exc`, for logs. Falls back to the class
    /// name when `toString` itself throws.
    pub fn describe_exception(&mut self, exc: ObjRef) -> String {
        let text = self
            .call_virtual(exc, "toString", "()Ljava/lang/String;", &[])
            .ok()
            .and_then(Value::obj)
            .and_then(|s| self.string(s).ok());
        text.unwrap_or_else(|| self.class_name(self.class_of(exc)).replace('/', "."))
    }

    /// The stack trace lines of throwable `exc`, each as HotSpot prints it
    /// after `at `.
    pub fn stack_trace(&self, exc: ObjRef) -> Vec<String> {
        self.state
            .heap
            .get(exc)
            .payload::<ThrowableData>()
            .map(|d| d.trace.iter().map(TraceElement::describe).collect())
            .unwrap_or_default()
    }
}

/// Prints `exc` and its trace (and its causes) to `System.err`.
pub(crate) fn print_stack_trace(ctx: &mut Ctx<'_>, exc: ObjRef, level: LogLevel) {
    print_with_prefix(ctx, exc, "", level);
}

fn print_with_prefix(ctx: &mut Ctx<'_>, exc: ObjRef, prefix: &str, level: LogLevel) {
    let mut lines = vec![format!("{prefix}{}", ctx.describe_exception(exc))];
    lines.extend(
        ctx.stack_trace(exc)
            .into_iter()
            .map(|e| format!("\tat {e}")),
    );
    let mut seen = vec![exc];
    let mut cause = get_slot(ctx, exc, cause_slot(ctx)).obj();
    while let Some(c) = cause.filter(|c| !seen.contains(c)) {
        seen.push(c);
        lines.push(format!("Caused by: {}", ctx.describe_exception(c)));
        lines.extend(ctx.stack_trace(c).into_iter().map(|e| format!("\tat {e}")));
        cause = get_slot(ctx, c, cause_slot(ctx)).obj();
    }
    for line in lines {
        if level == LogLevel::Err {
            super::system::err_line(ctx, &line);
        } else {
            ctx.log(level, line);
        }
    }
}

/// What a thread prints when an exception ends it.
pub(crate) fn report_uncaught(ctx: &mut Ctx<'_>, exc: ObjRef) {
    let thread = ctx.state.threads[&ctx.tid].java_thread;
    let name = ctx
        .try_payload::<super::thread::ThreadData>(thread)
        .map_or_else(
            || ctx.state.threads[&ctx.tid].name.clone(),
            |d| d.name.clone(),
        );
    print_with_prefix(
        ctx,
        exc,
        &format!("Exception in thread \"{name}\" "),
        LogLevel::Err,
    );
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it has the shape of a native method body, so natives return it directly"
)]
fn init(ctx: &mut Ctx<'_>, a: &[Value], message: Value, cause: Option<ObjRef>) -> JResult<Value> {
    let obj = this(a);
    let slot = ctx.state.wk.message_slot;
    set_slot(ctx, obj, slot, message);
    if let Some(cause) = cause {
        set_cause(ctx, obj, cause);
    }
    fill_in_stack_trace(ctx, obj);
    Ok(Value::Void)
}

/// `new Throwable(cause)` takes the cause's `toString()` as its message.
fn init_with_cause(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let message = match a[1].obj() {
        Some(cause) => {
            let text = ctx.describe_exception(cause);
            ctx.new_string(&text)?.into()
        }
        None => Value::NULL,
    };
    init(ctx, a, message, a[1].obj())
}

fn throwable() -> NativeClass {
    NativeClass::new("java/lang/Throwable")
        .payload::<ThrowableData>()
        .field_with(
            "detailMessage",
            "Ljava/lang/String;",
            crate::classfile::access::PRIVATE,
        )
        .field_with(
            "cause",
            "Ljava/lang/Throwable;",
            crate::classfile::access::PRIVATE,
        )
        .method("<init>", "()V", |ctx, a| init(ctx, a, Value::NULL, None))
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            init(ctx, a, a[1], None)
        })
        .method(
            "<init>",
            "(Ljava/lang/String;Ljava/lang/Throwable;)V",
            |ctx, a| init(ctx, a, a[1], a[2].obj()),
        )
        .method("<init>", "(Ljava/lang/Throwable;)V", init_with_cause)
        .method("getMessage", "()Ljava/lang/String;", |ctx, a| {
            let slot = ctx.state.wk.message_slot;
            Ok(get_slot(ctx, this(a), slot))
        })
        .method("getLocalizedMessage", "()Ljava/lang/String;", |ctx, a| {
            ctx.call_virtual(this(a), "getMessage", "()Ljava/lang/String;", &[])
        })
        .method("getCause", "()Ljava/lang/Throwable;", |ctx, a| {
            let slot = cause_slot(ctx);
            Ok(get_slot(ctx, this(a), slot))
        })
        .method(
            "initCause",
            "(Ljava/lang/Throwable;)Ljava/lang/Throwable;",
            |ctx, a| {
                if let Some(cause) = a[1].obj() {
                    set_cause(ctx, this(a), cause);
                }
                Ok(a[0])
            },
        )
        .method("toString", "()Ljava/lang/String;", |ctx, a| {
            let obj = this(a);
            let name = ctx.class_name(ctx.class_of(obj)).replace('/', ".");
            let message =
                ctx.call_virtual(obj, "getLocalizedMessage", "()Ljava/lang/String;", &[])?;
            match ctx.opt_string(message)? {
                Some(m) => string_value(ctx, &format!("{name}: {m}")),
                None => string_value(ctx, &name),
            }
        })
        .method("printStackTrace", "()V", |ctx, a| {
            print_stack_trace(ctx, this(a), LogLevel::Err);
            Ok(Value::Void)
        })
        .method("printStackTrace", "(Ljava/io/PrintStream;)V", |ctx, a| {
            let stream = ctx.non_null(a[1])?;
            let obj = this(a);
            let mut lines = vec![ctx.describe_exception(obj)];
            lines.extend(
                ctx.stack_trace(obj)
                    .into_iter()
                    .map(|e| format!("\tat {e}")),
            );
            for line in lines {
                let s = ctx.new_string(&line)?;
                ctx.call_virtual(stream, "println", "(Ljava/lang/String;)V", &[s.into()])?;
            }
            Ok(Value::Void)
        })
        .method("fillInStackTrace", "()Ljava/lang/Throwable;", |ctx, a| {
            fill_in_stack_trace(ctx, this(a));
            Ok(a[0])
        })
}

/// A throwable subclass with the usual constructors.
fn subclass(name: &str, super_name: &str) -> NativeClass {
    let sup = super_name.to_owned();
    let sup2 = super_name.to_owned();
    let sup3 = super_name.to_owned();
    let sup4 = super_name.to_owned();
    NativeClass::new(name)
        .extends(super_name)
        .method("<init>", "()V", move |ctx, a| {
            ctx.call_special(&sup, this(a), "<init>", "()V", &[])?;
            Ok(Value::Void)
        })
        .method("<init>", "(Ljava/lang/String;)V", move |ctx, a| {
            ctx.call_special(&sup2, this(a), "<init>", "(Ljava/lang/String;)V", &[a[1]])?;
            Ok(Value::Void)
        })
        .method(
            "<init>",
            "(Ljava/lang/String;Ljava/lang/Throwable;)V",
            move |ctx, a| {
                ctx.call_special(
                    &sup3,
                    this(a),
                    "<init>",
                    "(Ljava/lang/String;Ljava/lang/Throwable;)V",
                    &[a[1], a[2]],
                )?;
                Ok(Value::Void)
            },
        )
        .method("<init>", "(Ljava/lang/Throwable;)V", move |ctx, a| {
            ctx.call_special(
                &sup4,
                this(a),
                "<init>",
                "(Ljava/lang/Throwable;)V",
                &[a[1]],
            )?;
            Ok(Value::Void)
        })
}

/// Every throwable class but `Throwable`, each after its superclass.
const HIERARCHY: &[(&str, &str)] = &[
    ("java/lang/Exception", "java/lang/Throwable"),
    ("java/lang/Error", "java/lang/Throwable"),
    ("java/lang/RuntimeException", "java/lang/Exception"),
    (
        "java/lang/ArithmeticException",
        "java/lang/RuntimeException",
    ),
    (
        "java/lang/ArrayStoreException",
        "java/lang/RuntimeException",
    ),
    ("java/lang/ClassCastException", "java/lang/RuntimeException"),
    (
        "java/lang/IllegalArgumentException",
        "java/lang/RuntimeException",
    ),
    (
        "java/lang/IllegalMonitorStateException",
        "java/lang/RuntimeException",
    ),
    (
        "java/lang/IllegalStateException",
        "java/lang/RuntimeException",
    ),
    (
        "java/lang/IndexOutOfBoundsException",
        "java/lang/RuntimeException",
    ),
    (
        "java/lang/NegativeArraySizeException",
        "java/lang/RuntimeException",
    ),
    (
        "java/lang/NullPointerException",
        "java/lang/RuntimeException",
    ),
    ("java/lang/SecurityException", "java/lang/RuntimeException"),
    (
        "java/lang/UnsupportedOperationException",
        "java/lang/RuntimeException",
    ),
    (
        "java/lang/ArrayIndexOutOfBoundsException",
        "java/lang/IndexOutOfBoundsException",
    ),
    (
        "java/lang/StringIndexOutOfBoundsException",
        "java/lang/IndexOutOfBoundsException",
    ),
    (
        "java/lang/IllegalThreadStateException",
        "java/lang/IllegalArgumentException",
    ),
    (
        "java/lang/NumberFormatException",
        "java/lang/IllegalArgumentException",
    ),
    ("java/lang/ClassNotFoundException", "java/lang/Exception"),
    (
        "java/lang/CloneNotSupportedException",
        "java/lang/Exception",
    ),
    ("java/lang/IllegalAccessException", "java/lang/Exception"),
    ("java/lang/InstantiationException", "java/lang/Exception"),
    ("java/lang/InterruptedException", "java/lang/Exception"),
    ("java/lang/VirtualMachineError", "java/lang/Error"),
    (
        "java/lang/OutOfMemoryError",
        "java/lang/VirtualMachineError",
    ),
    (
        "java/lang/StackOverflowError",
        "java/lang/VirtualMachineError",
    ),
    ("java/lang/InternalError", "java/lang/VirtualMachineError"),
    ("java/lang/LinkageError", "java/lang/Error"),
    ("java/lang/NoClassDefFoundError", "java/lang/LinkageError"),
    ("java/lang/ClassFormatError", "java/lang/LinkageError"),
    ("java/lang/ClassCircularityError", "java/lang/LinkageError"),
    (
        "java/lang/ExceptionInInitializerError",
        "java/lang/LinkageError",
    ),
    ("java/lang/UnsatisfiedLinkError", "java/lang/LinkageError"),
    ("java/lang/VerifyError", "java/lang/LinkageError"),
    (
        "java/lang/IncompatibleClassChangeError",
        "java/lang/LinkageError",
    ),
    (
        "java/lang/NoSuchFieldError",
        "java/lang/IncompatibleClassChangeError",
    ),
    (
        "java/lang/NoSuchMethodError",
        "java/lang/IncompatibleClassChangeError",
    ),
    (
        "java/lang/AbstractMethodError",
        "java/lang/IncompatibleClassChangeError",
    ),
    (
        "java/lang/InstantiationError",
        "java/lang/IncompatibleClassChangeError",
    ),
    (
        "java/lang/IllegalAccessError",
        "java/lang/IncompatibleClassChangeError",
    ),
    ("java/io/IOException", "java/lang/Exception"),
    ("java/io/EOFException", "java/io/IOException"),
    ("java/io/InterruptedIOException", "java/io/IOException"),
    (
        "java/io/UnsupportedEncodingException",
        "java/io/IOException",
    ),
    ("java/io/UTFDataFormatException", "java/io/IOException"),
    (
        "java/util/NoSuchElementException",
        "java/lang/RuntimeException",
    ),
    (
        "java/util/EmptyStackException",
        "java/lang/RuntimeException",
    ),
];

/// A constructor taking an index, which says it in the message as Java's
/// `ArrayIndexOutOfBoundsException(int)` does: `"<what> out of range: 5"`.
fn index_constructor(class: NativeClass, sup: &str, what: &'static str) -> NativeClass {
    let sup = sup.to_owned();
    class.method("<init>", "(I)V", move |ctx, a| {
        let message = ctx.new_string(&format!("{what} out of range: {}", a[1].int()))?;
        let args = [message.into()];
        ctx.call_special(&sup, this(a), "<init>", "(Ljava/lang/String;)V", &args)?;
        Ok(Value::Void)
    })
}

/// The members a few throwables have beyond the usual constructors.
fn refine(name: &str, sup: &str, class: NativeClass) -> NativeClass {
    match name {
        "java/lang/ArrayIndexOutOfBoundsException" => index_constructor(class, sup, "Array index"),
        "java/lang/StringIndexOutOfBoundsException" => {
            index_constructor(class, sup, "String index")
        }
        "java/lang/VirtualMachineError" => class.abstract_class(),
        "java/io/InterruptedIOException" => class.field("bytesTransferred", "I"),
        _ => class,
    }
}

pub(crate) fn classes() -> Vec<NativeClass> {
    let mut all = vec![throwable()];
    all.extend(
        HIERARCHY
            .iter()
            .map(|&(name, sup)| refine(name, sup, subclass(name, sup))),
    );
    all
}
