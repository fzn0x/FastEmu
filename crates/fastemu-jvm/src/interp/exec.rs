//! The interpreter loop.
//!
//! [`run`] keeps the current frame's program counter and operand stack top
//! in locals and writes them back to the frame ("sync") before anything
//! that can look at the frame from outside: a call, a collection, a wait,
//! creating an exception (its stack trace reads the frame) or running a
//! static initializer (which pushes frames above this one).

use std::sync::atomic::Ordering;

use smallvec::SmallVec;

use super::insn::Insn;
use super::resolve::{FieldRes, MethodRes, Resolved};
use super::{Fault, push_frame};
use crate::class::{ArrayType, ClassId, ClassKind, InitState, Method, MethodBody, RtPool};
use crate::ctx::{Ctx, JResult, Throw};
use crate::descriptor::Kind;
use crate::heap::{Body, Heap};
use crate::lang;
use crate::value::{ObjRef, Value};
use crate::vm::{VmShared, VmState};

/// Backward branches and calls between safepoints.
const YIELD_INTERVAL: u32 = 4096;

/// What one exception transfer to a handler costs of that budget, so that
/// a safepoint comes every 256 of them. A loop of nothing but throws and
/// handlers has no backward branch and no call to count, and each pass
/// may make an exception with a stack trace of up to 4 KiB.
const HANDLER_COST: u32 = 16;

enum Flow {
    /// A callee frame was pushed.
    Call,
    /// The current frame returned this value.
    Return(Value),
}

// The run loop keeps its rare and long paths out of line, in the
// functions below, each `#[inline(never)]`. Its code is sensitive to its
// size: one more path inlined into it can make the compiler keep less in
// registers, and every instruction slower.

#[cold]
#[inline(never)]
fn wrong_array() -> Fault {
    Fault::Error(
        "java/lang/VerifyError",
        "an array instruction on the wrong array type".to_owned(),
    )
}

#[cold]
#[inline(never)]
fn wrong_field() -> Fault {
    Fault::Error(
        "java/lang/VerifyError",
        "a field access on an object without that field".to_owned(),
    )
}

#[cold]
#[inline(never)]
fn incompatible(message: String) -> Fault {
    Fault::Error("java/lang/IncompatibleClassChangeError", message)
}

#[cold]
#[inline(never)]
fn verify_error(message: &str) -> Fault {
    Fault::Error("java/lang/VerifyError", message.to_owned())
}

#[cold]
#[inline(never)]
fn unsupported(op: u8) -> Fault {
    incompatible(format!(
        "bytecode {op:#x} (invokedynamic or a method handle) is not supported"
    ))
}

#[cold]
#[inline(never)]
fn safepoint(ctx: &mut Ctx<'_>) -> JResult<()> {
    ctx.safepoint()
}

#[cold]
#[inline(never)]
fn safepoint_after_native(ctx: &mut Ctx<'_>) -> JResult<()> {
    ctx.safepoint_after_native()
}

#[cold]
#[inline(never)]
fn initialize(ctx: &mut Ctx<'_>, class: ClassId) -> JResult<()> {
    ctx.initialize(class)
}

#[cold]
#[inline(never)]
fn resolve_class(ctx: &mut Ctx<'_>, pool: &RtPool, index: u16) -> JResult<ClassId> {
    ctx.resolve_class(pool, index)
}

#[cold]
#[inline(never)]
fn ldc_string(ctx: &mut Ctx<'_>, pool: &RtPool, index: u16) -> JResult<ObjRef> {
    ctx.resolve_string(pool, index)
}

#[cold]
#[inline(never)]
fn ldc_class(ctx: &mut Ctx<'_>, pool: &RtPool, index: u16) -> Result<ObjRef, Fault> {
    let c = class_res(ctx, pool, index)?;
    Ok(lang::class::mirror(&mut ctx.state, c))
}

/// Resolves a `getstatic` or `putstatic`: the class, the index of its
/// static and whether it takes two slots.
#[cold]
#[inline(never)]
fn static_field(
    ctx: &mut Ctx<'_>,
    pool: &RtPool,
    index: u16,
) -> Result<(ClassId, u32, bool), Fault> {
    match ctx.resolve_field_ref(pool, index)? {
        FieldRes::Static { class, index, wide } => Ok((class, index, wide)),
        FieldRes::Instance { .. } => Err(incompatible("Expected static field".to_owned())),
    }
}

/// Resolves a `getfield` or `putfield`: the field's slot in the object and
/// whether it takes two slots.
#[cold]
#[inline(never)]
fn instance_field(ctx: &mut Ctx<'_>, pool: &RtPool, index: u16) -> Result<(usize, bool), Fault> {
    match ctx.resolve_field_ref(pool, index)? {
        FieldRes::Instance { slot, wide } => Ok((slot as usize, wide)),
        FieldRes::Static { .. } => Err(incompatible("Expected non-static field".to_owned())),
    }
}

/// `new`: resolves and initializes the class, and allocates the object.
#[inline(never)]
fn new_object(ctx: &mut Ctx<'_>, pool: &RtPool, index: u16) -> Result<ObjRef, Fault> {
    let cid = class_res(ctx, pool, index)?;
    if ctx.state.classes.init[cid.index()] != InitState::Done {
        initialize(ctx, cid)?;
    }
    let class = ctx.state.classes.get(cid);
    if class.is_abstract() || class.is_interface() || class.is_array() {
        let name = class.name.replace('/', ".");
        return Err(Fault::Error("java/lang/InstantiationError", name));
    }
    let bytes = 16 + class.template.len() * 8;
    if ctx.state.heap.gc_requested() || ctx.state.heap.would_exceed(bytes) {
        gc_for(ctx, bytes)?;
    }
    Ok(ctx.state.alloc_instance(cid))
}

/// `newarray`: an array of `n` primitives of type `atype`.
#[inline(never)]
fn new_prim_array(ctx: &mut Ctx<'_>, atype: u8, n: i32) -> Result<ObjRef, Fault> {
    if n < 0 {
        return Err(Fault::NegativeSize(n));
    }
    let t = ArrayType::from_atype(atype).expect("checked when decoded");
    let bytes = Heap::array_bytes(n as usize, t.elem_bytes());
    if ctx.state.heap.gc_requested() || ctx.state.heap.would_exceed(bytes) {
        gc_for(ctx, bytes)?;
    }
    Ok(ctx.state.alloc_prim_array(t, n as usize))
}

/// `anewarray`: an array of `n` references to the class of entry `index`.
#[inline(never)]
fn new_ref_array(ctx: &mut Ctx<'_>, pool: &RtPool, index: u16, n: i32) -> Result<ObjRef, Fault> {
    let elem = class_res(ctx, pool, index)?;
    if n < 0 {
        return Err(Fault::NegativeSize(n));
    }
    let class = ctx.array_class_of(elem)?;
    let bytes = Heap::array_bytes(n as usize, 4);
    if ctx.state.heap.gc_requested() || ctx.state.heap.would_exceed(bytes) {
        gc_for(ctx, bytes)?;
    }
    Ok(ctx.state.alloc_ref_array(class, n as usize))
}

