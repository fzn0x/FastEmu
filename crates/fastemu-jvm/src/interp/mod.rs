//! The bytecode interpreter.
//!
//! Java calls Java without recursing in Rust: frames live on the thread's
//! own stack ([`crate::thread::ThreadStack`]) and [`run`] loops over them.
//! Rust recursion happens only when native code calls back into Java.

mod decode;
mod exec;
mod flow;
mod insn;
mod resolve;

use std::sync::Arc;

pub(crate) use decode::decode;
pub(crate) use exec::run;
pub(crate) use insn::Code;
pub(crate) use resolve::Resolved;

use crate::class::{ClassId, Method};
use crate::ctx::{Ctx, Throw};
use crate::thread::Frame;
use crate::value::Value;

/// An exception the interpreter raises, made into a Java object only when
/// it is thrown, after the frame state is written back.
pub(crate) enum Fault {
    Throw(Throw),
    NullPointer,
    ArrayIndex {
        index: i32,
        len: usize,
    },
    DivideByZero,
    NegativeSize(i32),
    ClassCast {
        from: ClassId,
        to: ClassId,
    },
    ArrayStore(ClassId),
    StackOverflow,
    /// Another VM error: the exception class and its message.
    Error(&'static str, String),
}

impl From<Throw> for Fault {
    fn from(t: Throw) -> Self {
        Fault::Throw(t)
    }
}

impl Fault {
    /// The Java exception for this fault, with messages as HotSpot words
    /// them.
    pub(crate) fn into_throw(self, ctx: &mut Ctx<'_>) -> Throw {
        let dotted = |ctx: &Ctx<'_>, c: ClassId| ctx.class_name(c).replace('/', ".");
        match self {
            Fault::Throw(t) => t,
            Fault::NullPointer => ctx.new_exception("java/lang/NullPointerException", None),
            Fault::ArrayIndex { index, len } => ctx.new_exception(
                "java/lang/ArrayIndexOutOfBoundsException",
                Some(&format!("Index {index} out of bounds for length {len}")),
            ),
            Fault::DivideByZero => {
                ctx.new_exception("java/lang/ArithmeticException", Some("/ by zero"))
            }
            Fault::NegativeSize(n) => {
                ctx.new_exception("java/lang/NegativeArraySizeException", Some(&n.to_string()))
            }
            Fault::ClassCast { from, to } => {
                let message = format!(
                    "class {} cannot be cast to class {}",
                    dotted(ctx, from),
                    dotted(ctx, to)
                );
                ctx.new_exception("java/lang/ClassCastException", Some(&message))
            }
            Fault::ArrayStore(class) => {
                let message = dotted(ctx, class);
                ctx.new_exception("java/lang/ArrayStoreException", Some(&message))
            }
            Fault::StackOverflow => ctx.new_exception("java/lang/StackOverflowError", None),
            Fault::Error(class, message) => ctx.new_exception(class, Some(&message)),
        }
    }
}

/// Pushes a frame for bytecode method `method` whose arguments are in the
/// slots from `bp`. Decodes the method on its first call.
pub(crate) fn push_frame(ctx: &mut Ctx<'_>, method: &Arc<Method>, bp: usize) -> Result<(), Fault> {
    let code = match method.code() {
        Some(Ok(code)) => code,
        Some(Err(e)) => {
            return Err(Fault::Error(
                "java/lang/VerifyError",
                format!("{}: {e}", method.describe()),
            ));
        }
        None => {
            return Err(Fault::Error(
                "java/lang/InternalError",
                format!("{} has no bytecode", method.describe()),
            ));
        }
    };
    let t = &mut *ctx.thread;
    if t.frames.len() >= ctx.shared.config.max_frames {
        return Err(Fault::StackOverflow);
    }
    let locals_end = bp + code.max_locals as usize;
    if t.reserve(locals_end + code.max_stack as usize + 2).is_err() {
        return Err(Fault::StackOverflow);
    }
    t.slots[bp + method.arg_slots as usize..locals_end].fill(Value::Void);
    t.frames.push(Frame {
        method: Arc::clone(method),
        pc: 0,
        bp: bp as u32,
        sp: locals_end as u32,
        sync: None,
        native: false,
    });
    Ok(())
}
