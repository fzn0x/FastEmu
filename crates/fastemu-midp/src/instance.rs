//! [`MidpInstance`]: one running MIDlet, as the app sees it.
//!
//! Every method of [`Instance`] returns at once and never waits for the
//! VM lock, which a game thread may hold for a long time: the apps call
//! them on their UI thread for every game. Input, pause, resume and stop
//! go to the instance's control thread, a host thread that owns the `Vm`
//! and runs each in the order the app sent them:
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
//! [`MidpInstance::ping`] and [`MidpInstance::activity`] are not part of
//! [`Instance`]: they wait for the VM lock, so they are for tools such as
//! `cargo xtask compat`, not for an app's UI thread.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::Duration;

use fastemu_audio::Group;
use fastemu_core::{Frame, InputEvent, Instance, InstanceId, InstanceStatus, LogLevel, LogLine};
use fastemu_jvm::{Activity, Ctx, HeapStats, JResult, Vm};

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

    /// Ends every thread of the VM and the instance's sound.
    ///
    /// The VM is told to stop first, before anything waits for its lock:
    /// a thread wedged in a native that loops around calls into Java holds
    /// the lock for ever, but ends at its next call. Each wait for the
    /// lock after that gives up in time.
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
    vm: Vm,
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
    /// Starts the control thread and the first lifecycle thread, which
    /// constructs `class_name` and calls `startApp`. When either cannot
    /// start, the VM stops, so its event thread does not live on.
    pub(crate) fn start(
        vm: Vm,
        host: HostLink,
        group: Option<Group>,
        title: String,
        class_name: String,
    ) -> std::io::Result<Arc<Self>> {
        let queue = ControlQueue::default();
        host.set_control(queue.clone());
        let controller = Controller {
            vm: vm.clone(),
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
            vm.stop();
            return Err(e);
        }
        if let Err(e) = vm.spawn("MIDlet", move |ctx| midlet::launch(ctx, &class_name)) {
            // The control thread owns a clone of the VM: let it end both.
            queue.send(Control::Stop);
            return Err(e);
        }
        Ok(Arc::new(Self {
            id: host.id(),
            title,
            host,
            vm,
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

    /// What the instance's heap holds now.
    pub fn heap_stats(&self) -> HeapStats {
        self.vm.heap_stats()
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
/// wait for the VM lock.
impl MidpInstance {
    /// Asks the event thread whether it would take a key now: posts a task
    /// that answers when the event thread runs it between callbacks and
    /// gets the callback lock within a second. An instance the app paused
    /// does not answer, nor does one whose event thread is stuck in a
    /// callback.
    /// Waits for the VM lock for a second at most, as [`Vm::post`] does;
    /// a task it cannot post in that time never answers.
    pub fn ping(&self) -> Ping {
        let answer = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let ping = Ping(Arc::clone(&answer));
        self.vm.post(move |ctx| {
            if lcdui::event::acquire(ctx, Some(PING_LOCK_WAIT))? {
                lcdui::event::release(ctx);
                answer.store(true, Ordering::Release);
            }
            Ok(())
        });
        ping
    }

    /// What the game's threads are doing now; see [`Vm::activity`]. Waits
    /// for the VM lock for 100 ms at most, then returns `None`.
    pub fn activity(&self) -> Option<Activity> {
        self.vm.activity()
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
    /// once, however long a game thread holds the VM lock.
    fn send(&self, event: InputEvent) {
        if self.vm.is_stopping() {
            return;
        }
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