/// `multianewarray`: the array class of entry `index`, with the sizes
/// `counts`.
#[cold]
#[inline(never)]
fn new_multi_array(
    ctx: &mut Ctx<'_>,
    pool: &RtPool,
    index: u16,
    counts: &[i32],
) -> Result<ObjRef, Fault> {
    let class = class_res(ctx, pool, index)?;
    if let Some(&n) = counts.iter().find(|&&n| n < 0) {
        return Err(Fault::NegativeSize(n));
    }
    if !ctx.state.classes.get(class).is_array() {
        return Err(verify_error(
            "multianewarray of a class that is not an array",
        ));
    }
    let bytes = multi_bytes(&ctx.state, class, counts);
    if ctx.state.heap.gc_requested() || ctx.state.heap.would_exceed(bytes) {
        gc_for(ctx, bytes)?;
    }
    Ok(alloc_multi(&mut ctx.state, class, counts))
}

/// `athrow` of `e`.
#[cold]
#[inline(never)]
fn athrow(ctx: &Ctx<'_>, e: ObjRef) -> Fault {
    // Handlers and the report of an uncaught exception read the object as
    // a Throwable, as a verifier would ensure.
    let class = ctx.state.heap.class_of(e);
    if !ctx.state.classes.is_subclass(class, ctx.state.wk.throwable) {
        return verify_error("athrow of an object that is not a Throwable");
    }
    Fault::Throw(Throw::Exception(e))
}

/// `monitorenter` or `monitorexit` of `obj`.
#[inline(never)]
fn monitor(ctx: &mut Ctx<'_>, obj: ObjRef, enter: bool) -> JResult<()> {
    if enter {
        ctx.monitor_enter(obj)
    } else {
        ctx.monitor_exit(obj)
    }
}

/// Collects before an allocation of `bytes`; throws `OutOfMemoryError` if
/// the heap still has no room.
#[cold]
#[inline(never)]
fn gc_for(ctx: &mut Ctx<'_>, bytes: usize) -> JResult<()> {
    ctx.collect();
    if ctx.state.heap.would_exceed(bytes) {
        return Err(ctx.out_of_memory());
    }
    Ok(())
}

/// What the heap will count for the arrays `alloc_multi` builds for these
/// sizes: the outer array, and the arrays of every level below it, each at
/// its own length and element size. Saturates, so a request that no heap
/// holds is a number that no heap holds.
fn multi_bytes(st: &VmState, class: ClassId, counts: &[i32]) -> usize {
    let mut bytes = 0usize;
    // How many arrays the level being sized has.
    let mut arrays = 1usize;
    let mut class = class;
    for (level, &len) in counts.iter().enumerate() {
        let (len, elem_bytes, inner) = match st.classes.get(class).kind {
            ClassKind::PrimArray(t) => (len as usize, t.elem_bytes(), None),
            ClassKind::RefArray(elem) => (len as usize, 4, Some(elem)),
            // Not an array: `alloc_multi` makes an empty one.
            ClassKind::Plain | ClassKind::Interface => (0, 4, None),
        };
        bytes = bytes.saturating_add(arrays.saturating_mul(Heap::array_bytes(len, elem_bytes)));
        match inner {
            // The next sizes are for the arrays this level holds.
            Some(elem) if level + 1 < counts.len() && len > 0 => {
                arrays = arrays.saturating_mul(len);
                class = elem;
            }
            _ => break,
        }
    }
    bytes
}

/// Allocates a multi-dimensional array of class `class`.
fn alloc_multi(st: &mut VmState, class: ClassId, counts: &[i32]) -> ObjRef {
    let len = counts[0] as usize;
    match st.classes.get(class).kind {
        ClassKind::PrimArray(t) => st.alloc_prim_array(t, len),
        ClassKind::RefArray(elem) => {
            let arr = st.alloc_ref_array(class, len);
            if counts.len() > 1 {
                for i in 0..len {
                    let sub = alloc_multi(st, elem, &counts[1..]);
                    if let Body::Refs(a) = &mut st.heap.get_mut(arr).body {
                        a[i] = Some(sub);
                    }
                }
            }
            arr
        }
        // multianewarray names an array class; the decoder's caller checks.
        ClassKind::Plain | ClassKind::Interface => st.alloc_ref_array(class, 0),
    }
}

#[inline]
fn method_res<'p>(ctx: &mut Ctx<'_>, pool: &'p RtPool, index: u16) -> Result<&'p MethodRes, Fault> {
    match pool.cache[index as usize].get() {
        Some(Resolved::Method(m)) => Ok(m),
        _ => Ok(ctx.resolve_method_ref(pool, index)?),
    }
}

#[inline]
fn class_res(ctx: &mut Ctx<'_>, pool: &RtPool, index: u16) -> Result<ClassId, Fault> {
    match pool.cache[index as usize].get() {
        Some(Resolved::Class(c)) => Ok(*c),
        _ => Ok(ctx.resolve_class(pool, index)?),
    }
}

#[inline]
fn receiver(ctx: &Ctx<'_>, sp: usize, res: &MethodRes) -> Result<ObjRef, Fault> {
    ctx.thread.slots[sp - res.arg_slots as usize]
        .obj()
        .ok_or(Fault::NullPointer)
}

#[cold]
#[inline(never)]
fn abstract_error(m: &Method) -> Fault {
    Fault::Error(
        "java/lang/AbstractMethodError",
        m.describe().replace('/', "."),
    )
}

