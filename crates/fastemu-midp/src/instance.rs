//! [`MidpInstance`]: one running MIDlet, as the app sees it.
//!
//! Every method of [`Instance`] returns at once and never waits for the
//! VM lock, which a game thread may hold for a long time: the apps call
//! them on their UI thread for every game. Input, pause, resume and stop
//! go to the instance's control thread, a host thread that holds the
//! host's only `Vm` and runs each in the order the app sent them:
//!
//! - input: posted to the VM for the event thread once the control thread
//!   is done with what came before it, such as a pause still under way
//!   (which waits up to a second each for `hideNotify` and `pauseApp`).
//!   Input sent after a pause, until the resume, or after a stop is
//!   dropped;
//! - pause: `hideNotify` on the event thread, `pauseApp` on a lifecycle
//!   thread once the lifecycle calls before it have returned (each waited
//!   for up to a second), then the VM and the sound group pause. A MIDlet
//!   that paused itself (`notifyPaused`) gets no `pauseApp`, but freezes
//!   all the same;
//! - resume: the VM and the sound group resume, `showNotify` and a full
//!   repaint, then `startApp` on a lifecycle thread, in its turn. The
//!   MIDlet's `resumeRequest` does the same for a MIDlet that paused
//!   itself, and nothing while the app has it paused;
//! - stop: `destroyApp(true)` on a lifecycle thread, waited for up to a
//!   second, then every thread of the VM stops and the sound group closes.
//!
//! The MIDlet's own messages (`resumeRequest`, `notifyDestroyed`, a crash)
//! share the control thread's queue with the app's, merged so that a game
//! that sends them in a loop cannot grow it (`crate::host::ControlQueue`).
//! The app's messages are never merged or dropped there.
//!
//! A game thread can hold the VM lock for ever, in a native that loops
//! around calls into Java or in a loop with no safepoint. The control
//! thread does not wait for the lock longer than a second at a time: a
//! lifecycle call or input that cannot start in that time is dropped, and
//! a stop sets the VM's stop flag as soon as `destroyApp` has had its
//! second, before it waits for the lock again.
//!
//! `notifyDestroyed` ends the instance the same way without `destroyApp`.
//! A crash (the MIDlet class missing, its constructor or `startApp`
//! throwing, an internal error of the VM) calls `destroyApp(true)` if
//! there is a MIDlet, then ends it as [`InstanceStatus::Crashed`].
//!
//! The [`MidpInstance`] the app holds keeps only a [`WeakVm`], so the
//! machine of a game that has ended is freed on the control thread, or on
//! the last of its own threads to end, while the app still holds the
//! instance. What a tile shows (the last frame, the status, the log, a
//! link the game asked for) lives in the [`HostLink`], apart from the
//! machine.
//!
//! [`MidpInstance::ping`] and [`MidpInstance::activity`] are not part of
//! [`Instance`]: they wait for the VM lock and hold the machine while they
//! run, so they are for tools such as `cargo xtask compat`, not for an
//! app's UI thread.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use fastemu_audio::Group;
use fastemu_core::{Frame, InputEvent, Instance, InstanceId, InstanceStatus, LogLevel, LogLine};
use fastemu_jvm::{Activity, Ctx, HeapStats, JResult, Vm, WeakVm};

use crate::host::{Control, ControlQueue, HostLink};
use crate::lcdui;
use crate::midlet::{self, Lifecycle};

/// How long the control thread waits for `pauseApp`, `destroyApp` and
/// `hideNotify` before it carries on without them.
const LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(1);

/// How long a stopping instance waits for its threads to unwind.
const STOP_TIMEOUT: Duration = Duration::from_secs(2);

/// How many lifecycle threads may be alive at once, waiting for their turn
/// or running. A `startApp` that never returns (a game that runs its loop
/// in it) keeps every call after it waiting, and each waiter is a Java
/// thread, so a game that pauses and resumes again and again would fill
/// the machine's threads with them. The threads are the host's, and the
/// machine counts them against its limit; this bound keeps them from using
/// the room that `destroyApp`, the event thread and the media thread need.
/// `destroyApp` is not held to it, since it runs at once.
const MAX_LIFECYCLE_THREADS: usize = 8;

