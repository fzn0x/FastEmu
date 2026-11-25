//! `java.util.Timer` and `TimerTask`, which MIDP adds to CLDC.
//!
//! Each `Timer` runs its tasks one at a time on a Java thread of its own
//! named `Timer-n`, whose body is Rust. Scheduling follows the JDK's
//! `Timer`: `schedule` with a period is fixed-delay (the next run counts
//! from the start of this one) and `scheduleAtFixedRate` fixed-rate (from
//! when this one was due, so late runs catch up).
//!
//! Tasks are due on the machine's run time ([`crate::VmClock`]), which
//! stands still while the machine is paused, so a pause does not make
//! runs late: a fixed-rate task paused for a minute goes on at its rate
//! when the machine resumes, where on the wall clock it would run a
//! minute of tasks at once. A `Date` given to `schedule` is wall time and
//! counts from now; `scheduledExecutionTime` reports wall time.
//!
//! Unlike Java SE, where an exception in a task kills the timer thread
//! and every other task, a task that throws is reported as uncaught and
//! cancelled, and the timer goes on with the rest (rule 16 of
//! `docs/compat/emulator-notes.md`, as J2ME Loader does).
//!
//! As on Java SE, a timer's thread holds its queue, not the timer. The
//! queue holds its timer weakly, in the machine's weak table, so the
//! collection that finds a timer dropped marks its queue ([`reap`]), and
//! the thread ends once no task is left. At the thread limit,
//! `Thread.start` and `new Timer()` collect once and wait for such
//! threads to end ([`make_room`]).

use std::time::{Duration, Instant};

use crate::classfile::access::{PRIVATE, PROTECTED, STATIC};
use crate::ctx::{Ctx, GlobalRef, JResult, Throw, Wake};
use crate::heap::{Payload, Tracer};
use crate::lang::system::current_time_millis;
use crate::lang::throwable::report_uncaught;
use crate::lang::{this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};
use crate::vm::VmState;

const TIMER: &str = "java/util/Timer";
const TASK: &str = "java/util/TimerTask";
const QUEUE: &str = "java/util/Timer$Queue";

/// Where a `TimerTask` is in its life.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Phase {
    /// Not scheduled yet.
    #[default]
    Virgin,
    /// Waiting in a timer's queue.
    Scheduled,
    /// A one-shot task that has run (or is running).
    Executed,
    Cancelled,
}

/// What a `TimerTask` keeps.
#[derive(Default)]
struct Task {
    phase: Phase,
    /// When it runs next, in run-time milliseconds ([`run_millis`]).
    next: i64,
    /// 0 for a one-shot task, positive for fixed-rate, negative for
    /// fixed-delay, as in Java.
    period: i64,
}

impl Payload for Task {}

/// One task in a queue, ordered by when it runs and then by when it was
/// queued.
struct Slot {
    at: i64,
    order: u64,
    task: ObjRef,
}

/// The queue a `Timer` and its thread share.
struct Queue {
    /// Sorted by `(at, order)`.
    slots: Vec<Slot>,
    next_order: u64,
    /// False once the timer is cancelled.
    open: bool,
    /// Set by the collection that finds the timer dropped. Nothing can
    /// schedule on it again, so its thread ends once no task is left.
    reaped: bool,
    /// While the thread runs a task, and reports what the task threw:
    /// both run the game's code.
    running: bool,
}

impl Default for Queue {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            next_order: 0,
            open: true,
            reaped: false,
            running: false,
        }
    }
}

impl Payload for Queue {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        for slot in &self.slots {
            tracer.mark(slot.task);
        }
    }
}

impl Queue {
    fn insert(&mut self, at: i64, task: ObjRef) {
        let order = self.next_order;
        self.next_order += 1;
        let i = self
            .slots
            .partition_point(|s| (s.at, s.order) <= (at, order));
        self.slots.insert(i, Slot { at, order, task });
    }

    fn remove(&mut self, task: ObjRef) {
        self.slots.retain(|s| s.task != task);
    }
}

/// What a `Timer` keeps: its queue.
#[derive(Default)]
struct TimerData {
    queue: Option<ObjRef>,
}

impl Payload for TimerData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.queue);
    }
}

fn task<'a>(ctx: &'a mut Ctx<'_>, obj: ObjRef) -> JResult<&'a mut Task> {
    ctx.payload_mut::<Task>(obj)
}

fn queue<'a>(ctx: &'a mut Ctx<'_>, obj: ObjRef) -> JResult<&'a mut Queue> {
    ctx.payload_mut::<Queue>(obj)
}

