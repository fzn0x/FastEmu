//! The link between an instance's virtual machine and the app that shows
//! it: the latest frame, the status, the log, the web page the game asks
//! to open, and the messages natives send to the instance's control
//! thread.
//!
//! A [`HostLink`] holds no Java references and no `Vm`, so it may live in
//! the virtual machine (inside [`crate::env::InstanceEnv`]) and in the
//! app's [`crate::instance::MidpInstance`] at once. Every method returns
//! quickly; the ones the VM calls run with the VM lock held.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, mpsc};

use fastemu_core::{Frame, InstanceId, InstanceStatus, LogLevel, LogLine, Notify};
use parking_lot::Mutex;

/// The most log lines kept between two [`HostLink::drain_log`] calls; the
/// oldest go first, so a game that prints in a loop cannot exhaust memory.
const MAX_LOG_LINES: usize = 10_000;

/// What the virtual machine prints when a panic ends one of its threads.
const INTERNAL_ERROR: &str = "internal error in the virtual machine";

/// Messages to the instance's control thread, which owns the `Vm`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    /// The app paused the instance.
    Pause,
    /// The app resumed it.
    Resume,
    /// The MIDlet called `resumeRequest`.
    ResumeRequest,
    /// The app stopped it, or dropped it.
    Stop,
    /// The MIDlet called `notifyDestroyed`.
    Exited,
    /// The game hit an error the runtime cannot recover from.
    Crashed(String),
}

struct HostState {
    status: InstanceStatus,
    frame: Option<Frame>,
    log: VecDeque<LogLine>,
    /// The web page the game asked to open, until the app takes it.
    link: Option<String>,
}

struct Inner {
    id: InstanceId,
    notify: Option<Notify>,
    /// Set when the app has been notified and has not read anything since,
    /// so a burst of frames and log lines makes one notification.
    pending: AtomicBool,
    /// Whether a text box is shown or a text field has the focus.
    wants_text: AtomicBool,
    state: Mutex<HostState>,
    errors: AtomicU64,
    control: Mutex<Option<mpsc::Sender<Control>>>,
}

/// One instance's side of the app: cheap to clone, shared by the VM's
/// threads and the app.
#[derive(Clone)]
pub struct HostLink(Arc<Inner>);

impl std::fmt::Debug for HostLink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HostLink")
            .field("id", &self.0.id)
            .finish_non_exhaustive()
    }
}

impl HostLink {
    /// A link for instance `id` that calls `notify` on news.
    pub fn new(id: InstanceId, notify: Option<Notify>) -> Self {
        Self(Arc::new(Inner {
            id,
            notify,
            pending: AtomicBool::new(false),
            wants_text: AtomicBool::new(false),
            state: Mutex::new(HostState {
                status: InstanceStatus::Starting,
                frame: None,
                log: VecDeque::new(),
                link: None,
            }),
            errors: AtomicU64::new(0),
            control: Mutex::new(None),
        }))
    }

    /// The instance this link serves.
    pub fn id(&self) -> InstanceId {
        self.0.id
    }

    /// The instance's status.
    pub fn status(&self) -> InstanceStatus {
        self.0.pending.store(false, Ordering::Release);
        self.0.state.lock().status.clone()
    }

    /// The status, read by the runtime itself without counting as the app
    /// having looked.
    pub(crate) fn peek_status(&self) -> InstanceStatus {
        self.0.state.lock().status.clone()
    }

    /// The latest presented frame.
    pub fn frame(&self) -> Option<Frame> {
        self.0.pending.store(false, Ordering::Release);
        self.0.state.lock().frame.clone()
    }

    /// Takes the log lines written since the last call.
    pub fn drain_log(&self) -> Vec<LogLine> {
        self.0.pending.store(false, Ordering::Release);
        self.0.state.lock().log.drain(..).collect()
    }

    /// How many exceptions the game let escape (uncaught in a thread or
    /// thrown out of a callback) and internal errors, since launch.
    pub fn error_count(&self) -> u64 {
        self.0.errors.load(Ordering::Relaxed)
    }

    /// Whether the game shows a text box or a focused text or date field,
    /// so the app should send typed text: `Instance::wants_text`.
    pub fn wants_text(&self) -> bool {
        self.0.wants_text.load(Ordering::Acquire)
    }

    /// Records whether the game wants typed text, telling the app when
    /// that changes.
    pub(crate) fn set_wants_text(&self, wants: bool) {
        if self.0.wants_text.swap(wants, Ordering::AcqRel) != wants {
            self.changed();
        }
    }

    /// Takes the web page the game asked to open since the last call:
    /// `Instance::take_link_request`.
    pub fn take_link(&self) -> Option<String> {
        self.0.pending.store(false, Ordering::Release);
        self.0.state.lock().link.take()
    }

    /// Records a web page the game asks the app to open, in place of one
    /// the app has not taken yet, and tells the app.
    pub(crate) fn request_link(&self, url: String) {
        self.0.state.lock().link = Some(url);
        self.changed();
    }

    /// Drops the web page the game asked to open if the app has not taken
    /// it yet. Returns whether there was one.
    pub(crate) fn cancel_link(&self) -> bool {
        self.0.state.lock().link.take().is_some()
    }

    /// Tells the app something changed, once until it reads again.
    fn changed(&self) {
        if let Some(notify) = &self.0.notify
            && !self.0.pending.swap(true, Ordering::AcqRel)
        {
            notify(self.0.id);
        }
    }