/// A flag a Java thread raises when it finishes, however it finishes.
#[derive(Default)]
struct Done {
    finished: Mutex<bool>,
    cond: Condvar,
}

impl Done {
    /// Waits for the flag; returns whether it rose in time.
    fn wait(&self, timeout: Duration) -> bool {
        let guard = self.finished.lock().unwrap_or_else(PoisonError::into_inner);
        let (guard, _) = self
            .cond
            .wait_timeout_while(guard, timeout, |finished| !*finished)
            .unwrap_or_else(PoisonError::into_inner);
        *guard
    }
}

/// Raises its [`Done`] when dropped, so a thread that ends by an exception,
/// a stop or a panic still signals.
struct RaiseOnDrop(Arc<Done>);

impl Drop for RaiseOnDrop {
    fn drop(&mut self) {
        *self
            .0
            .finished
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = true;
        self.0.cond.notify_all();
    }
}

/// Counts a lifecycle thread for as long as the thread, or the closure it
/// was to run, lives.
struct Alive(Arc<AtomicUsize>);

impl Alive {
    fn new(count: &Arc<AtomicUsize>) -> Self {
        count.fetch_add(1, Ordering::Relaxed);
        Self(Arc::clone(count))
    }
}

impl Drop for Alive {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Starts a lifecycle thread running `which` as call `turn` (see
/// [`midlet::lifecycle`]) and waits up to `timeout` for it. Returns
/// whether the thread started: it did not when the machine has no room for
/// another thread or is wedged. `alive` counts the instance's lifecycle
/// threads.
fn run_lifecycle(
    vm: &Vm,
    which: Lifecycle,
    turn: u64,
    timeout: Duration,
    alive: &Arc<AtomicUsize>,
) -> bool {
    let done = Arc::new(Done::default());
    let raise = RaiseOnDrop(Arc::clone(&done));
    let counted = Alive::new(alive);
    let spawned = vm.spawn("MIDlet", move |ctx| {
        let _raise = raise;
        let _counted = counted;
        midlet::lifecycle(ctx, which, turn)
    });
    if spawned.is_err() {
        return false;
    }
    done.wait(timeout);
    true
}

/// Runs `f` on the event thread and waits up to `timeout` for it.
fn on_event_thread(
    vm: &Vm,
    timeout: Duration,
    f: impl FnOnce(&mut Ctx<'_>) -> JResult<()> + Send + 'static,
) -> bool {
    let done = Arc::new(Done::default());
    let raise = RaiseOnDrop(Arc::clone(&done));
    vm.post(move |ctx| {
        let _raise = raise;
        f(ctx)
    });
    done.wait(timeout)
}

/// The control thread: applies the app's and the MIDlet's lifecycle
/// requests one at a time.
struct Controller {
    /// The host's only strong handle on the machine (see
    /// [`Controller::finish`]).
    vm: Vm,
    host: HostLink,
    group: Option<Group>,
    queue: ControlQueue,
    /// A message taken from the queue while gathering input, handled
    /// next.
    held: Option<Control>,
    /// Whether the VM is paused.
    frozen: bool,
    /// The turn the next lifecycle call takes; the launch took 0.
    next_turn: u64,
    /// The lifecycle threads alive now (see [`MAX_LIFECYCLE_THREADS`]).
    alive: Arc<AtomicUsize>,
    /// Whether a lifecycle call was dropped, and the log says so.
    dropped: bool,
}

impl Controller {
    fn run(mut self) {
        loop {
            let message = match self.held.take() {
                Some(message) => message,
                None => self.queue.recv(),
            };
            match message {
                Control::Input(event) => self.input(event),
                Control::Pause => self.pause(),
                Control::Resume => self.resume(),
                // The game's request resumes only a pause it made itself;
                // one the app made waits for the app's resume.
                Control::ResumeRequest if !self.frozen => self.resume(),
                Control::ResumeRequest => {}
                Control::Stop => {
                    self.destroy();
                    return self.finish(InstanceStatus::Exited);
                }
                Control::Exited => return self.finish(InstanceStatus::Exited),
                Control::Crashed(message) => {
                    self.destroy();
                    return self.finish(InstanceStatus::Crashed { message });
                }
            }
        }
    }

    /// Posts `first`, and the input queued right behind it, to the event
    /// thread as one task, so a backlog takes the VM lock once. Input that
    /// comes while the app has the instance paused, or once it is
    /// stopping, is dropped.
    fn input(&mut self, first: InputEvent) {
        let mut events = vec![first];
        while let Some(message) = self.queue.try_recv() {
            if let Control::Input(event) = message {
                events.push(event);
            } else {
                self.held = Some(message);
                break;
            }
        }
        if self.frozen || self.vm.is_stopping() {
            return;
        }
        self.vm.post(move |ctx| {
            for event in events {
                lcdui::input::queue(ctx, event);
            }
            Ok(())
        });
    }

    /// Pauses a running MIDlet. One that paused itself (`notifyPaused`, or
    /// a `startApp` that asked to stay paused) freezes the same way but
    /// gets no `pauseApp`, which MIDP calls only on an active MIDlet.
    fn pause(&mut self) {
        if self.frozen {
            return;
        }
        let running = match self.host.peek_status() {
            InstanceStatus::Running => true,
            InstanceStatus::Paused => false,
            _ => return,
        };
        on_event_thread(&self.vm, LIFECYCLE_TIMEOUT, |ctx| {
            lcdui::event::set_visible(ctx, false)
        });
        if running {
            self.call(Lifecycle::Pause, LIFECYCLE_TIMEOUT);
        }
        // A startApp still on its way sees the machine paused before this
        // sets Paused, and waits for the resume (`midlet::start_app`).
        self.vm.pause();
        if let Some(group) = &self.group {
            group.set_paused(true);
        }
        self.frozen = true;
        self.host.set_status(InstanceStatus::Paused);
    }

    fn resume(&mut self) {
        if self.host.peek_status() != InstanceStatus::Paused {
            return;
        }
        self.thaw();
        // Running from here, so a pause that follows at once is not lost
        // while startApp is still on its way.
        self.host.set_status(InstanceStatus::Running);
        self.vm.post(|ctx| lcdui::event::set_visible(ctx, true));
        self.call(Lifecycle::Resume, Duration::ZERO);
    }

    /// Starts `startApp` or `pauseApp` in its turn, after the calls before
    /// it, and waits up to `timeout` for it.
    fn call(&mut self, which: Lifecycle, timeout: Duration) {
        if self.alive.load(Ordering::Relaxed) >= MAX_LIFECYCLE_THREADS {
            if !std::mem::replace(&mut self.dropped, true) {
                self.host.log(LogLine {
                    level: LogLevel::Warn,
                    text: "too many lifecycle calls are waiting; later ones are dropped".to_owned(),
                });
            }
            return;
        }
        if run_lifecycle(&self.vm, which, self.next_turn, timeout, &self.alive) {
            self.next_turn += 1;
        }
    }

    fn thaw(&mut self) {
        if self.frozen {
            self.vm.resume();
            if let Some(group) = &self.group {
                group.set_paused(false);
            }
            self.frozen = false;
        }
    }

    /// Gives the MIDlet its `destroyApp(true)`.
    fn destroy(&mut self) {
        if self.vm.is_stopping() {
            return;
        }
        self.thaw();
        // destroyApp takes no turn: it runs even beside a startApp that
        // never returns.
        run_lifecycle(
            &self.vm,
            Lifecycle::Destroy,
            0,
            LIFECYCLE_TIMEOUT,
            &self.alive,
        );
    }

    /// Ends every thread of the VM and the instance's sound, then lets go
    /// of the VM.
    ///
    /// The VM is told to stop first, before anything waits for its lock:
    /// a thread wedged in a native that loops around calls into Java holds
    /// the lock for ever, but ends at its next call. Each wait for the
    /// lock after that gives up in time.
    ///
    /// The controller's `Vm` is the only one the host holds (the
    /// [`MidpInstance`] keeps a [`WeakVm`]), so the machine is freed when
    /// this returns, on this thread, or later on the last of its own
    /// threads to end. The status is set first, so the app sees the game
    /// ended before the machine goes. The app keeps what its tile shows in
    /// the [`HostLink`], and never frees a machine on its own thread.
    fn finish(self, status: InstanceStatus) {
        self.vm.stop();
        self.vm.resume();
        self.vm.flush_output();
        self.vm.wait_finished(Some(STOP_TIMEOUT));
        if let Some(group) = &self.group {
            group.close();
        }
        self.host.set_status(status);
    }
}

impl Drop for Controller {
    /// Messages that still wait when the control thread ends, by a return
    /// or a panic, are dropped, and so are those that come later, as they
    /// were with a channel whose receiver is gone.
    fn drop(&mut self) {
        self.queue.close();
    }
}

/// One running MIDlet in its own virtual machine.
pub struct MidpInstance {
    id: InstanceId,
    title: String,
    host: HostLink,
    /// The machine, without keeping it alive: the control thread and the
    /// machine's own threads hold it, so a game that ends frees it once
    /// its threads have ended, while its tile still shows the last frame
    /// and the log.
    vm: WeakVm,
    group: Option<Group>,
    /// The control thread's queue. Sending on it never blocks.
    control: ControlQueue,
}

impl std::fmt::Debug for MidpInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidpInstance")
            .field("id", &self.id)
            .field("title", &self.title)
            .finish_non_exhaustive()
    }
}

impl MidpInstance {
    /// Starts the first lifecycle thread, which constructs `class_name`
    /// and calls `startApp`, and then the control thread, which takes
    /// `vm`: the host's only strong handle, so the machine is freed when
    /// the game ends (see [`Controller::finish`]). What the MIDlet sends
    /// the control thread before it runs waits in its queue, which exists
    /// before either thread. When either thread cannot start, the VM
    /// stops, so its event thread does not live on; when it is the control
    /// thread, the MIDlet's constructor may have run already.
    pub(crate) fn start(
        vm: Vm,
        host: HostLink,
        group: Option<Group>,
        title: String,
        class_name: String,
    ) -> std::io::Result<Arc<Self>> {
        let queue = ControlQueue::default();
        host.set_control(queue.clone());
        let weak = vm.downgrade();
        if let Err(e) = vm.spawn("MIDlet", move |ctx| midlet::launch(ctx, &class_name)) {
            vm.stop();
            return Err(e);
        }
        let controller = Controller {
            vm,
            host: host.clone(),
            group: group.clone(),
            queue: queue.clone(),
            held: None,
            frozen: false,
            next_turn: 1,
            alive: Arc::default(),
            dropped: false,
        };
        let control = std::thread::Builder::new()
            .name(format!("instance {} control", host.id()))
            .spawn(move || controller.run());
        if let Err(e) = control {
            // The controller went with the thread that did not start; the
            // machine lives on in its own threads until they stop.
            if let Some(vm) = weak.upgrade() {
                vm.stop();
            }
            return Err(e);
        }
        Ok(Arc::new(Self {
            id: host.id(),
            title,
            host,
            vm: weak,
            group,
            control: queue,
        }))
    }