/// The machine's run time in milliseconds: the clock tasks are due on.
fn run_millis(ctx: &Ctx<'_>) -> i64 {
    ctx.clock().elapsed().as_millis() as i64
}

/// Waits until something changes or `deadline` passes. An interrupt of
/// the timer thread is cleared, as Java's timer thread ignores it, and a
/// paused virtual machine parks the thread until it resumes.
fn wait(ctx: &mut Ctx<'_>, deadline: Option<Instant>) -> JResult<()> {
    match ctx.wait_until(deadline)? {
        Wake::Interrupted => {
            ctx.take_interrupt();
        }
        Wake::Paused => ctx.park_while_paused()?,
        Wake::Notified | Wake::TimedOut => {}
    }
    Ok(())
}

/// One turn of a timer thread: runs the first task if it is due, or
/// waits. Returns false when the queue is empty and nothing can schedule
/// on it again, because the timer is cancelled or a collection found it
/// dropped, which ends the thread.
fn turn(ctx: &mut Ctx<'_>, queue_ref: &GlobalRef) -> JResult<bool> {
    let q_obj = queue_ref.get(ctx);
    let q = queue(ctx, q_obj)?;
    let (first, open, reaped) = (q.slots.first().map(|s| s.task), q.open, q.reaped);
    let Some(t_obj) = first else {
        if !open || reaped {
            return Ok(false);
        }
        wait(ctx, None)?;
        return Ok(true);
    };
    let t_obj = ctx.root(t_obj);
    let t = task(ctx, t_obj)?;
    let (phase, due, period) = (t.phase, t.next, t.period);
    if phase == Phase::Cancelled {
        queue(ctx, q_obj)?.remove(t_obj);
        return Ok(true);
    }
    let now = run_millis(ctx);
    if due > now {
        let at = ctx.clock().instant_at(Duration::from_millis(due as u64));
        wait(ctx, at)?;
        return Ok(true);
    }
    if period == 0 {
        task(ctx, t_obj)?.phase = Phase::Executed;
        queue(ctx, q_obj)?.remove(t_obj);
    } else {
        let next = if period < 0 {
            now - period
        } else {
            due + period
        };
        task(ctx, t_obj)?.next = next;
        let q = queue(ctx, q_obj)?;
        q.remove(t_obj);
        q.insert(next, t_obj);
    }
    // Until the report is done too: it calls the exception's `toString`,
    // which is the game's code, and `make_room` must never wait for a
    // thread that runs the game's code.
    queue(ctx, q_obj)?.running = true;
    match ctx.call_virtual(t_obj, "run", "()V", &[]) {
        Ok(_) => {}
        Err(Throw::Stop) => return Err(Throw::Stop),
        Err(Throw::Exception(e)) => {
            report_uncaught(ctx, e);
            task(ctx, t_obj)?.phase = Phase::Cancelled;
            queue(ctx, q_obj)?.remove(t_obj);
        }
    }
    queue(ctx, q_obj)?.running = false;
    Ok(true)
}

/// The queue of the timer `timer`; a timer no constructor ran on has
/// no thread, and counts as cancelled.
fn timer_queue(ctx: &mut Ctx<'_>, timer: ObjRef) -> JResult<ObjRef> {
    match ctx.payload::<TimerData>(timer)?.queue {
        Some(q) => Ok(ctx.root(q)),
        None => throw(
            ctx,
            "java/lang/IllegalStateException",
            "Timer already cancelled.",
        ),
    }
}

/// Java's `Timer.sched`: puts `task_obj` on the queue to run at `time`
/// (run-time milliseconds), with `period` as Java stores it.
fn schedule(
    ctx: &mut Ctx<'_>,
    timer: ObjRef,
    task_obj: Value,
    time: i64,
    period: i64,
) -> JResult<Value> {
    let task_obj = ctx.non_null(task_obj)?;
    // Java halves huge periods so that adding one never overflows.
    let period = if period.unsigned_abs() > (i64::MAX >> 1) as u64 {
        period >> 1
    } else {
        period
    };
    let q_obj = timer_queue(ctx, timer)?;
    if !queue(ctx, q_obj)?.open {
        return throw(
            ctx,
            "java/lang/IllegalStateException",
            "Timer already cancelled.",
        );
    }
    let t = task(ctx, task_obj)?;
    if t.phase != Phase::Virgin {
        return throw(
            ctx,
            "java/lang/IllegalStateException",
            "Task already scheduled or cancelled",
        );
    }
    t.phase = Phase::Scheduled;
    t.next = time;
    t.period = period;
    queue(ctx, q_obj)?.insert(time, task_obj);
    ctx.notify_waiters();
    Ok(Value::Void)
}

