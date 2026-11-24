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

use std::time::{Duration, Instant};

use crate::classfile::access::{PRIVATE, PROTECTED, STATIC};
use crate::ctx::{Ctx, GlobalRef, JResult, Throw, Wake};
use crate::heap::{Payload, Tracer};
use crate::lang::system::current_time_millis;
use crate::lang::throwable::report_uncaught;
use crate::lang::{this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

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
}

impl Default for Queue {
    fn default() -> Self {
        Self {
            slots: Vec::new(),
            next_order: 0,
            open: true,
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
/// waits. Returns false when the timer is cancelled and empty, which
/// ends the thread.
fn turn(ctx: &mut Ctx<'_>, queue_ref: &GlobalRef) -> JResult<bool> {
    let q_obj = queue_ref.get(ctx);
    let q = queue(ctx, q_obj)?;
    let (first, open) = (q.slots.first().map(|s| s.task), q.open);
    let Some(t_obj) = first else {
        if !open {
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
    match ctx.call_virtual(t_obj, "run", "()V", &[]) {
        Ok(_) => {}
        Err(Throw::Stop) => return Err(Throw::Stop),
        Err(Throw::Exception(e)) => {
            report_uncaught(ctx, e);
            task(ctx, t_obj)?.phase = Phase::Cancelled;
            queue(ctx, q_obj)?.remove(t_obj);
        }
    }
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

/// `new Timer()`: the queue, and the thread that runs it.
fn init(ctx: &mut Ctx<'_>, timer: ObjRef) -> JResult<Value> {
    let class = ctx.find_class(QUEUE)?;
    let q_obj = ctx.new_object(class)?;
    ctx.payload_mut::<TimerData>(timer)?.queue = Some(q_obj);
    let serial = ctx.static_field_id(TIMER, "serialNumber", "I")?;
    let n = ctx.get_static(serial)?.int();
    ctx.set_static(serial, Value::Int(n + 1))?;
    let queue_ref = ctx.new_global(q_obj);
    ctx.spawn_thread(&format!("Timer-{n}"), move |ctx| {
        while ctx.local_frame(|ctx| turn(ctx, &queue_ref))? {}
        Ok(())
    })?;
    Ok(Value::Void)
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
            ctx.sleep(Duration::from_millis(100)).unwrap();
            let before = runs(ctx);
            ctx.sleep(Duration::from_millis(100)).unwrap();
            let after = runs(ctx);
            let boom_phase = ctx.payload::<Task>(boom).unwrap().phase;
            // The timer still takes new tasks.
            let one = ctx.construct("test/Count", "()V", &[]).unwrap();
            let once = "(Ljava/util/TimerTask;J)V";
            ctx.call_virtual(timer, "schedule", once, &[one.into(), Value::Long(0)])
                .unwrap();
            ctx.sleep(Duration::from_millis(50)).unwrap();
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

    /// A task that counts its runs and keeps the worst lateness it sees
    /// as javadoc's tardiness check computes it:
    /// `System.currentTimeMillis() - scheduledExecutionTime()`.
    fn tardy() -> NativeClass {
        const TARDY: &str = "test/Tardy";
        NativeClass::new(TARDY)
            .extends(TASK)
            .static_field("runs", "I")
            .static_field("worst", "J")
            .method("<init>", "()V", |ctx, a| {
                ctx.call_special(TASK, this(a), "<init>", "()V", &[])?;
                Ok(Value::Void)
            })
            .method("run", "()V", |ctx, a| {
                let at = ctx.call_virtual(this(a), "scheduledExecutionTime", "()J", &[])?;
                let late = current_time_millis() - at.long();
                let runs = ctx.static_field_id(TARDY, "runs", "I")?;
                let worst = ctx.static_field_id(TARDY, "worst", "J")?;
                let n = ctx.get_static(runs)?.int();
                ctx.set_static(runs, Value::Int(n + 1))?;
                let w = ctx.get_static(worst)?.long();
                ctx.set_static(worst, Value::Long(w.max(late)))?;
                Ok(Value::Void)
            })
    }

    /// The runs and the worst lateness a `test/Tardy` task saw so far.
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
        let timer = vm.attach("test", |ctx| {
            let timer = ctx.construct(TIMER, "()V", &[]).unwrap();
            let task = ctx.construct("test/Tardy", "()V", &[]).unwrap();
            let args = [task.into(), Value::Long(0), Value::Long(10)];
            let desc = "(Ljava/util/TimerTask;JJ)V";
            ctx.call_virtual(timer, "scheduleAtFixedRate", desc, &args)
                .unwrap();
            ctx.new_global(timer)
        });
        std::thread::sleep(Duration::from_millis(60));
        vm.pause();
        std::thread::sleep(Duration::from_millis(20));
        let (paused_at, _) = tardiness(&vm);
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!(tardiness(&vm).0, paused_at, "no runs while paused");
        let resumed_at = Instant::now();
        vm.resume();
        std::thread::sleep(Duration::from_millis(25));
        let (resumed, worst) = tardiness(&vm);
        let window = resumed_at.elapsed();
        // On the wall clock, the 30 runs the pause missed would come at once,
        // each some 300 ms late. Allow the runs the window holds at one per
        // 10 ms, plus a few: a sleep can last far longer than asked on a
        // busy machine (CI's macOS runners).
        let allowed = i32::try_from(window.as_millis() / 10).unwrap_or(i32::MAX) + 4;
        assert!(
            resumed - paused_at <= allowed,
            "{paused_at} then {resumed} in {window:?}"
        );
        assert!(worst < 100, "a run seemed {worst} ms late");
        std::thread::sleep(Duration::from_millis(100));
        let (later, _) = tardiness(&vm);
        assert!(
            later - resumed >= 4,
            "the rate goes on: {resumed} then {later}"
        );
        vm.attach("cancel", |ctx| {
            let timer = timer.get(ctx);
            ctx.call_virtual(timer, "cancel", "()V", &[]).unwrap();
        });
        assert!(vm.wait_finished(Some(Duration::from_secs(10))));
    }
}
