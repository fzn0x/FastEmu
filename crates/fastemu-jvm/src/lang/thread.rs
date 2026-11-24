//! `java.lang.Thread`.

use std::time::{Duration, Instant};

use super::{string_value, this, throw};
use crate::ctx::{Ctx, JResult};
use crate::heap::{Payload, Tracer};
use crate::native::NativeClass;
use crate::thread::{self, ThreadId};
use crate::value::{ObjRef, Value};
use crate::vm::VmState;

/// The state of a `Thread` object.
pub(crate) struct ThreadData {
    /// The running thread, while it runs.
    pub(crate) tid: Option<ThreadId>,
    pub(crate) name: String,
    pub(crate) priority: i32,
    /// The `Runnable` given to the constructor.
    pub(crate) target: Option<ObjRef>,
    pub(crate) started: bool,
    pub(crate) alive: bool,
}

impl Default for ThreadData {
    fn default() -> Self {
        Self {
            tid: None,
            name: String::new(),
            priority: 5,
            target: None,
            started: false,
            alive: false,
        }
    }
}

impl Payload for ThreadData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.target);
    }
}

/// A new `Thread` object named `name`, for threads the host starts.
pub(crate) fn new_thread_object(state: &mut VmState, name: &str) -> ObjRef {
    let obj = state.alloc_instance(state.wk.thread);
    state.heap.get_mut(obj).payload = Some(Box::new(ThreadData {
        name: name.to_owned(),
        ..ThreadData::default()
    }));
    obj
}

fn data<'a>(ctx: &'a mut Ctx<'_>, a: &[Value]) -> JResult<&'a mut ThreadData> {
    ctx.payload_mut::<ThreadData>(this(a))
}

fn init(
    ctx: &mut Ctx<'_>,
    a: &[Value],
    target: Option<ObjRef>,
    name: Option<String>,
) -> JResult<Value> {
    let name = match name {
        Some(n) => n,
        None => {
            let n = ctx.state.thread_names;
            ctx.state.thread_names += 1;
            format!("Thread-{n}")
        }
    };
    let d = data(ctx, a)?;
    d.name = name;
    d.target = target;
    Ok(Value::Void)
}

fn name_arg(ctx: &mut Ctx<'_>, v: Value) -> JResult<String> {
    let s = ctx.non_null(v)?;
    ctx.string(s)
}

fn start(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let obj = this(a);
    let d = data(ctx, a)?;
    if d.started {
        return throw(
            ctx,
            "java/lang/IllegalThreadStateException",
            "already started",
        );
    }
    d.started = true;
    let name = d.name.clone();
    let result = thread::spawn(
        ctx.shared,
        &mut ctx.state,
        obj,
        name,
        Box::new(|ctx: &mut Ctx<'_>| {
            let me = ctx.current_thread();
            ctx.call_virtual(me, "run", "()V", &[])?;
            Ok(())
        }),
    );
    match result {
        Ok(_) => Ok(Value::Void),
        Err(message) => throw(ctx, "java/lang/OutOfMemoryError", &message),
    }
}

fn join(ctx: &mut Ctx<'_>, a: &[Value], millis: i64) -> JResult<Value> {
    if millis < 0 {
        return throw(
            ctx,
            "java/lang/IllegalArgumentException",
            "timeout value is negative",
        );
    }
    let deadline = (millis > 0).then(|| Instant::now() + Duration::from_millis(millis as u64));
    loop {
        if !ctx.payload::<ThreadData>(this(a))?.alive {
            return Ok(Value::Void);
        }
        if ctx.take_interrupt() {
            return throw(ctx, "java/lang/InterruptedException", "join interrupted");
        }
        if deadline.is_some_and(|d| Instant::now() >= d) {
            return Ok(Value::Void);
        }
        ctx.wait_raw(deadline)?;
    }
}