/// The run time `delay` milliseconds from now.
fn check_delay(ctx: &mut Ctx<'_>, delay: i64) -> JResult<i64> {
    if delay < 0 {
        return throw(ctx, "java/lang/IllegalArgumentException", "Negative delay.");
    }
    Ok(run_millis(ctx).saturating_add(delay))
}

fn check_period(ctx: &mut Ctx<'_>, period: i64) -> JResult<i64> {
    if period <= 0 {
        return throw(
            ctx,
            "java/lang/IllegalArgumentException",
            "Non-positive period.",
        );
    }
    Ok(period)
}

/// The run time of the wall time `date` holds: as far from the run time
/// now as the date is from `System.currentTimeMillis()`. Java refuses a
/// date before 1970.
fn date_time(ctx: &mut Ctx<'_>, date: Value) -> JResult<i64> {
    let date = ctx.non_null(date)?;
    let millis = ctx.call_virtual(date, "getTime", "()J", &[])?.long();
    if millis < 0 {
        return throw(
            ctx,
            "java/lang/IllegalArgumentException",
            "Illegal execution time.",
        );
    }
    let from_now = millis.saturating_sub(current_time_millis());
    Ok(run_millis(ctx).saturating_add(from_now))
}

/// `new Timer()`: the queue, and the thread that runs it. The thread counts
/// against the game's threads, so past `max_threads` the constructor throws
/// `OutOfMemoryError` and leaves the timer as no constructor ran on it,
/// with no queue. It first makes room if it can ([`make_room`]).
fn init(ctx: &mut Ctx<'_>, timer: ObjRef) -> JResult<Value> {
    // First: it may collect and let other threads run. `timer` is this
    // native's argument, so a root.
    make_room(ctx)?;
    let class = ctx.find_class(QUEUE)?;
    let q_obj = ctx.new_object(class)?;
    let serial = ctx.static_field_id(TIMER, "serialNumber", "I")?;
    let n = ctx.get_static(serial)?.int();
    let queue_ref = ctx.new_global(q_obj);
    ctx.spawn_thread(&format!("Timer-{n}"), move |ctx| {
        let ended = loop {
            match ctx.local_frame(|ctx| turn(ctx, &queue_ref)) {
                Ok(true) => {}
                other => break other,
            }
        };
        // On every way out, before the thread can run the game's code
        // again (the report of an error it returns): `make_room` counts
        // the threads in the weak table as ones that end without it.
        let q_obj = queue_ref.get(ctx);
        ctx.state.weak.remove(&q_obj);
        ended.map(|_| ())
    })?;
    // The thread cannot run before this native returns, and a timer that
    // got no thread has changed nothing.
    ctx.set_static(serial, Value::Int(n.wrapping_add(1)))?;
    ctx.payload_mut::<TimerData>(timer)?.queue = Some(q_obj);
    // The thread holds the queue, not the timer. The queue holds its
    // timer as a `WeakReference` holds its referent, so a collection
    // finds out when the game drops it.
    ctx.state.weak.insert(q_obj, Some(timer));
    Ok(Value::Void)
}

/// Marks the queue of each timer this collection found dropped, and
/// returns whether it marked any. `Ctx::collect` calls it right after the
/// sweep, when every key of the weak table is alive, and wakes the
/// threads when it returns true: each ends once no task is left.
pub(crate) fn reap(state: &mut VmState) -> bool {
    let VmState { weak, heap, .. } = state;
    let mut any = false;
    for (&key, timer) in weak.iter() {
        if timer.is_some() {
            continue;
        }
        if let Some(q) = heap.get_mut(key).payload_mut::<Queue>()
            && q.open
            && !q.reaped
        {
            q.reaped = true;
            any = true;
        }
    }
    any
}