/// Picks the method an invoke instruction runs. The method is the
/// machine's, borrowed from its arena, so the call counts no reference.
#[inline(never)]
fn select<'vm>(
    ctx: &mut Ctx<'vm>,
    pool: &RtPool,
    insn: Insn,
    sp: usize,
) -> Result<&'vm Method, Fault> {
    let shared: &'vm VmShared = ctx.shared;
    let methods = &*shared.methods;
    match insn {
        Insn::InvokeStatic(i) => {
            let res = method_res(ctx, pool, i)?;
            let m = res.target(methods);
            if !res.is_static {
                return Err(incompatible(format!(
                    "Expected static method '{}'",
                    m.describe()
                )));
            }
            ctx.initialize(m.class)?;
            Ok(m)
        }
        Insn::InvokeVirtual(i) => {
            let res = method_res(ctx, pool, i)?;
            if res.is_static {
                let m = res.target(methods);
                return Err(incompatible(format!(
                    "Expecting non-static method '{}'",
                    m.describe()
                )));
            }
            let recv = receiver(ctx, sp, res)?;
            if res.direct {
                return Ok(res.target(methods));
            }
            let rc = ctx.state.heap.class_of(recv);
            let class = ctx.state.classes.get(rc);
            // The vtable index is the one of the referenced class. An object
            // of an unrelated class holds another method at that index, or
            // none, and its arguments would not be the ones the caller
            // pushed. A selector stands for a name and a descriptor.
            let found = match res.vindex {
                Some(v) => class
                    .vtable
                    .get(v as usize)
                    .filter(|m| m.selector == res.selector),
                None => class.virtual_method(res.selector),
            };
            match found {
                Some(m) => Ok(methods.method(m.id)),
                None => Err(incompatible(format!(
                    "Class {} does not have the method '{}'",
                    ctx.class_name(rc).replace('/', "."),
                    res.target(methods).describe().replace('/', ".")
                ))),
            }
        }
        Insn::InvokeInterface(i) => {
            let res = method_res(ctx, pool, i)?;
            let recv = receiver(ctx, sp, res)?;
            let rc = ctx.state.heap.class_of(recv);
            let class = ctx.state.classes.get(rc);
            let packed = res.icache.load(Ordering::Relaxed);
            if (packed >> 32) as u32 == rc.0 + 1 {
                return Ok(methods.method(class.vtable[packed as u32 as usize].id));
            }
            match class.vmap.get(&res.selector) {
                Some(&v) => {
                    res.icache.store(
                        (u64::from(rc.0) + 1) << 32 | u64::from(v),
                        Ordering::Relaxed,
                    );
                    Ok(methods.method(class.vtable[v as usize].id))
                }
                None => {
                    if ctx.state.classes.is_assignable(rc, res.class) {
                        Err(abstract_error(res.target(methods)))
                    } else {
                        Err(incompatible(format!(
                            "Class {} does not implement the requested interface {}",
                            ctx.class_name(rc).replace('/', "."),
                            ctx.class_name(res.class).replace('/', ".")
                        )))
                    }
                }
            }
        }
        Insn::InvokeSpecial(i) => {
            let res = method_res(ctx, pool, i)?;
            if res.is_static {
                let m = res.target(methods);
                return Err(incompatible(format!(
                    "Expecting non-static method '{}'",
                    m.describe()
                )));
            }
            receiver(ctx, sp, res)?;
            let chosen = *res.special.get_or_init(|| {
                ctx.state.classes.select_special(
                    pool.class,
                    pool.super_semantics,
                    res.target(methods),
                )
            });
            Ok(methods.method(chosen))
        }
        _ => unreachable!("select is only called for invoke instructions"),
    }
}

/// Calls `m` with its arguments on top of frame `fi`'s operand stack.
/// Returns whether a callee frame was pushed; for natives, runs them and
/// leaves the result on the stack.
fn call(ctx: &mut Ctx<'_>, fi: usize, m: &Method, sp: &mut usize) -> Result<bool, Fault> {
    // The stack walk counted these arguments, so this holds for code that
    // passed it. A release build would wrap on a bug and run far off the
    // frame.
    let Some(new_bp) = sp.checked_sub(m.arg_slots as usize) else {
        return Err(Fault::Error(
            "java/lang/VerifyError",
            format!("{} takes more arguments than the stack holds", m.describe()),
        ));
    };
    match &m.body {
        MethodBody::Bytecode(_) => {
            ctx.thread.frames[fi].sp = new_bp as u32;
            if let Err(f) = push_frame(ctx, m, new_bp) {
                ctx.thread.frames[fi].sp = *sp as u32;
                return Err(f);
            }
            if m.is_synchronized() {
                let obj = if m.is_static() {
                    lang::class::mirror(&mut ctx.state, m.class)
                } else {
                    ctx.thread.slots[new_bp]
                        .obj()
                        .expect("the receiver was null-checked")
                };
                if let Err(e) = ctx.monitor_enter(obj) {
                    ctx.thread.frames.pop();
                    ctx.thread.frames[fi].sp = *sp as u32;
                    return Err(e.into());
                }
                if let Some(frame) = ctx.thread.frames.last_mut() {
                    frame.sync = Some(obj);
                }
            }
            Ok(true)
        }
        MethodBody::Native(f) => {
            let args: SmallVec<[Value; 8]> = ctx.thread.slots[new_bp..*sp]
                .iter()
                .copied()
                .filter(|v| !matches!(v, Value::Void))
                .collect();
            ctx.baseline = ctx.state.heap.used();
            let v = ctx.call_native(m, f, new_bp, &args)?;
            let s = &mut ctx.thread.slots;
            let mut top = new_bp;
            match m.ret {
                Kind::Void => {}
                Kind::Long | Kind::Double => {
                    s[top] = if v == Value::Void {
                        m.ret.default_value()
                    } else {
                        v
                    };
                    s[top + 1] = Value::Void;
                    top += 2;
                }
                kind => {
                    s[top] = if v == Value::Void {
                        kind.default_value()
                    } else {
                        v
                    };
                    top += 1;
                }
            }
            *sp = top;
            Ok(false)
        }
        MethodBody::Abstract => Err(abstract_error(m)),
        MethodBody::Unlinked => Err(Fault::Error(
            "java/lang/UnsatisfiedLinkError",
            m.describe().replace('/', "."),
        )),
    }
}

/// Looks for a handler of `exc` in the frames above `base`, popping the
/// frames that have none. Returns whether one was found; its frame is
/// then on top with the exception on its operand stack.
fn catch(ctx: &mut Ctx<'_>, base: usize, exc: ObjRef) -> bool {
    let shared = ctx.shared;
    let exc_class = ctx.state.heap.class_of(exc);
    while ctx.thread.frames.len() > base {
        let frame = ctx.thread.frames.last().expect("more frames than base");
        let method = shared.methods.method(frame.method);
        let pc = frame.pc.saturating_sub(1);
        if !frame.native
            && let (Some(Ok(code)), Some(pool)) = (method.code(), method.pool())
        {
            for h in &code.handlers {
                if pc < h.start || pc >= h.end {
                    continue;
                }
                let matches = h.catch_type == 0
                    || catch_class(ctx, pool, h.catch_type)
                        .is_some_and(|c| ctx.state.classes.is_assignable(exc_class, c));
                if matches {
                    let t = &mut *ctx.thread;
                    let f = t.frames.last_mut().expect("checked above");
                    let at = f.bp as usize + code.max_locals as usize;
                    t.slots[at] = Value::from(exc);
                    f.sp = at as u32 + 1;
                    f.pc = h.target;
                    return true;
                }
            }
        }
        let frame = ctx.thread.frames.pop().expect("more frames than base");
        if let Some(obj) = frame.sync {
            let _ = ctx.monitor_exit(obj);
        }
    }
    false
}

