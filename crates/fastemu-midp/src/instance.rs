//! [`MidpInstance`]: one running MIDlet, as the app sees it.
//!
//! Every method of [`Instance`] returns at once. Input is posted to the
//! VM for the event thread. Pause, resume and stop go to the instance's
//! control thread, a host thread that owns the `Vm` and runs each change
//! in order:
//!
//! - pause: `hideNotify` on the event thread, `pauseApp` on a lifecycle
//!   thread once the lifecycle calls before it have returned (each waited
//!   for up to a second), then the VM and the sound group pause;
//! - resume: the VM and the sound group resume, `showNotify` and a full
//!   repaint, then `startApp` on a lifecycle thread;
//! - stop: `destroyApp(true)` on a lifecycle thread, waited for up to a
//!   second, then every thread of the VM stops and the sound group closes.
//!
//! `notifyDestroyed` ends the instance the same way without `destroyApp`.
//! A crash (the MIDlet class missing, its constructor or `startApp`
//! throwing, an internal error of the VM) calls `destroyApp(true)` if
//! there is a MIDlet, then ends it as [`InstanceStatus::Crashed`].

use std::sync::{Arc, Condvar, Mutex, PoisonError, mpsc};
use std::time::Duration;

use fastemu_audio::Group;
use fastemu_core::{Frame, InputEvent, Instance, InstanceId, InstanceStatus, LogLine};
use fastemu_jvm::{Ctx, HeapStats, JResult, Vm};

use crate::host::{Control, HostLink};
use crate::lcdui;
use crate::midlet::{self, Lifecycle};

/// How long the control thread waits for `pauseApp`, `destroyApp` and
/// `hideNotify` before it carries on without them.
const LIFECYCLE_TIMEOUT: Duration = Duration::from_secs(1);

/// How long a stopping instance waits for its threads to unwind.
const STOP_TIMEOUT: Duration = Duration::from_secs(2);

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

/// Starts a lifecycle thread running `which` as call `turn` (see
/// [`midlet::lifecycle`]) and waits up to `timeout` for it. Returns
/// whether the thread started.
fn run_lifecycle(vm: &Vm, which: Lifecycle, turn: u64, timeout: Duration) -> bool {
    let done = Arc::new(Done::default());
    let raise = RaiseOnDrop(Arc::clone(&done));
    let spawned = vm.spawn("MIDlet", move |ctx| {
        let _raise = raise;
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
    rx: mpsc::Receiver<Control>,
    /// Whether the VM is paused.
    frozen: bool,
    /// The turn the next lifecycle call takes; the launch took 0.
    next_turn: u64,
}

impl Controller {
    fn run(mut self) {
        while let Ok(message) = self.rx.recv() {
            match message {
                Control::Pause => self.pause(),
                Control::Resume | Control::ResumeRequest => self.resume(),
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

    fn pause(&mut self) {
        if self.frozen || self.host.peek_status() != InstanceStatus::Running {
            return;
        }
        on_event_thread(&self.vm, LIFECYCLE_TIMEOUT, |ctx| {
            lcdui::event::set_visible(ctx, false)
        });
        self.call(Lifecycle::Pause, LIFECYCLE_TIMEOUT);
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
        if run_lifecycle(&self.vm, which, self.next_turn, timeout) {
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
        run_lifecycle(&self.vm, Lifecycle::Destroy, 0, LIFECYCLE_TIMEOUT);
    }

    /// Ends every thread of the VM and the instance's sound.
    fn finish(self, status: InstanceStatus) {
        self.vm.flush_output();
        self.vm.stop();
        self.vm.resume();
        self.vm.wait_finished(Some(STOP_TIMEOUT));
        if let Some(group) = &self.group {
            group.close();
        }
        self.host.set_status(status);
    }
}

/// One running MIDlet in its own virtual machine.
pub struct MidpInstance {
    id: InstanceId,
    title: String,
    host: HostLink,
    vm: Vm,
    group: Option<Group>,
    control: parking_lot::Mutex<mpsc::Sender<Control>>,
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
        let (tx, rx) = mpsc::channel();
        host.set_control(tx.clone());
        let controller = Controller {
            vm: vm.clone(),
            host: host.clone(),
            group: group.clone(),
            rx,
            frozen: false,
            next_turn: 1,
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
            let _ = tx.send(Control::Stop);
            return Err(e);
        }
        Ok(Arc::new(Self {
            id: host.id(),
            title,
            host,
            vm,
            group,
            control: parking_lot::Mutex::new(tx),
        }))
    }

    fn send_control(&self, message: Control) {
        let _ = self.control.lock().send(message);
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

    fn send(&self, event: InputEvent) {
        if self.vm.is_stopping() {
            return;
        }
        self.vm.post(move |ctx| {
            lcdui::input::queue(ctx, event);
            Ok(())
        });
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
}

impl Drop for MidpInstance {
    /// An instance the app lets go of stops: the VM's own references to
    /// the control channel would otherwise keep it running.
    fn drop(&mut self) {
        self.send_control(Control::Stop);
    }
}
