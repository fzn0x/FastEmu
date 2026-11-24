//! How closely sleeps and timed waits keep their time, and how quickly an
//! interrupt or a stop ends them.
//!
//! A game loop that sleeps 10 ms a frame runs at the rate its sleeps
//! allow, so a sleep that wakes on a coarse system tick (15.6 ms on
//! Windows) caps it far below what the phone ran. Run with `--nocapture`
//! to see the measurements.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fastemu_jvm::{JResult, MemorySource, Value, Vm, VmConfig, Wake};

/// How long each timed sleep or wait asks for.
const ASKED: Duration = Duration::from_millis(10);
/// How many times each is measured.
const ROUNDS: usize = 30;
/// The median lateness allowed. A wait that rounds up to Windows'
/// default tick is about 5.6 ms late.
const MEDIAN_SLACK: Duration = Duration::from_micros(2500);

fn vm() -> Vm {
    Vm::new(
        VmConfig::default(),
        Arc::new(MemorySource::new()),
        Vec::new(),
    )
}

/// Runs `round` `ROUNDS` times on a Java thread and returns how long
/// each took.
fn measure(
    round: impl Fn(&mut fastemu_jvm::Ctx<'_>) -> JResult<()> + Send + 'static,
) -> Vec<Duration> {
    let vm = vm();
    let times: Arc<Mutex<Vec<Duration>>> = Arc::default();
    let out = Arc::clone(&times);
    vm.spawn("measure", move |ctx| {
        // One round first, so thread start-up costs are not counted.
        round(ctx)?;
        for _ in 0..ROUNDS {
            let start = Instant::now();
            round(ctx)?;
            out.lock().unwrap().push(start.elapsed());
        }
        Ok(())
    })
    .unwrap();
    assert!(vm.wait_finished(Some(Duration::from_secs(30))));
    let mut times = times.lock().unwrap().clone();
    assert_eq!(times.len(), ROUNDS, "every round ran");
    times.sort();
    times
}

/// Checks that no round ended early and that the median one ended
/// within [`MEDIAN_SLACK`] of what it asked for.
fn check(what: &str, times: &[Duration]) {
    let median = times[times.len() / 2];
    let mean = times.iter().sum::<Duration>() / times.len() as u32;
    println!(
        "{what}: asked {ASKED:?}, min {:?}, median {median:?}, mean {mean:?}, max {:?}",
        times[0],
        times[times.len() - 1]
    );
    assert!(times[0] >= ASKED, "{what} ended early: {:?}", times[0]);
    assert!(
        median <= ASKED + MEDIAN_SLACK,
        "{what} wakes late: median {median:?} for {ASKED:?}"
    );
}

#[test]
fn sleeps_wake_on_time() {
    let times = measure(|ctx| ctx.sleep(ASKED));
    check("Ctx::sleep", &times);
}

#[test]
fn java_sleeps_wake_on_time() {
    let millis = ASKED.as_millis() as i64;
    let times = measure(move |ctx| {
        ctx.call_static("java/lang/Thread", "sleep", "(J)V", &[Value::Long(millis)])?;
        Ok(())
    });
    check("Thread.sleep", &times);
}

#[test]
fn timed_object_waits_wake_on_time() {
    let times = measure(|ctx| {
        let class = ctx.find_class("java/lang/Object")?;
        let lock = ctx.new_object(class)?;
        ctx.monitor_enter(lock)?;
        let waited = ctx.monitor_wait(lock, Some(Instant::now() + ASKED));
        ctx.monitor_exit(lock)?;
        waited
    });
    check("Object.wait", &times);
}

#[test]
fn native_waits_wake_on_time() {
    let times = measure(|ctx| {
        let deadline = Instant::now() + ASKED;
        while ctx.wait_until(Some(deadline))? != Wake::TimedOut {}
        Ok(())
    });
    check("Ctx::wait_until", &times);
}

/// How long `vm`'s only thread takes to end after `end` runs.
fn time_to_end(vm: &Vm, end: impl FnOnce(&Vm)) -> Duration {
    // Let the thread get into its sleep.
    std::thread::sleep(Duration::from_millis(50));
    let start = Instant::now();
    end(vm);
    assert!(vm.wait_finished(Some(Duration::from_secs(5))), "it ended");
    start.elapsed()
}

#[test]
fn an_interrupt_ends_a_sleep_at_once() {
    let vm = vm();
    let thread = vm.attach("main", |ctx| {
        let thread = ctx
            .spawn_thread("sleeper", |ctx| match ctx.sleep(Duration::from_secs(30)) {
                Err(fastemu_jvm::Throw::Exception(_)) => Ok(()),
                other => panic!("the sleep ended with {other:?}"),
            })
            .unwrap();
        ctx.new_global(thread)
    });
    let took = time_to_end(&vm, |vm| {
        vm.attach("interrupt", |ctx| {
            let t = thread.get(ctx);
            ctx.call_virtual(t, "interrupt", "()V", &[]).unwrap();
        });
    });
    println!("interrupt to end of sleep: {took:?}");
    assert!(took < Duration::from_millis(500), "took {took:?}");
}

#[test]
fn a_stop_ends_sleeps_and_timed_waits_at_once() {
    let vm = vm();
    vm.spawn("sleeper", |ctx| ctx.sleep(Duration::from_secs(30)))
        .unwrap();
    vm.spawn("waiter", |ctx| {
        let class = ctx.find_class("java/lang/Object")?;
        let lock = ctx.new_object(class)?;
        ctx.monitor_enter(lock)?;
        ctx.monitor_wait(lock, Some(Instant::now() + Duration::from_secs(30)))
    })
    .unwrap();
    let took = time_to_end(&vm, Vm::stop);
    println!("stop to end of every thread: {took:?}");
    assert!(took < Duration::from_millis(500), "took {took:?}");
}