    /// Changes the status. A finished status (exited or crashed) is final.
    pub(crate) fn set_status(&self, status: InstanceStatus) {
        {
            let mut state = self.0.state.lock();
            if state.status.is_finished() || state.status == status {
                return;
            }
            state.status = status;
        }
        self.changed();
    }

    /// Shows a new frame.
    pub(crate) fn present(&self, frame: Frame) {
        self.0.state.lock().frame = Some(frame);
        self.changed();
    }

    /// Adds a line to the log. Lines that report an uncaught exception
    /// count as errors; an internal error of the VM crashes the instance.
    pub(crate) fn log(&self, line: LogLine) {
        if line.level == LogLevel::Err {
            if line.text.starts_with("Exception in thread") {
                self.count_error();
            } else if line.text.starts_with(INTERNAL_ERROR) {
                self.count_error();
                self.control(Control::Crashed(line.text.clone()));
            }
        }
        {
            let mut state = self.0.state.lock();
            if state.log.len() >= MAX_LOG_LINES {
                state.log.pop_front();
            }
            state.log.push_back(line);
        }
        self.changed();
    }

    /// Counts an exception the runtime caught and logged.
    pub(crate) fn count_error(&self) {
        self.0.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Where the control thread listens.
    pub(crate) fn set_control(&self, sender: mpsc::Sender<Control>) {
        *self.0.control.lock() = Some(sender);
    }

    /// Sends a message to the control thread, if there is one (tests that
    /// run natives without a launcher have none).
    pub(crate) fn control(&self, message: Control) {
        if let Some(sender) = &*self.0.control.lock() {
            let _ = sender.send(message);
        }
    }

    /// A log sink for the VM's configuration that writes to this link.
    pub fn log_sink(&self) -> fastemu_jvm::LogSink {
        let link = self.clone();
        Arc::new(move |line| link.log(line))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    fn counting() -> (HostLink, Arc<AtomicUsize>) {
        let count = Arc::new(AtomicUsize::new(0));
        let seen = Arc::clone(&count);
        let notify: Notify = Arc::new(move |_| {
            seen.fetch_add(1, Ordering::Relaxed);
        });
        (HostLink::new(InstanceId::next(), Some(notify)), count)
    }

    #[test]
    fn notifications_coalesce_until_the_app_reads() {
        let (link, count) = counting();
        link.present(Frame::filled(1, 1, 0, 1));
        link.present(Frame::filled(1, 1, 0, 2));
        link.log(LogLine {
            level: LogLevel::Out,
            text: "hi".into(),
        });
        assert_eq!(count.load(Ordering::Relaxed), 1);
        assert_eq!(link.frame().map(|f| f.serial()), Some(2));
        link.set_status(InstanceStatus::Running);
        assert_eq!(count.load(Ordering::Relaxed), 2);
        assert_eq!(link.drain_log().len(), 1);
        assert!(link.drain_log().is_empty());
    }

    #[test]
    fn the_app_hears_when_text_is_wanted() {
        let (link, count) = counting();
        assert!(!link.wants_text());
        link.set_wants_text(true);
        assert!(link.wants_text());
        assert_eq!(count.load(Ordering::Relaxed), 1);
        link.frame();
        link.set_wants_text(true);
        assert_eq!(count.load(Ordering::Relaxed), 1, "no change, no news");
        link.set_wants_text(false);
        assert!(!link.wants_text());
        assert_eq!(count.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn the_app_takes_the_latest_link_once() {
        let (link, count) = counting();
        assert_eq!(link.take_link(), None);
        link.request_link("http://a.example/".into());
        link.request_link("http://b.example/".into());
        assert_eq!(count.load(Ordering::Relaxed), 1, "one notification");
        assert_eq!(link.take_link().as_deref(), Some("http://b.example/"));
        assert_eq!(link.take_link(), None);
        link.request_link("http://c.example/".into());
        assert_eq!(count.load(Ordering::Relaxed), 2, "taking counts as reading");
        assert!(link.cancel_link());
        assert!(!link.cancel_link());
        assert_eq!(link.take_link(), None);
    }

    #[test]
    fn finished_statuses_are_final() {
        let (link, _) = counting();
        link.set_status(InstanceStatus::Exited);
        link.set_status(InstanceStatus::Running);
        assert_eq!(link.status(), InstanceStatus::Exited);
    }

    #[test]
    fn uncaught_exceptions_count_and_internal_errors_crash() {
        let (link, _) = counting();
        let (tx, rx) = mpsc::channel();
        link.set_control(tx);
        link.log(LogLine {
            level: LogLevel::Err,
            text: "Exception in thread \"main\" java.lang.Error".into(),
        });
        link.log(LogLine {
            level: LogLevel::Err,
            text: format!("{INTERNAL_ERROR}: boom; stopping it"),
        });
        assert_eq!(link.error_count(), 2);
        assert!(matches!(rx.try_recv(), Ok(Control::Crashed(_))));
    }

    #[test]
    fn the_log_keeps_the_newest_lines() {
        let (link, _) = counting();
        for i in 0..MAX_LOG_LINES + 5 {
            link.log(LogLine {
                level: LogLevel::Out,
                text: i.to_string(),
            });
        }
        let lines = link.drain_log();
        assert_eq!(lines.len(), MAX_LOG_LINES);
        assert_eq!(lines[0].text, "5");
    }
}