/// The class a catch clause names, or `None` when it cannot load. A class
/// that does not exist cannot be the class of a thrown object, so such a
/// clause catches nothing; the missing class is still reported.
fn catch_class(ctx: &mut Ctx<'_>, pool: &RtPool, index: u16) -> Option<ClassId> {
    if let Some(Resolved::Class(c)) = pool.cache[index as usize].get() {
        return Some(*c);
    }
    let name = pool.cp.class_name(index).ok()?;
    match ctx.state.classes.load(name) {
        Ok(c) => {
            let _ = pool.cache[index as usize].set(Resolved::Class(c));
            Some(c)
        }
        Err(e) => {
            if let Some(missing) = e.missing {
                ctx.state
                    .report(ctx.shared, crate::vm::Diagnostic::MissingClass(missing));
            }
            None
        }
    }
}

/// Runs the interpreter until the frame stack is back to `base` frames,
/// returning the value the frame at index `base` returned.
///
/// Everything the interpreter holds lives in its frames, so references
/// rooted as native locals while it runs (exceptions it creates, results
/// of static initializers) are dropped again once they are on a frame or
/// returned; the caller roots what this returns.
pub(crate) fn run(ctx: &mut Ctx<'_>, base: usize) -> JResult<Value> {
    let mut budget = YIELD_INTERVAL;
    let locals_mark = ctx.thread.locals.len();
    loop {
        let fi = ctx.thread.frames.len() - 1;
        // The frame's method is borrowed from the machine's arena, which
        // outlives this thread, so a call and a return count no reference.
        let shared: &VmShared = ctx.shared;
        let method = shared.methods.method(ctx.thread.frames[fi].method);
        let Some(Ok(code)) = method.code() else {
            unreachable!("frames are pushed only for decoded bytecode")
        };
        let pool: &RtPool = method.pool().expect("bytecode methods have a pool");
        let insns = &*code.insns;
        let bp = ctx.thread.frames[fi].bp as usize;
        let mut pc = ctx.thread.frames[fi].pc as usize;
        let mut sp = ctx.thread.frames[fi].sp as usize;

        // The thread's slots as a slice of the loop's own, so that it can
        // keep their pointer and length in registers. A call that takes
        // all of `ctx` may grow the slots, and the slice is taken again
        // after it (`again!`).
        let mut slots: &mut [Value] = &mut ctx.thread.slots;
        macro_rules! s {
            () => {
                (*slots)
            };
        }
        macro_rules! again {
            () => {
                slots = &mut ctx.thread.slots
            };
        }
        macro_rules! sync {
            () => {{
                let f = &mut ctx.thread.frames[fi];
                f.pc = pc as u32;
                f.sp = sp as u32;
            }};
        }
        macro_rules! tri {
            ($e:expr) => {
                match $e {
                    Ok(v) => v,
                    Err(e) => break Err(Fault::from(e)),
                }
            };
        }
        macro_rules! jump {
            ($t:expr) => {{
                let t = $t as usize;
                if t < pc {
                    budget -= 1;
                    if budget == 0 {
                        budget = YIELD_INTERVAL;
                        pc = t;
                        sync!();
                        tri!(safepoint(ctx));
                        again!();
                    }
                }
                pc = t;
            }};
        }
        macro_rules! push {
            ($v:expr) => {{
                let v = $v;
                s!()[sp] = v;
                sp += 1;
            }};
        }
        macro_rules! push2 {
            ($v:expr) => {{
                let v = $v;
                let s = &mut s!();
                s[sp] = v;
                s[sp + 1] = Value::Void;
                sp += 2;
            }};
        }
        macro_rules! ibin {
            ($f:expr) => {{
                let s = &mut s!();
                let b = s[sp - 1].int();
                let a = s[sp - 2].int();
                s[sp - 2] = Value::Int($f(a, b));
                sp -= 1;
            }};
        }
        macro_rules! lbin {
            ($f:expr) => {{
                let s = &mut s!();
                let b = s[sp - 2].long();
                let a = s[sp - 4].long();
                s[sp - 4] = Value::Long($f(a, b));
                sp -= 2;
            }};
        }
        macro_rules! fbin {
            ($f:expr) => {{
                let s = &mut s!();
                let b = s[sp - 1].float();
                let a = s[sp - 2].float();
                s[sp - 2] = Value::Float($f(a, b));
                sp -= 1;
            }};
        }
        macro_rules! dbin {
            ($f:expr) => {{
                let s = &mut s!();
                let b = s[sp - 2].double();
                let a = s[sp - 4].double();
                s[sp - 4] = Value::Double($f(a, b));
                sp -= 2;
            }};
        }
        macro_rules! lshift {
            ($f:expr) => {{
                let s = &mut s!();
                let n = s[sp - 1].int() as u32;
                let a = s[sp - 3].long();
                s[sp - 3] = Value::Long($f(a, n));
                sp -= 1;
            }};
        }
        macro_rules! if1 {
            ($t:expr, $cond:expr) => {{
                sp -= 1;
                let v = s!()[sp].int();
                if $cond(v) {
                    jump!($t);
                }
            }};
        }
        macro_rules! if2 {
            ($t:expr, $cond:expr) => {{
                sp -= 2;
                let a = s!()[sp].int();
                let b = s!()[sp + 1].int();
                if $cond(a, b) {
                    jump!($t);
                }
            }};
        }
        macro_rules! aload {
            ($variant:ident, $wrap:expr, $wide:expr) => {{
                let index = s!()[sp - 1].int();
                let Some(arr) = s!()[sp - 2].obj() else {
                    break Err(Fault::NullPointer);
                };
                let value = match &ctx.state.heap.get(arr).body {
                    Body::$variant(a) => match a.get(index as usize) {
                        Some(&v) => $wrap(v),
                        None => {
                            break Err(Fault::ArrayIndex {
                                index,
                                len: a.len(),
                            });
                        }
                    },
                    _ => break Err(wrong_array()),
                };
                let s = &mut s!();
                s[sp - 2] = value;
                if $wide {
                    s[sp - 1] = Value::Void;
                } else {
                    sp -= 1;
                }
            }};
        }
        macro_rules! astore {
            ($variant:ident, $unwrap:expr, $wide:expr) => {{
                let s = &s!();
                let (value, index, arr) = if $wide {
                    (s[sp - 2], s[sp - 3].int(), s[sp - 4].obj())
                } else {
                    (s[sp - 1], s[sp - 2].int(), s[sp - 3].obj())
                };
                let Some(arr) = arr else {
                    break Err(Fault::NullPointer);
                };
                match &mut ctx.state.heap.get_mut(arr).body {
                    Body::$variant(a) => {
                        let len = a.len();
                        match a.get_mut(index as usize) {
                            Some(slot) => *slot = $unwrap(value),
                            None => break Err(Fault::ArrayIndex { index, len }),
                        }
                    }
                    _ => break Err(wrong_array()),
                }
                sp -= if $wide { 4 } else { 3 };
            }};
        }
        macro_rules! fcmp {
            ($nan:expr) => {{
                let s = &mut s!();
                let b = s[sp - 1].float();
                let a = s[sp - 2].float();
                s[sp - 2] = Value::Int(compare(a, b, $nan));
                sp -= 1;
            }};
        }
        macro_rules! dcmp {
            ($nan:expr) => {{
                let s = &mut s!();
                let b = s[sp - 2].double();
                let a = s[sp - 4].double();
                s[sp - 4] = Value::Int(compare(a, b, $nan));
                sp -= 3;
            }};
        }

        let outcome: Result<Flow, Fault> = loop {
            let insn = &insns[pc];
            pc += 1;
            match *insn {
                Insn::Nop => {}
                Insn::AconstNull => push!(Value::NULL),
                Insn::Iconst(v) => push!(Value::Int(v)),
                Insn::Lconst(v) => push2!(Value::Long(v)),
                Insn::Fconst(v) => push!(Value::Float(v)),
                Insn::Dconst(v) => push2!(Value::Double(v)),
                Insn::LdcString(i) => {
                    let r = match pool.cache[i as usize].get() {
                        Some(Resolved::String(r)) => *r,
                        _ => {
                            sync!();
                            let r = tri!(ldc_string(ctx, pool, i));
                            again!();
                            r
                        }
                    };
                    push!(Value::from(r));
                }
                Insn::LdcClass(i) => {
                    sync!();
                    let mirror = tri!(ldc_class(ctx, pool, i));
                    again!();
                    push!(Value::from(mirror));
                }
                Insn::Load(n) => {
                    let s = &mut s!();
                    s[sp] = s[bp + n as usize];
                    sp += 1;
                }
                Insn::ILoad(n) => {
                    let s = &mut s!();
                    s[sp] = Value::Int(s[bp + n as usize].int_or_zero());
                    sp += 1;
                }
                Insn::FLoad(n) => {
                    let s = &mut s!();
                    s[sp] = Value::Float(s[bp + n as usize].float_or_zero());
                    sp += 1;
                }
                Insn::LLoad(n) => {
                    let s = &mut s!();
                    s[sp] = Value::Long(s[bp + n as usize].long_or_zero());
                    s[sp + 1] = Value::Void;
                    sp += 2;
                }
                Insn::DLoad(n) => {
                    let s = &mut s!();
                    s[sp] = Value::Double(s[bp + n as usize].double_or_zero());
                    s[sp + 1] = Value::Void;
                    sp += 2;
                }
                Insn::Store(n) => {
                    let s = &mut s!();
                    sp -= 1;
                    s[bp + n as usize] = s[sp];
                }
                Insn::IStore(n) => {
                    let s = &mut s!();
                    sp -= 1;
                    s[bp + n as usize] = Value::Int(s[sp].int_or_zero());
                }
                Insn::FStore(n) => {
                    let s = &mut s!();
                    sp -= 1;
                    s[bp + n as usize] = Value::Float(s[sp].float_or_zero());
                }
                Insn::LStore(n) => {
                    let s = &mut s!();
                    sp -= 2;
                    let i = bp + n as usize;
                    s[i] = Value::Long(s[sp].long_or_zero());
                    s[i + 1] = Value::Void;
                }
                Insn::DStore(n) => {
                    let s = &mut s!();
                    sp -= 2;
                    let i = bp + n as usize;
                    s[i] = Value::Double(s[sp].double_or_zero());
                    s[i + 1] = Value::Void;
                }
                Insn::Iaload => aload!(Ints, Value::Int, false),
                Insn::Laload => aload!(Longs, Value::Long, true),
                Insn::Faload => aload!(Floats, Value::Float, false),
                Insn::Daload => aload!(Doubles, Value::Double, true),
                Insn::Aaload => aload!(Refs, Value::Ref, false),
                Insn::Baload => aload!(Bytes, |v: i8| Value::Int(i32::from(v)), false),
                Insn::Caload => aload!(Chars, |v: u16| Value::Int(i32::from(v)), false),
                Insn::Saload => aload!(Shorts, |v: i16| Value::Int(i32::from(v)), false),
                Insn::Iastore => astore!(Ints, Value::int, false),
                Insn::Lastore => astore!(Longs, Value::long, true),
                Insn::Fastore => astore!(Floats, Value::float, false),
                Insn::Dastore => astore!(Doubles, Value::double, true),
                Insn::Castore => astore!(Chars, |v: Value| v.int() as u16, false),
                Insn::Sastore => astore!(Shorts, |v: Value| v.int() as i16, false),
                Insn::Bastore => {
                    let s = &s!();
                    let (value, index, arr) = (s[sp - 1].int(), s[sp - 2].int(), s[sp - 3].obj());
                    let Some(arr) = arr else {
                        break Err(Fault::NullPointer);
                    };
                    let st = &mut *ctx.state;
                    // A boolean array stores only the low bit (JVMS 6.5.bastore).
                    let value = if value & !1 != 0
                        && st.classes.get(st.heap.class_of(arr)).kind
                            == ClassKind::PrimArray(ArrayType::Boolean)
                    {
                        value & 1
                    } else {
                        value
                    };
                    match &mut st.heap.get_mut(arr).body {
                        Body::Bytes(a) => {
                            let len = a.len();
                            match a.get_mut(index as usize) {
                                Some(slot) => *slot = value as i8,
                                None => break Err(Fault::ArrayIndex { index, len }),
                            }
                        }
                        _ => break Err(wrong_array()),
                    }
                    sp -= 3;
                }
                Insn::Aastore => {
                    let s = &s!();
                    let (value, index, arr) = (s[sp - 1].obj(), s[sp - 2].int(), s[sp - 3].obj());
                    let Some(arr) = arr else {
                        break Err(Fault::NullPointer);
                    };
                    let st = &mut *ctx.state;
                    let len = st.heap.get(arr).body.array_len().unwrap_or(0);
                    if index < 0 || index as usize >= len {
                        break Err(Fault::ArrayIndex { index, len });
                    }
                    if let Some(v) = value {
                        let ClassKind::RefArray(elem) = st.classes.get(st.heap.class_of(arr)).kind
                        else {
                            break Err(wrong_array());
                        };
                        let vc = st.heap.class_of(v);
                        if vc != elem && !st.classes.is_assignable(vc, elem) {
                            break Err(Fault::ArrayStore(vc));
                        }
                    }
                    match &mut st.heap.get_mut(arr).body {
                        Body::Refs(a) => a[index as usize] = value,
                        _ => break Err(wrong_array()),
                    }
                    sp -= 3;
                }
                Insn::Pop => sp -= 1,
                Insn::Pop2 => sp -= 2,
                Insn::Dup => {
                    let s = &mut s!();
                    s[sp] = s[sp - 1];
                    sp += 1;
                }
                Insn::DupX1 => {
                    let s = &mut s!();
                    let (v1, v2) = (s[sp - 1], s[sp - 2]);
                    s[sp - 2] = v1;
                    s[sp - 1] = v2;
                    s[sp] = v1;
                    sp += 1;
                }
                Insn::DupX2 => {
                    let s = &mut s!();
                    let (v1, v2, v3) = (s[sp - 1], s[sp - 2], s[sp - 3]);
                    s[sp - 3] = v1;
                    s[sp - 2] = v3;
                    s[sp - 1] = v2;
                    s[sp] = v1;
                    sp += 1;
                }
                Insn::Dup2 => {
                    let s = &mut s!();
                    s[sp] = s[sp - 2];
                    s[sp + 1] = s[sp - 1];
                    sp += 2;
                }
                Insn::Dup2X1 => {
                    let s = &mut s!();
                    let (v1, v2, v3) = (s[sp - 1], s[sp - 2], s[sp - 3]);
                    s[sp - 3] = v2;
                    s[sp - 2] = v1;
                    s[sp - 1] = v3;
                    s[sp] = v2;
                    s[sp + 1] = v1;
                    sp += 2;
                }
                Insn::Dup2X2 => {
                    let s = &mut s!();
                    let (v1, v2, v3, v4) = (s[sp - 1], s[sp - 2], s[sp - 3], s[sp - 4]);
                    s[sp - 4] = v2;
                    s[sp - 3] = v1;
                    s[sp - 2] = v4;
                    s[sp - 1] = v3;
                    s[sp] = v2;
                    s[sp + 1] = v1;
                    sp += 2;
                }
                Insn::Swap => s!().swap(sp - 1, sp - 2),
                Insn::Iadd => ibin!(i32::wrapping_add),
                Insn::Ladd => lbin!(i64::wrapping_add),
                Insn::Fadd => fbin!(|a: f32, b: f32| a + b),
                Insn::Dadd => dbin!(|a: f64, b: f64| a + b),
                Insn::Isub => ibin!(i32::wrapping_sub),
                Insn::Lsub => lbin!(i64::wrapping_sub),
                Insn::Fsub => fbin!(|a: f32, b: f32| a - b),
                Insn::Dsub => dbin!(|a: f64, b: f64| a - b),
                Insn::Imul => ibin!(i32::wrapping_mul),
                Insn::Lmul => lbin!(i64::wrapping_mul),
                Insn::Fmul => fbin!(|a: f32, b: f32| a * b),
                Insn::Dmul => dbin!(|a: f64, b: f64| a * b),
                Insn::Idiv | Insn::Irem => {
                    let s = &mut s!();
                    let b = s[sp - 1].int();
                    if b == 0 {
                        break Err(Fault::DivideByZero);
                    }
                    let a = s[sp - 2].int();
                    s[sp - 2] = Value::Int(if matches!(*insn, Insn::Idiv) {
                        a.wrapping_div(b)
                    } else {
                        a.wrapping_rem(b)
                    });
                    sp -= 1;
                }
                Insn::Ldiv | Insn::Lrem => {
                    let s = &mut s!();
                    let b = s[sp - 2].long();
                    if b == 0 {
                        break Err(Fault::DivideByZero);
                    }
                    let a = s[sp - 4].long();
                    s[sp - 4] = Value::Long(if matches!(*insn, Insn::Ldiv) {
                        a.wrapping_div(b)
                    } else {
                        a.wrapping_rem(b)
                    });
                    sp -= 2;
                }
                Insn::Fdiv => fbin!(|a: f32, b: f32| a / b),
                Insn::Ddiv => dbin!(|a: f64, b: f64| a / b),
                Insn::Frem => fbin!(|a: f32, b: f32| a % b),
                Insn::Drem => dbin!(|a: f64, b: f64| a % b),
                Insn::Ineg => {
                    let s = &mut s!();
                    s[sp - 1] = Value::Int(s[sp - 1].int().wrapping_neg());
                }
                Insn::Lneg => {
                    let s = &mut s!();
                    s[sp - 2] = Value::Long(s[sp - 2].long().wrapping_neg());
                }
                Insn::Fneg => {
                    let s = &mut s!();
                    s[sp - 1] = Value::Float(-s[sp - 1].float());
                }
                Insn::Dneg => {
                    let s = &mut s!();
                    s[sp - 2] = Value::Double(-s[sp - 2].double());
                }
                Insn::Ishl => ibin!(|a: i32, b: i32| a.wrapping_shl(b as u32)),
                Insn::Ishr => ibin!(|a: i32, b: i32| a.wrapping_shr(b as u32)),
                Insn::Iushr => ibin!(|a: i32, b: i32| (a as u32).wrapping_shr(b as u32) as i32),
                Insn::Lshl => lshift!(i64::wrapping_shl),
                Insn::Lshr => lshift!(i64::wrapping_shr),
                Insn::Lushr => lshift!(|a: i64, n: u32| (a as u64).wrapping_shr(n) as i64),
                Insn::Iand => ibin!(|a: i32, b: i32| a & b),
                Insn::Land => lbin!(|a: i64, b: i64| a & b),
                Insn::Ior => ibin!(|a: i32, b: i32| a | b),
                Insn::Lor => lbin!(|a: i64, b: i64| a | b),
                Insn::Ixor => ibin!(|a: i32, b: i32| a ^ b),
                Insn::Lxor => lbin!(|a: i64, b: i64| a ^ b),
                Insn::Iinc(n, d) => {
                    let s = &mut s!();
                    let i = bp + n as usize;
                    s[i] = Value::Int(s[i].int().wrapping_add(i32::from(d)));
                }
                Insn::I2l => {
                    let v = s!()[sp - 1].int();
                    sp -= 1;
                    push2!(Value::Long(i64::from(v)));
                }
                Insn::I2f => {
                    let s = &mut s!();
                    s[sp - 1] = Value::Float(s[sp - 1].int() as f32);
                }
                Insn::I2d => {
                    let v = s!()[sp - 1].int();
                    sp -= 1;
                    push2!(Value::Double(f64::from(v)));
                }
                Insn::L2i => {
                    let s = &mut s!();
                    s[sp - 2] = Value::Int(s[sp - 2].long() as i32);
                    sp -= 1;
                }
                Insn::L2f => {
                    let s = &mut s!();
                    s[sp - 2] = Value::Float(s[sp - 2].long() as f32);
                    sp -= 1;
                }
                Insn::L2d => {
                    let s = &mut s!();
                    s[sp - 2] = Value::Double(s[sp - 2].long() as f64);
                }
                Insn::F2i => {
                    let s = &mut s!();
                    s[sp - 1] = Value::Int(s[sp - 1].float() as i32);
                }
                Insn::F2l => {
                    let v = s!()[sp - 1].float();
                    sp -= 1;
                    push2!(Value::Long(v as i64));
                }
                Insn::F2d => {
                    let v = s!()[sp - 1].float();
                    sp -= 1;
                    push2!(Value::Double(f64::from(v)));
                }
                Insn::D2i => {
                    let s = &mut s!();
                    s[sp - 2] = Value::Int(s[sp - 2].double() as i32);
                    sp -= 1;
                }
                Insn::D2l => {
                    let s = &mut s!();
                    s[sp - 2] = Value::Long(s[sp - 2].double() as i64);
                }
                Insn::D2f => {
                    let s = &mut s!();
                    s[sp - 2] = Value::Float(s[sp - 2].double() as f32);
                    sp -= 1;
                }
                Insn::I2b => {
                    let s = &mut s!();
                    s[sp - 1] = Value::Int(i32::from(s[sp - 1].int() as i8));
                }
                Insn::I2c => {
                    let s = &mut s!();
                    s[sp - 1] = Value::Int(i32::from(s[sp - 1].int() as u16));
                }
                Insn::I2s => {
                    let s = &mut s!();
                    s[sp - 1] = Value::Int(i32::from(s[sp - 1].int() as i16));
                }
                Insn::Lcmp => {
                    let s = &mut s!();
                    let b = s[sp - 2].long();
                    let a = s[sp - 4].long();
                    s[sp - 4] = Value::Int(a.cmp(&b) as i32);
                    sp -= 3;
                }
                Insn::Fcmpl => fcmp!(-1),
                Insn::Fcmpg => fcmp!(1),
                Insn::Dcmpl => dcmp!(-1),
                Insn::Dcmpg => dcmp!(1),
                Insn::Ifeq(t) => if1!(t, |v: i32| v == 0),
                Insn::Ifne(t) => if1!(t, |v: i32| v != 0),
                Insn::Iflt(t) => if1!(t, |v: i32| v < 0),
                Insn::Ifge(t) => if1!(t, |v: i32| v >= 0),
                Insn::Ifgt(t) => if1!(t, |v: i32| v > 0),
                Insn::Ifle(t) => if1!(t, |v: i32| v <= 0),
                Insn::IfIcmpeq(t) => if2!(t, |a: i32, b: i32| a == b),
                Insn::IfIcmpne(t) => if2!(t, |a: i32, b: i32| a != b),
                Insn::IfIcmplt(t) => if2!(t, |a: i32, b: i32| a < b),
                Insn::IfIcmpge(t) => if2!(t, |a: i32, b: i32| a >= b),
                Insn::IfIcmpgt(t) => if2!(t, |a: i32, b: i32| a > b),
                Insn::IfIcmple(t) => if2!(t, |a: i32, b: i32| a <= b),
                Insn::IfAcmpeq(t) | Insn::IfAcmpne(t) => {
                    sp -= 2;
                    let same = s!()[sp].obj() == s!()[sp + 1].obj();
                    if same == matches!(*insn, Insn::IfAcmpeq(_)) {
                        jump!(t);
                    }
                }
                Insn::IfNull(t) | Insn::IfNonNull(t) => {
                    sp -= 1;
                    let null = s!()[sp].obj().is_none();
                    if null == matches!(*insn, Insn::IfNull(_)) {
                        jump!(t);
                    }
                }
                Insn::Goto(t) => jump!(t),
                Insn::Jsr(t) => {
                    push!(Value::RetAddr(pc as u32));
                    jump!(t);
                }
                Insn::Ret(n) => {
                    let Value::RetAddr(t) = s!()[bp + n as usize] else {
                        break Err(verify_error("ret through a local without a return address"));
                    };
                    // The decoder assumed the subroutine leaves the stack
                    // as its jsr found it; a return address from another
                    // method or another height would run off the frame.
                    let depth = sp - bp - code.max_locals as usize;
                    if code.ret_heights.get(t as usize) != Some(&(depth as u32)) {
                        break Err(verify_error(
                            "a subroutine returns with a different operand stack",
                        ));
                    }
                    jump!(t);
                }
                Insn::TableSwitch(i) | Insn::LookupSwitch(i) => {
                    sp -= 1;
                    let key = s!()[sp].int();
                    jump!(code.switches[i as usize].target(key));
                }
                Insn::Return1 => break Ok(Flow::Return(s!()[sp - 1])),
                Insn::Return2 => break Ok(Flow::Return(s!()[sp - 2])),
                Insn::Return => break Ok(Flow::Return(Value::Void)),
                Insn::GetStatic(i) | Insn::PutStatic(i) => {
                    let (class, index, wide) = match pool.cache[i as usize].get() {
                        Some(Resolved::Field(FieldRes::Static { class, index, wide })) => {
                            (*class, *index, *wide)
                        }
                        _ => {
                            sync!();
                            let r = tri!(static_field(ctx, pool, i));
                            again!();
                            r
                        }
                    };
                    if ctx.state.classes.init[class.index()] != InitState::Done {
                        sync!();
                        tri!(initialize(ctx, class));
                        again!();
                    }
                    let index = index as usize;
                    if matches!(*insn, Insn::GetStatic(_)) {
                        let v = ctx.state.classes.statics[index];
                        if wide {
                            push2!(v);
                        } else {
                            push!(v);
                        }
                    } else {
                        sp -= if wide { 2 } else { 1 };
                        let v = s!()[sp];
                        ctx.state.classes.statics[index] = v;
                    }
                }
                Insn::GetField(i) | Insn::PutField(i) => {
                    let (slot, wide) = match pool.cache[i as usize].get() {
                        Some(Resolved::Field(FieldRes::Instance { slot, wide })) => {
                            (*slot as usize, *wide)
                        }
                        _ => {
                            sync!();
                            let r = tri!(instance_field(ctx, pool, i));
                            again!();
                            r
                        }
                    };
                    if matches!(*insn, Insn::GetField(_)) {
                        let Some(obj) = s!()[sp - 1].obj() else {
                            break Err(Fault::NullPointer);
                        };
                        let v = match &ctx.state.heap.get(obj).body {
                            Body::Fields(f) => match f.get(slot) {
                                Some(&v) => v,
                                None => break Err(wrong_field()),
                            },
                            _ => break Err(wrong_field()),
                        };
                        let s = &mut s!();
                        s[sp - 1] = v;
                        if wide {
                            s[sp] = Value::Void;
                            sp += 1;
                        }
                    } else {
                        let width = if wide { 2 } else { 1 };
                        let v = s!()[sp - width];
                        let Some(obj) = s!()[sp - width - 1].obj() else {
                            break Err(Fault::NullPointer);
                        };
                        match &mut ctx.state.heap.get_mut(obj).body {
                            Body::Fields(f) => match f.get_mut(slot) {
                                Some(field) => *field = v,
                                None => break Err(wrong_field()),
                            },
                            _ => break Err(wrong_field()),
                        }
                        sp -= width + 1;
                    }
                }
                Insn::InvokeVirtual(_)
                | Insn::InvokeSpecial(_)
                | Insn::InvokeStatic(_)
                | Insn::InvokeInterface(_) => {
                    sync!();
                    budget -= 1;
                    if budget == 0 {
                        budget = YIELD_INTERVAL;
                        tri!(safepoint(ctx));
                    }
                    let target = match select(ctx, pool, *insn, sp) {
                        Ok(m) => m,
                        Err(f) => break Err(f),
                    };
                    match call(ctx, fi, target, &mut sp) {
                        Ok(true) => break Ok(Flow::Call),
                        Ok(false) => {
                            if ctx.state.heap.gc_requested()
                                || ctx.shared.is_stopping()
                                || ctx.shared.is_paused()
                            {
                                sync!();
                                tri!(safepoint_after_native(ctx));
                            }
                            again!();
                        }
                        Err(f) => break Err(f),
                    }
                }
                Insn::New(i) => {
                    sync!();
                    let obj = tri!(new_object(ctx, pool, i));
                    again!();
                    push!(Value::from(obj));
                }
                Insn::NewArray(atype) => {
                    let n = s!()[sp - 1].int();
                    sync!();
                    let arr = tri!(new_prim_array(ctx, atype, n));
                    again!();
                    s!()[sp - 1] = Value::from(arr);
                }
                Insn::ANewArray(i) => {
                    let n = s!()[sp - 1].int();
                    sync!();
                    let arr = tri!(new_ref_array(ctx, pool, i, n));
                    again!();
                    s!()[sp - 1] = Value::from(arr);
                }
                Insn::MultiANewArray(i, dims) => {
                    sync!();
                    let dims = dims as usize;
                    let counts: SmallVec<[i32; 4]> =
                        s!()[sp - dims..sp].iter().map(|v| v.int()).collect();
                    let arr = tri!(new_multi_array(ctx, pool, i, &counts));
                    again!();
                    sp -= dims;
                    push!(Value::from(arr));
                }
                Insn::ArrayLength => {
                    let Some(arr) = s!()[sp - 1].obj() else {
                        break Err(Fault::NullPointer);
                    };
                    let Some(len) = ctx.state.heap.get(arr).body.array_len() else {
                        break Err(wrong_array());
                    };
                    s!()[sp - 1] = Value::Int(len as i32);
                }
                Insn::Athrow => {
                    let Some(e) = s!()[sp - 1].obj() else {
                        break Err(Fault::NullPointer);
                    };
                    break Err(athrow(ctx, e));
                }
                Insn::CheckCast(i) => {
                    if let Some(obj) = s!()[sp - 1].obj() {
                        let target = match pool.cache[i as usize].get() {
                            Some(Resolved::Class(c)) => *c,
                            _ => {
                                sync!();
                                let r = tri!(resolve_class(ctx, pool, i));
                                again!();
                                r
                            }
                        };
                        let oc = ctx.state.heap.class_of(obj);
                        if oc != target && !ctx.state.classes.is_assignable(oc, target) {
                            break Err(Fault::ClassCast {
                                from: oc,
                                to: target,
                            });
                        }
                    }
                }
                Insn::InstanceOf(i) => {
                    let result = match s!()[sp - 1].obj() {
                        None => 0,
                        Some(obj) => {
                            let target = match pool.cache[i as usize].get() {
                                Some(Resolved::Class(c)) => *c,
                                _ => {
                                    sync!();
                                    let r = tri!(resolve_class(ctx, pool, i));
                                    again!();
                                    r
                                }
                            };
                            let oc = ctx.state.heap.class_of(obj);
                            i32::from(oc == target || ctx.state.classes.is_assignable(oc, target))
                        }
                    };
                    s!()[sp - 1] = Value::Int(result);
                }
                Insn::MonitorEnter | Insn::MonitorExit => {
                    sp -= 1;
                    let Some(obj) = s!()[sp].obj() else {
                        break Err(Fault::NullPointer);
                    };
                    sync!();
                    tri!(monitor(ctx, obj, matches!(*insn, Insn::MonitorEnter)));
                    again!();
                }
                Insn::Unsupported(op) => break Err(unsupported(op)),
            }
        };

        match outcome {
            Ok(Flow::Call) => {}
            Ok(Flow::Return(v)) => {
                let frame = ctx.thread.frames.pop().expect("the frame that returned");
                if let Some(obj) = frame.sync {
                    let _ = ctx.monitor_exit(obj);
                }
                if ctx.thread.frames.len() == base {
                    ctx.thread.locals.truncate(locals_mark);
                    return Ok(v);
                }
                let t = &mut *ctx.thread;
                let caller = t
                    .frames
                    .last_mut()
                    .expect("a caller below the returning frame");
                let at = caller.sp as usize;
                // As many slots as the callee declares, whatever it
                // returned: the caller's stack walk counted on that, and
                // bytecode can load a long as one slot or the second half
                // of one.
                match method.ret.slots() {
                    0 => {}
                    2 => {
                        t.slots[at] = v;
                        t.slots[at + 1] = Value::Void;
                        caller.sp += 2;
                    }
                    _ => {
                        t.slots[at] = v;
                        caller.sp += 1;
                    }
                }
            }
            Err(fault) => {
                sync!();
                let thrown = match fault.into_throw(ctx) {
                    Throw::Exception(exc) => ctx.judge_thrown(exc),
                    stop @ Throw::Stop => stop,
                };
                ctx.thread.locals.truncate(locals_mark);
                match thrown {
                    Throw::Stop => {
                        ctx.thread.frames.truncate(base);
                        return Err(Throw::Stop);
                    }
                    Throw::Exception(exc) => {
                        if !catch(ctx, base, exc) {
                            return Err(Throw::Exception(exc));
                        }
                        // A handler has the exception on its stack, and
                        // the frame is written back, so a collection here
                        // sees it. Without this a handler that throws
                        // again is a loop with no safepoint.
                        budget = budget.saturating_sub(HANDLER_COST);
                        if budget == 0 {
                            budget = YIELD_INTERVAL;
                            if let Err(stop) = ctx.safepoint() {
                                ctx.thread.frames.truncate(base);
                                return Err(stop);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// `fcmpl`/`fcmpg` and `dcmpl`/`dcmpg`: `nan` is the result when either
/// operand is NaN.
#[inline]
fn compare<T: PartialOrd>(a: T, b: T, nan: i32) -> i32 {
    if a > b {
        1
    } else if a == b {
        0
    } else if a < b {
        -1
    } else {
        nan
    }
}

impl Ctx<'_> {
    /// The class of arrays of `elem`, cached on the element class.
    pub(crate) fn array_class_of(&mut self, elem: ClassId) -> JResult<ClassId> {
        if let Some(&c) = self.state.classes.get(elem).array_type.get() {
            return Ok(c);
        }
        match self.state.classes.array_of(elem) {
            Ok(c) => {
                let _ = self.state.classes.get(elem).array_type.set(c);
                Ok(c)
            }
            Err(e) => Err(self.load_error(e)),
        }
    }
}