fn sleep(ctx: &mut Ctx<'_>, millis: i64) -> JResult<Value> {
    if millis < 0 {
        return throw(
            ctx,
            "java/lang/IllegalArgumentException",
            "timeout value is negative",
        );
    }
    ctx.sleep(Duration::from_millis(millis as u64))?;
    Ok(Value::Void)
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new("java/lang/Thread")
            .implements("java/lang/Runnable")
            .payload::<ThreadData>()
            .constant("MIN_PRIORITY", "I", Value::Int(1))
            .constant("NORM_PRIORITY", "I", Value::Int(5))
            .constant("MAX_PRIORITY", "I", Value::Int(10))
            .method("<init>", "()V", |ctx, a| init(ctx, a, None, None))
            .method("<init>", "(Ljava/lang/Runnable;)V", |ctx, a| {
                init(ctx, a, a[1].obj(), None)
            })
            .method(
                "<init>",
                "(Ljava/lang/Runnable;Ljava/lang/String;)V",
                |ctx, a| {
                    let name = name_arg(ctx, a[2])?;
                    init(ctx, a, a[1].obj(), Some(name))
                },
            )
            .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
                let name = name_arg(ctx, a[1])?;
                init(ctx, a, None, Some(name))
            })
            .method("run", "()V", |ctx, a| {
                if let Some(target) = data(ctx, a)?.target {
                    ctx.call_virtual(target, "run", "()V", &[])?;
                }
                Ok(Value::Void)
            })
            .method("start", "()V", start)
            .method("join", "()V", |ctx, a| join(ctx, a, 0))
            .method("join", "(J)V", |ctx, a| join(ctx, a, a[1].long()))
            .method("isAlive", "()Z", |ctx, a| {
                Ok(Value::from(data(ctx, a)?.alive))
            })
            .method("interrupt", "()V", |ctx, a| {
                if let Some(tid) = data(ctx, a)?.tid
                    && let Some(entry) = ctx.state.threads.get_mut(&tid)
                {
                    entry.interrupted = true;
                    ctx.notify_waiters();
                }
                Ok(Value::Void)
            })
            .method("isInterrupted", "()Z", |ctx, a| {
                let interrupted = data(ctx, a)?
                    .tid
                    .and_then(|tid| ctx.state.threads.get(&tid))
                    .is_some_and(|e| e.interrupted);
                Ok(Value::from(interrupted))
            })
            .method("getName", "()Ljava/lang/String;", |ctx, a| {
                let name = data(ctx, a)?.name.clone();
                string_value(ctx, &name)
            })
            .method("setName", "(Ljava/lang/String;)V", |ctx, a| {
                let name = name_arg(ctx, a[1])?;
                data(ctx, a)?.name = name;
                Ok(Value::Void)
            })
            .method("getPriority", "()I", |ctx, a| {
                Ok(Value::Int(data(ctx, a)?.priority))
            })
            .method("setPriority", "(I)V", |ctx, a| {
                let p = a[1].int();
                if !(1..=10).contains(&p) {
                    return throw(ctx, "java/lang/IllegalArgumentException", &p.to_string());
                }
                data(ctx, a)?.priority = p;
                Ok(Value::Void)
            })
            .method("setDaemon", "(Z)V", |_, _| Ok(Value::Void))
            .method("isDaemon", "()Z", |_, _| Ok(Value::from(false)))
            .method("toString", "()Ljava/lang/String;", |ctx, a| {
                let d = data(ctx, a)?;
                let text = format!("Thread[{},{}]", d.name, d.priority);
                string_value(ctx, &text)
            })
            .static_method("currentThread", "()Ljava/lang/Thread;", |ctx, _| {
                Ok(ctx.current_thread().into())
            })
            .static_method("sleep", "(J)V", |ctx, a| sleep(ctx, a[0].long()))
            .static_method("sleep", "(JI)V", |ctx, a| {
                let nanos = a[1].int();
                if !(0..=999_999).contains(&nanos) {
                    return throw(
                        ctx,
                        "java/lang/IllegalArgumentException",
                        "nanosecond timeout value out of range",
                    );
                }
                sleep(ctx, a[0].long() + i64::from(nanos >= 500_000))
            })
            .static_method("yield", "()V", |ctx, _| {
                ctx.yield_now()?;
                Ok(Value::Void)
            })
            .static_method("interrupted", "()Z", |ctx, _| {
                Ok(Value::from(ctx.take_interrupt()))
            })
            .static_method("activeCount", "()I", |ctx, _| {
                Ok(Value::Int(ctx.state.live_threads as i32))
            }),
    ]
}