    fn send_control(&self, message: Control) {
        self.control.send(message);
    }

    /// Exceptions the game let escape and internal errors since launch.
    pub fn error_count(&self) -> u64 {
        self.host.error_count()
    }

    /// The instance's link to the app, for tests and tools that read the
    /// frame and log without the [`Instance`] trait.
    pub fn host(&self) -> &HostLink {
        &self.host
    }

    /// What the instance's heap holds now, or `None` once the game has
    /// ended and its machine is freed. It waits for the VM lock, and it
    /// holds the machine while it reads: if the game ends meanwhile, the
    /// caller's thread frees it. The apps do not call it.
    pub fn heap_stats(&self) -> Option<HeapStats> {
        self.vm.upgrade().map(|vm| vm.heap_stats())
    }

    /// Whether the game's machine is freed: its heap, its classes, its JAR
    /// and what its natives keep, such as its players. That happens when
    /// the game ends and its threads have ended, without waiting for the
    /// app to let go of the instance; a thread stuck in a call that cannot
    /// be stopped holds it until the call returns. It takes no lock and
    /// never holds the machine.
    pub fn vm_freed(&self) -> bool {
        self.vm.is_freed()
    }
}

/// The answer to [`MidpInstance::ping`]. Only `ping` makes one.
#[derive(Clone, Debug)]
pub struct Ping(Arc<std::sync::atomic::AtomicBool>);

impl Ping {
    /// Whether the event thread took the question between two callbacks
    /// and got the callback lock, as it does to deliver a key.
    pub fn answered(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

/// How long the event thread waits for the callback lock to answer a ping.
const PING_LOCK_WAIT: Duration = Duration::from_secs(1);

/// For tools that tell a game waiting on a still screen from a stuck one,
/// such as `cargo xtask compat`. Unlike the methods of [`Instance`], these
/// wait for the VM lock, and they hold the machine while they run, as
/// [`MidpInstance::heap_stats`] does: if the game ends meanwhile, the
/// caller's thread frees it.
impl MidpInstance {
    /// Asks the event thread whether it would take a key now: posts a task
    /// that answers when the event thread runs it between callbacks and
    /// gets the callback lock within a second. An instance the app paused
    /// does not answer, nor does one whose event thread is stuck in a
    /// callback, nor one whose machine is freed.
    /// Waits for the VM lock for a second at most, as [`Vm::post`] does;
    /// a task it cannot post in that time never answers.
    pub fn ping(&self) -> Ping {
        let answer = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let ping = Ping(Arc::clone(&answer));
        if let Some(vm) = self.vm.upgrade() {
            vm.post(move |ctx| {
                if lcdui::event::acquire(ctx, Some(PING_LOCK_WAIT))? {
                    lcdui::event::release(ctx);
                    answer.store(true, Ordering::Release);
                }
                Ok(())
            });
        }
        ping
    }

    /// What the game's threads are doing now; see [`Vm::activity`]. Waits
    /// for the VM lock for 100 ms at most, then returns `None`. It returns
    /// `None` too once the machine is freed ([`MidpInstance::vm_freed`]).
    pub fn activity(&self) -> Option<Activity> {
        self.vm.upgrade().and_then(|vm| vm.activity())
    }
}

impl Instance for MidpInstance {
    fn id(&self) -> InstanceId {
        self.id
    }

    fn title(&self) -> String {
        self.title.clone()
    }

    fn status(&self) -> InstanceStatus {
        self.host.status()
    }

    fn frame(&self) -> Option<Frame> {
        self.host.frame()
    }

    /// Hands the event to the control thread, which posts it to the
    /// event thread in its turn among pause, resume and stop. Returns at
    /// once, however long a game thread holds the VM lock. Input sent
    /// after a stop, or after the game ended itself, is dropped: the
    /// control thread takes no message after the one that ends the game,
    /// and closes its queue as it ends.
    fn send(&self, event: InputEvent) {
        self.send_control(Control::Input(event));
    }

    fn pause(&self) {
        self.send_control(Control::Pause);
    }

    fn resume(&self) {
        self.send_control(Control::Resume);
    }

    fn stop(&self) {
        self.send_control(Control::Stop);
    }

    fn set_volume(&self, volume: f32) {
        if let Some(group) = &self.group {
            group.set_volume(volume.clamp(0.0, 1.0));
        }
    }

    fn drain_log(&self) -> Vec<LogLine> {
        self.host.drain_log()
    }

    /// True while a `TextBox` is shown or a `TextField` or `DateField`
    /// has the focus, until the instance stops.
    fn wants_text(&self) -> bool {
        !self.host.peek_status().is_finished() && self.host.wants_text()
    }

    /// The latest web link `platformRequest` asked for, once. A link asked
    /// for just before the game ended is still handed over, as MIDP has the
    /// phone handle a request after its suite exits.
    fn take_link_request(&self) -> Option<String> {
        self.host.take_link()
    }
}

impl Drop for MidpInstance {
    /// An instance the app lets go of stops: the VM's own references to
    /// the control queue would otherwise keep it running.
    fn drop(&mut self) {
        self.send_control(Control::Stop);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;
    use std::time::Instant;

    use fastemu_jvm::{MemorySource, VmConfig};

    use super::*;
    use crate::env::InstanceEnv;

    /// How long a test waits for what takes milliseconds: generous, for a
    /// loaded CI machine.
    const DEADLINE: Duration = Duration::from_secs(30);

    /// Sends the name of the thread that drops it: put in a machine as an
    /// extension, it tells when and where the machine is freed.
    struct Witness(mpsc::Sender<Option<String>>);

    impl Drop for Witness {
        fn drop(&mut self) {
            let _ = self
                .0
                .send(std::thread::current().name().map(str::to_owned));
        }
    }

    /// An instance whose MIDlet class does not exist, so it crashes at
    /// once, on a machine that holds a [`Witness`], with `group` as its
    /// sound; and the receiver of the witness.
    fn crashing(group: Option<Group>) -> (Arc<MidpInstance>, mpsc::Receiver<Option<String>>) {
        let mut env = InstanceEnv::for_tests("unused");
        env.audio = group.clone();
        let host = env.host.clone();
        let config = VmConfig {
            log: host.log_sink(),
            ..VmConfig::default()
        };
        let classes = crate::classes(&fastemu_core::default_profile());
        let vm = Vm::new(config, Arc::new(MemorySource::new()), classes);
        let (tx, rx) = mpsc::channel();
        vm.attach("test", move |ctx| {
            ctx.set_extension(env);
            ctx.set_extension(Witness(tx));
        });
        let instance = MidpInstance::start(vm, host, group, "Gone".into(), "no.such.Game".into())
            .expect("starting the instance");
        (instance, rx)
    }

    /// Waits until `done` holds, failing after [`DEADLINE`].
    fn wait_until(what: &str, done: impl Fn() -> bool) {
        let start = Instant::now();
        while !done() {
            assert!(
                start.elapsed() < DEADLINE,
                "{what}: not within {DEADLINE:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn a_finished_game_frees_its_machine_while_the_app_holds_it() {
        let (instance, freed) = crashing(None);
        wait_until("the instance ends", || instance.status().is_finished());
        let on = freed
            .recv_timeout(DEADLINE)
            .expect("the machine is freed while the app still holds the instance");
        let on = on.unwrap_or_default();
        assert!(
            on.starts_with("instance ") || on.starts_with("java "),
            "freed on {on:?}, not on the instance's own threads"
        );
        // What the tile shows stays.
        match instance.status() {
            InstanceStatus::Crashed { message } => {
                assert!(message.contains("no.such.Game"), "{message}");
            }
            other => panic!("expected a crash, got {other:?}"),
        }
        assert!(
            instance
                .drain_log()
                .iter()
                .any(|l| l.text.contains("no.such.Game")),
            "the log stays"
        );
        assert_eq!(instance.error_count(), 1);
        assert!(instance.vm_freed());
        assert!(instance.heap_stats().is_none());
        assert!(instance.activity().is_none());
        assert!(!instance.ping().answered());
        // The app's calls on a finished game still return, and change
        // nothing.
        instance.send(InputEvent::Text("x".into()));
        instance.pause();
        instance.resume();
        instance.set_volume(0.5);
        instance.stop();
        assert!(!instance.wants_text());
        assert_eq!(instance.take_link_request(), None);
        assert!(instance.status().is_finished());
        assert!(instance.vm_freed());
    }
}
