//! The link between an instance's virtual machine and the app that shows
//! it: the latest frame, the status and the log.
//!
//! A [`HostLink`] holds no Java references and no `Vm`, so it may live in
//! the virtual machine (inside [`crate::env::InstanceEnv`]) and in the
//! app's side of the instance at once. Every method returns quickly; the
//! ones the VM calls run with the VM lock held.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use fastemu_core::{Frame, InstanceId, InstanceStatus, LogLevel, LogLine, Notify};
use parking_lot::Mutex;

/// The most log lines kept between two [`HostLink::drain_log`] calls; the
/// oldest go first, so a game that prints in a loop cannot exhaust memory.
const MAX_LOG_LINES: usize = 10_000;

struct HostState {
    status: InstanceStatus,
    frame: Option<Frame>,
    log: VecDeque<LogLine>,
}

struct Inner {
    id: InstanceId,
    notify: Option<Notify>,
    /// Set when the app has been notified and has not read anything since,
    /// so a burst of frames and log lines makes one notification.
    pending: AtomicBool,
    state: Mutex<HostState>,
    errors: AtomicU64,
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
            state: Mutex::new(HostState {
                status: InstanceStatus::Starting,
                frame: None,
                log: VecDeque::new(),
            }),
            errors: AtomicU64::new(0),
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

    /// Tells the app something changed, once until it reads again.
    fn changed(&self) {
        if let Some(notify) = &self.0.notify
            && !self.0.pending.swap(true, Ordering::AcqRel)
        {
            notify(self.0.id);
        }
    }

    /// Shows a new frame.
    pub(crate) fn present(&self, frame: Frame) {
        self.0.state.lock().frame = Some(frame);
        self.changed();
    }

    /// Adds a line to the log. Lines that report an uncaught exception
    /// count as errors.
    pub(crate) fn log(&self, line: LogLine) {
        if line.level == LogLevel::Err && line.text.starts_with("Exception in thread") {
            self.count_error();
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
        link.present(Frame::filled(1, 1, 0, 3));
        assert_eq!(count.load(Ordering::Relaxed), 2);
        assert_eq!(link.drain_log().len(), 1);
        assert!(link.drain_log().is_empty());
    }

    #[test]
    fn uncaught_exceptions_count_as_errors() {
        let (link, _) = counting();
        link.log(LogLine {
            level: LogLevel::Err,
            text: "Exception in thread \"main\" java.lang.Error".into(),
        });
        link.log(LogLine {
            level: LogLevel::Out,
            text: "Exception in thread, but printed by the game".into(),
        });
        assert_eq!(link.error_count(), 1);
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