/// The timer threads that have no task to run, now or later, as two
/// counts. The first holds those that end at their next turn without
/// running the game's code, since their timer is cancelled or dropped.
/// The second holds the others, whose timer a collection may find
/// dropped. A thread that runs a task, or has one that is not cancelled,
/// is in neither.
fn spare(state: &VmState) -> (usize, usize) {
    let (mut ending, mut idle) = (0, 0);
    for &key in state.weak.keys() {
        let Some(q) = state.heap.get(key).payload::<Queue>() else {
            continue;
        };
        let has_task = q.slots.iter().any(|s| {
            state
                .heap
                .get(s.task)
                .payload::<Task>()
                .is_none_or(|t| t.phase != Phase::Cancelled)
        });
        if q.running || has_task {
            continue;
        }
        if q.reaped || !q.open {
            ending += 1;
        } else {
            idle += 1;
        }
    }
    (ending, idle)
}

/// Makes room for one more game thread when the machine has none left,
/// as a phone's collector frees the threads of the timers nobody holds.
/// If a timer thread has no task left, one collection finds the timers
/// the game dropped, then this waits for the threads of the dropped and
/// cancelled timers to end. It returns at once when there is room, or
/// when no timer thread could end; the caller then refuses as before.
///
/// It may collect and let other threads run, so `Thread.start` and
/// `new Timer()` call it before anything else. The wait counts only
/// threads that end without running the game's code, so it cannot wait
/// on a monitor the caller holds. A stop ends it; a pause lasts it until
/// the resume, as the ending threads park.
pub(crate) fn make_room(ctx: &mut Ctx<'_>) -> JResult<()> {
    let max = ctx.limits().max_threads;
    if ctx.state.live_threads < max {
        return Ok(());
    }
    let (ending, idle) = spare(&ctx.state);
    if ending == 0 {
        if idle == 0 {
            return Ok(());
        }
        ctx.gc();
    }
    // A thread that waits for the due time of a task the game has since
    // cancelled looks again now.
    ctx.notify_waiters();
    while ctx.state.live_threads >= max && spare(&ctx.state).0 > 0 {
        ctx.wait_raw(None)?;
    }
    Ok(())
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new(TASK)
            .abstract_class()
            .implements("java/lang/Runnable")
            .payload::<Task>()
            .method_with("<init>", "()V", PROTECTED, |_, _| Ok(Value::Void))
            .abstract_method("run", "()V")
            .method("cancel", "()Z", |ctx, a| {
                let t = task(ctx, this(a))?;
                let was_scheduled = t.phase == Phase::Scheduled;
                t.phase = Phase::Cancelled;
                Ok(Value::from(was_scheduled))
            })
            .method("scheduledExecutionTime", "()J", |ctx, a| {
                let t = task(ctx, this(a))?;
                let at = if t.period < 0 {
                    t.next.wrapping_add(t.period)
                } else {
                    t.next.wrapping_sub(t.period)
                };
                // In wall time, as games compare it with currentTimeMillis.
                Ok(Value::Long(ctx.shared.clock.wall_millis(at)))
            }),
        NativeClass::new(TIMER)
            .payload::<TimerData>()
            .field_with("serialNumber", "I", PRIVATE | STATIC)
            .method("<init>", "()V", |ctx, a| init(ctx, this(a)))
            .method("schedule", "(Ljava/util/TimerTask;J)V", |ctx, a| {
                let time = check_delay(ctx, a[2].long())?;
                schedule(ctx, this(a), a[1], time, 0)
            })
            .method(
                "schedule",
                "(Ljava/util/TimerTask;Ljava/util/Date;)V",
                |ctx, a| {
                    let time = date_time(ctx, a[2])?;
                    schedule(ctx, this(a), a[1], time, 0)
                },
            )
            .method("schedule", "(Ljava/util/TimerTask;JJ)V", |ctx, a| {
                let time = check_delay(ctx, a[2].long())?;
                let period = check_period(ctx, a[3].long())?;
                schedule(ctx, this(a), a[1], time, -period)
            })
            .method(
                "schedule",
                "(Ljava/util/TimerTask;Ljava/util/Date;J)V",
                |ctx, a| {
                    let period = check_period(ctx, a[3].long())?;
                    let time = date_time(ctx, a[2])?;
                    schedule(ctx, this(a), a[1], time, -period)
                },
            )
            .method(
                "scheduleAtFixedRate",
                "(Ljava/util/TimerTask;JJ)V",
                |ctx, a| {
                    let time = check_delay(ctx, a[2].long())?;
                    let period = check_period(ctx, a[3].long())?;
                    schedule(ctx, this(a), a[1], time, period)
                },
            )
            .method(
                "scheduleAtFixedRate",
                "(Ljava/util/TimerTask;Ljava/util/Date;J)V",
                |ctx, a| {
                    let period = check_period(ctx, a[3].long())?;
                    let time = date_time(ctx, a[2])?;
                    schedule(ctx, this(a), a[1], time, period)
                },
            )
            .method("cancel", "()V", |ctx, a| {
                let q_obj = timer_queue(ctx, this(a))?;
                let q = queue(ctx, q_obj)?;
                q.open = false;
                q.slots.clear();
                ctx.notify_waiters();
                Ok(Value::Void)
            }),
        NativeClass::new(QUEUE).final_class().payload::<Queue>(),
    ]
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use fastemu_core::LogLine;

    use super::*;
    use crate::{MemorySource, Vm, VmConfig};

    /// A task that throws each time it runs.
    fn boom() -> NativeClass {
        NativeClass::new("test/Boom")
            .extends(TASK)
            .method("<init>", "()V", |ctx, a| {
                ctx.call_special(TASK, this(a), "<init>", "()V", &[])?;
                Ok(Value::Void)
            })
            .method("run", "()V", |ctx, _| {
                ctx.throw("java/lang/RuntimeException", "boom")
            })
    }

    /// A task that counts its runs in a static field.
    fn count() -> NativeClass {
        NativeClass::new("test/Count")
            .extends(TASK)
            .static_field("runs", "I")
            .method("<init>", "()V", |ctx, a| {
                ctx.call_special(TASK, this(a), "<init>", "()V", &[])?;
                Ok(Value::Void)
            })
            .method("run", "()V", |ctx, _| {
                let runs = ctx.static_field_id("test/Count", "runs", "I")?;
                let n = ctx.get_static(runs)?.int();
                ctx.set_static(runs, Value::Int(n + 1))?;
                Ok(Value::Void)
            })
    }

    fn runs(ctx: &mut Ctx<'_>) -> i32 {
        let runs = ctx.static_field_id("test/Count", "runs", "I").unwrap();
        ctx.get_static(runs).unwrap().int()
    }

    /// Sleeps a few milliseconds at a time until `done` holds or ten
    /// seconds pass. A fixed sleep would be too short on a busy machine
    /// that runs the timer's thread late; the test then checks what it
    /// waited for.
    fn wait_for(ctx: &mut Ctx<'_>, mut done: impl FnMut(&mut Ctx<'_>) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while !done(ctx) && Instant::now() < deadline {
            ctx.sleep(Duration::from_millis(5)).unwrap();
        }
    }

    /// Rule 16 of `docs/compat/emulator-notes.md`: a task that throws is
    /// cancelled, and the timer keeps running its other tasks and takes
    /// new ones.
    #[test]
    fn a_throwing_task_is_cancelled_and_the_timer_goes_on() {
        let lines: Arc<Mutex<Vec<LogLine>>> = Arc::default();
        let sink = Arc::clone(&lines);
        let config = VmConfig {
            log: Arc::new(move |line| sink.lock().unwrap().push(line)),
            ..VmConfig::default()
        };
        let source = Arc::new(MemorySource::new());
        let vm = Vm::new(config, source, vec![boom(), count()]);
        let (boom_phase, before, after, late) = vm.attach("test", |ctx| {
            let timer = ctx.construct(TIMER, "()V", &[]).unwrap();
            let boom = ctx.construct("test/Boom", "()V", &[]).unwrap();
            let count = ctx.construct("test/Count", "()V", &[]).unwrap();
            let desc = "(Ljava/util/TimerTask;JJ)V";
            let every = [Value::Long(0), Value::Long(5)];
            ctx.call_virtual(
                timer,
                "scheduleAtFixedRate",
                desc,
                &[boom.into(), every[0], every[1]],
            )
            .unwrap();
            ctx.call_virtual(timer, "schedule", desc, &[count.into(), every[0], every[1]])
                .unwrap();
            // The throwing task is due first, so it has run by the
            // counter's second run.
            wait_for(ctx, |ctx| runs(ctx) >= 2);
            let before = runs(ctx);
            wait_for(ctx, |ctx| runs(ctx) > before);
            let after = runs(ctx);
            let boom_phase = ctx.payload::<Task>(boom).unwrap().phase;
            // The timer still takes new tasks.
            let one = ctx.construct("test/Count", "()V", &[]).unwrap();
            let once = "(Ljava/util/TimerTask;J)V";
            ctx.call_virtual(timer, "schedule", once, &[one.into(), Value::Long(0)])
                .unwrap();
            wait_for(ctx, |ctx| {
                ctx.payload::<Task>(one).unwrap().phase == Phase::Executed
            });
            let late = ctx.payload::<Task>(one).unwrap().phase;
            ctx.call_virtual(timer, "cancel", "()V", &[]).unwrap();
            (boom_phase, before, after, late)
        });
        assert_eq!(boom_phase, Phase::Cancelled);
        assert!(before >= 2 && after > before, "runs {before} then {after}");
        assert_eq!(late, Phase::Executed);
        assert!(vm.wait_finished(Some(Duration::from_secs(10))));
        let lines = lines.lock().unwrap();
        let reported = lines
            .iter()
            .filter(|l| {
                l.text == "Exception in thread \"Timer-0\" java.lang.RuntimeException: boom"
            })
            .count();
        assert_eq!(
            reported,
            1,
            "{:?}",
            lines.iter().map(|l| &l.text).collect::<Vec<_>>()
        );
    }

    /// A task that counts its runs and keeps the worst lateness it sees,
    /// as javadoc's tardiness check computes it
    /// (`System.currentTimeMillis() - scheduledExecutionTime()`), beyond
    /// how far the run time had passed the run's due time.
    ///
    /// A run that comes late because its thread woke late is late by that
    /// much on both clocks, so a busy machine adds nothing here. A
    /// scheduled time that left out a pause would add the whole pause.
    ///
    /// It also keeps the widest gap between the due times of two runs in
    /// a row, in run time. A fixed-rate task's runs are due one period
    /// apart however late its thread runs them.
    fn tardy() -> NativeClass {
        const TARDY: &str = "test/Tardy";
        NativeClass::new(TARDY)
            .extends(TASK)
            .static_field("runs", "I")
            .static_field("worst", "J")
            .static_field("last", "J")
            .static_field("gap", "J")
            .method("<init>", "()V", |ctx, a| {
                ctx.call_special(TASK, this(a), "<init>", "()V", &[])?;
                Ok(Value::Void)
            })
            .method("run", "()V", |ctx, a| {
                // The wall clock first and the run time last, so that a
                // thread descheduled between the reads can only make the
                // run seem less late than its lag.
                let now = current_time_millis();
                let at = ctx.call_virtual(this(a), "scheduledExecutionTime", "()J", &[])?;
                let t = task(ctx, this(a))?;
                let due = t.next - t.period;
                let lag = run_millis(ctx) - due;
                let late = now - at.long() - lag;
                let runs = ctx.static_field_id(TARDY, "runs", "I")?;
                let worst = ctx.static_field_id(TARDY, "worst", "J")?;
                let n = ctx.get_static(runs)?.int();
                ctx.set_static(runs, Value::Int(n + 1))?;
                let w = ctx.get_static(worst)?.long();
                ctx.set_static(worst, Value::Long(w.max(late)))?;
                let last = ctx.static_field_id(TARDY, "last", "J")?;
                let gap = ctx.static_field_id(TARDY, "gap", "J")?;
                if n > 0 {
                    let g = ctx.get_static(gap)?.long();
                    let since = due - ctx.get_static(last)?.long();
                    ctx.set_static(gap, Value::Long(g.max(since)))?;
                }
                ctx.set_static(last, Value::Long(due))?;
                Ok(Value::Void)
            })
    }

    /// The widest gap between the due times of two runs in a row of a
    /// `test/Tardy` task, in run-time milliseconds.
    fn widest_gap(vm: &Vm) -> i64 {
        vm.attach("read", |ctx| {
            let gap = ctx.static_field_id("test/Tardy", "gap", "J").unwrap();
            ctx.get_static(gap).unwrap().long()
        })
    }

    /// The runs a `test/Tardy` task made so far, and the worst lateness
    /// beyond its lag it saw.
    fn tardiness(vm: &Vm) -> (i32, i64) {
        vm.attach("read", |ctx| {
            let runs = ctx.static_field_id("test/Tardy", "runs", "I").unwrap();
            let worst = ctx.static_field_id("test/Tardy", "worst", "J").unwrap();
            let runs = ctx.get_static(runs).unwrap().int();
            (runs, ctx.get_static(worst).unwrap().long())
        })
    }

    /// Tasks are due on the run time, which stands still during a pause,
    /// so a fixed-rate task does not catch up on the runs a pause missed,
    /// and its scheduled times stay close to the wall time.
    #[test]
    fn a_pause_does_not_make_a_fixed_rate_timer_catch_up() {
        let source = Arc::new(MemorySource::new());
        let vm = Vm::new(VmConfig::default(), source, vec![tardy()]);
        let clock = vm.clock();
        let start = clock.elapsed();
        let timer = vm.attach("test", |ctx| {
            let timer = ctx.construct(TIMER, "()V", &[]).unwrap();
            let task = ctx.construct("test/Tardy", "()V", &[]).unwrap();
            let args = [task.into(), Value::Long(0), Value::Long(10)];
            let desc = "(Ljava/util/TimerTask;JJ)V";
            ctx.call_virtual(timer, "scheduleAtFixedRate", desc, &args)
                .unwrap();
            ctx.new_global(timer)
        });
        // Run n is due 10n ms of run time after the task was scheduled, at
        // `start` or later. So by the run time `t` this many runs at most
        // can have come, however late the timer's thread runs them. The
        // bounds below count from it rather than from how long the test's
        // sleeps took, which a busy machine stretches.
        let due_by = |t: Duration| {
            let runs = (t.as_millis() - start.as_millis()) / 10 + 1;
            i32::try_from(runs).unwrap_or(i32::MAX)
        };
        std::thread::sleep(Duration::from_millis(60));
        vm.pause();
        let paused_at = clock.elapsed();
        let pause_began = Instant::now();
        std::thread::sleep(Duration::from_millis(300));
        // `pause` returns at once: a timer thread behind its rate may still
        // run the runs already due, but none due after the pause.
        let (in_pause, _) = tardiness(&vm);
        assert!(
            in_pause <= due_by(paused_at),
            "no runs while paused: {in_pause} by a run time of {paused_at:?}"
        );
        let pause = pause_began.elapsed();
        let resumed_at = Instant::now();
        vm.resume();
        std::thread::sleep(Duration::from_millis(25));
        let (resumed, _) = tardiness(&vm);
        let window = resumed_at.elapsed();
        // The run time went on for at most `window` after the resume. On
        // the wall clock, the 30 runs the pause missed would come at once.
        assert!(
            resumed <= due_by(paused_at + window),
            "{resumed} runs by a run time of {paused_at:?}, then {window:?}"
        );
        // The rate goes on: wait for a few more runs, however late the
        // timer's thread gets to them.
        let deadline = Instant::now() + Duration::from_secs(10);
        let (mut later, mut worst) = (resumed, 0);
        while later - resumed < 4 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
            (later, worst) = tardiness(&vm);
        }
        assert!(
            later - resumed >= 4,
            "the rate goes on: {resumed} then {later}"
        );
        // At its rate: each run was due one period after the one before,
        // across the pause too. A resume that put the next run off, or a
        // slower rate, shows here however late the thread ran them.
        let gap = widest_gap(&vm);
        assert_eq!(gap, 10, "runs were due up to {gap} ms of run time apart");
        // Counted on the wall clock, the runs after the resume would seem
        // late by the whole pause, beyond their lag. A correct timer's
        // runs show a millisecond or two of rounding.
        let limit = i64::try_from(pause.as_millis() / 2).unwrap_or(i64::MAX);
        assert!(
            worst < limit,
            "a run seemed {worst} ms late beyond its lag after a {pause:?} pause"
        );
        vm.attach("cancel", |ctx| {
            let timer = timer.get(ctx);
            ctx.call_virtual(timer, "cancel", "()V", &[]).unwrap();
        });
        assert!(vm.wait_finished(Some(Duration::from_secs(10))));
    }

    /// A machine that may have `max_threads` threads.
    fn limited(max_threads: usize, natives: Vec<NativeClass>) -> Vm {
        let config = VmConfig {
            limits: crate::VmLimits {
                max_threads,
                ..crate::VmLimits::default()
            },
            ..VmConfig::default()
        };
        Vm::new(config, Arc::new(MemorySource::new()), natives)
    }

    /// Starts a plain `Thread`, whose `run` does nothing, and says whether
    /// it started.
    fn start_thread(ctx: &mut Ctx<'_>) -> bool {
        let thread = ctx.construct("java/lang/Thread", "()V", &[]).unwrap();
        ctx.call_virtual(thread, "start", "()V", &[]).is_ok()
    }

    /// A timer that nothing references and that has no task ends its
    /// thread once a collection finds it: the machine then has no thread
    /// left, which `wait_finished` waits for without a fixed sleep.
    #[test]
    fn a_dropped_timer_ends_its_thread_at_the_next_collection() {
        let vm = Vm::new(
            VmConfig::default(),
            Arc::new(MemorySource::new()),
            Vec::new(),
        );
        vm.attach("test", |ctx| {
            ctx.local_frame(|ctx| {
                ctx.construct(TIMER, "()V", &[]).unwrap();
            });
            ctx.gc();
        });
        let ended = vm.wait_finished(Some(Duration::from_secs(10)));
        let left = vm.thread_count();
        vm.stop();
        assert!(ended, "{left} threads left");
    }

    /// At the thread limit, `new Timer()` collects once and waits for the
    /// threads of the timers the game dropped, so dropping them never runs
    /// the game out of threads.
    #[test]
    fn dropped_timers_leave_room_for_new_ones() {
        let vm = limited(4, Vec::new());
        let made = vm.attach("test", |ctx| {
            (0..20)
                .take_while(|_| ctx.local_frame(|ctx| ctx.construct(TIMER, "()V", &[]).is_ok()))
                .count()
        });
        vm.stop();
        assert!(vm.wait_finished(Some(Duration::from_secs(10))));
        assert_eq!(made, 20);
    }

    /// A timer the game cancels gives its thread back: a start at the
    /// limit right after the cancel waits for that thread to end, where
    /// before it was refused unless the thread had ended already.
    #[test]
    fn a_start_at_the_limit_waits_for_the_thread_of_a_cancelled_timer() {
        let vm = limited(3, Vec::new());
        let (started, collections) = vm.attach("test", |ctx| {
            let cancelled = ctx.construct(TIMER, "()V", &[]).unwrap();
            let kept = ctx.construct(TIMER, "()V", &[]).unwrap();
            ctx.call_virtual(cancelled, "cancel", "()V", &[]).unwrap();
            let before = ctx.state.heap.stats().collections;
            let started = start_thread(ctx);
            let collections = ctx.state.heap.stats().collections - before;
            ctx.call_virtual(kept, "cancel", "()V", &[]).unwrap();
            (started, collections)
        });
        assert!(started, "the start was refused");
        // A thread was already ending, so nothing had to be collected.
        assert_eq!(collections, 0);
        assert!(vm.wait_finished(Some(Duration::from_secs(10))));
    }

    /// At the limit, with no timer thread that could end, a start is
    /// refused as before, and without a collection.
    #[test]
    fn a_start_at_the_limit_collects_only_when_a_timer_has_no_task() {
        let vm = limited(2, vec![count()]);
        let (started, collections) = vm.attach("test", |ctx| {
            let timer = ctx.construct(TIMER, "()V", &[]).unwrap();
            let task = ctx.construct("test/Count", "()V", &[]).unwrap();
            let once = "(Ljava/util/TimerTask;J)V";
            let hour = Value::Long(3_600_000);
            ctx.call_virtual(timer, "schedule", once, &[task.into(), hour])
                .unwrap();
            let before = ctx.state.heap.stats().collections;
            let started = start_thread(ctx);
            let collections = ctx.state.heap.stats().collections - before;
            ctx.call_virtual(timer, "cancel", "()V", &[]).unwrap();
            (started, collections)
        });
        assert!(!started, "a thread past the limit");
        assert_eq!(collections, 0);
        assert!(vm.wait_finished(Some(Duration::from_secs(10))));
    }

    /// A timer the game holds survives collections and runs what it is
    /// given afterwards.
    #[test]
    fn a_collection_keeps_a_timer_the_game_holds() {
        let source = Arc::new(MemorySource::new());
        let vm = Vm::new(VmConfig::default(), source, vec![count()]);
        let ran = vm.attach("test", |ctx| {
            let timer = ctx.construct(TIMER, "()V", &[]).unwrap();
            ctx.gc();
            ctx.gc();
            // A turn for the timer's thread with its queue empty, where a
            // collection that took the timer for dropped would end it.
            // However late the thread gets it, a kept timer still runs
            // the task below.
            ctx.sleep(Duration::from_millis(20)).unwrap();
            let task = ctx.construct("test/Count", "()V", &[]).unwrap();
            let once = "(Ljava/util/TimerTask;J)V";
            ctx.call_virtual(timer, "schedule", once, &[task.into(), Value::Long(0)])
                .unwrap();
            wait_for(ctx, |ctx| runs(ctx) >= 1);
            ctx.call_virtual(timer, "cancel", "()V", &[]).unwrap();
            runs(ctx)
        });
        assert_eq!(ran, 1);
        assert!(vm.wait_finished(Some(Duration::from_secs(10))));
    }
}
