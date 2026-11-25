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

use fastemu_core::{Frame, InputEvent, InstanceId, InstanceStatus, LogLevel, LogLine, Notify};
use fastemu_jvm::{VmLimits, cut_log_text};
use parking_lot::Mutex;

/// The most log lines kept between two [`HostLink::drain_log`] calls; the
/// oldest go first, so a game that prints in a loop cannot exhaust memory.
const MAX_LOG_LINES: usize = 10_000;

/// The most bytes of log text kept between two [`HostLink::drain_log`]
/// calls. A line is cut to `VmLimits::max_log_line_bytes` first, so this
/// holds at least 256 of them.
const MAX_LOG_BYTES: usize = 1 << 20;

/// The longest message a crash carries to the app, in bytes. The text is
/// the game's own (an exception's message), and the app shows it whole.
const MAX_CRASH_MESSAGE_BYTES: usize = 1024;

/// What the virtual machine prints when a panic ends one of its threads.
const INTERNAL_ERROR: &str = "internal error in the virtual machine";

/// Messages to the instance's control thread, which owns the `Vm`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    /// Input from the app, for the event thread.
    Input(InputEvent),
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
    /// The bytes of text the queued lines hold.
    log_bytes: usize,
    /// The web page the game asked to open, until the app takes it.
    link: Option<String>,
}

struct Inner {
    id: InstanceId,
    notify: Option<Notify>,
    /// What bounds a log line.
    limits: VmLimits,
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
    /// A link for instance `id` that calls `notify` on news, with the
    /// default limits.
    pub fn new(id: InstanceId, notify: Option<Notify>) -> Self {
        Self::with_limits(id, notify, VmLimits::default())
    }

    /// A link for instance `id` that calls `notify` on news and cuts each
    /// log line to `limits.max_log_line_bytes`: the limits of the virtual
    /// machine the link serves.
    pub fn with_limits(id: InstanceId, notify: Option<Notify>, limits: VmLimits) -> Self {
        Self(Arc::new(Inner {
            id,
            notify,
            limits,
            pending: AtomicBool::new(false),
            wants_text: AtomicBool::new(false),
            state: Mutex::new(HostState {
                status: InstanceStatus::Starting,
                frame: None,
                log: VecDeque::new(),
                log_bytes: 0,
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
        let mut state = self.0.state.lock();
        state.log_bytes = 0;
        state.log.drain(..).collect()
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

    /// Shows a new frame. The frame it replaces is freed after the lock is
    /// released, so the app's `frame` never waits for that.
    pub(crate) fn present(&self, frame: Frame) {
        let old = self.0.state.lock().frame.replace(frame);
        drop(old);
        self.changed();
    }

    /// Adds a line to the log. Lines that report an uncaught exception
    /// count as errors; an internal error of the VM crashes the instance.
    ///
    /// Every line the app and the command line runner print passes here,
    /// whoever wrote it (the virtual machine, a native, the JAR's source of
    /// classes and resources): it is cut to `max_log_line_bytes`, and the
    /// control characters in it are escaped (see [`escape_control`]), so a
    /// game's text cannot make a terminal do anything or pass for a line
    /// of its own. The queue holds [`MAX_LOG_LINES`] lines and
    /// [`MAX_LOG_BYTES`] bytes at most, oldest first out.
    pub(crate) fn log(&self, line: LogLine) {
        let text = clean_text(line.text, self.0.limits.max_log_line_bytes);
        if line.level == LogLevel::Err {
            if text.starts_with("Exception in thread") {
                self.count_error();
            } else if text.starts_with(INTERNAL_ERROR) {
                self.count_error();
                self.control(Control::Crashed(text.clone()));
            }
        }
        {
            let mut state = self.0.state.lock();
            state.log_bytes += text.len();
            state.log.push_back(LogLine {
                level: line.level,
                text,
            });
            while state.log.len() > MAX_LOG_LINES || state.log_bytes > MAX_LOG_BYTES {
                let Some(oldest) = state.log.pop_front() else {
                    break;
                };
                state.log_bytes -= oldest.text.len();
            }
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
    /// run natives without a launcher have none). A crash message is the
    /// game's text, so it is cut to [`MAX_CRASH_MESSAGE_BYTES`] and its
    /// control characters are escaped.
    pub(crate) fn control(&self, message: Control) {
        let message = match message {
            Control::Crashed(text) => Control::Crashed(clean_text(text, MAX_CRASH_MESSAGE_BYTES)),
            other => other,
        };
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

/// `text` cut to `max_bytes` and with its control characters escaped.
fn clean_text(text: String, max_bytes: usize) -> String {
    // Cut first, so escaping never works on more than a line; cut again,
    // since an escaped character takes more bytes than it did.
    cut_log_text(escape_control(cut_log_text(text, max_bytes)), max_bytes)
}

/// Escapes what a terminal or a reader could take for something other than
/// text. CR and LF become the two characters `\r` and `\n`, so a line stays
/// one line, and every other control character (below U+0020 but the tab,
/// U+007F, and U+0080 to U+009F, which hold the one character forms of
/// escape sequences) becomes U+FFFD. ESC is one of them, so no sequence
/// that clears a screen, sets a title or writes to the clipboard survives.
fn escape_control(text: String) -> String {
    if !text.chars().any(|c| c.is_control() && c != '\t') {
        return text;
    }
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match c {
            '\r' => out.push_str("\\r"),
            '\n' => out.push_str("\\n"),
            c if c.is_control() && c != '\t' => out.push('\u{fffd}'),
            c => out.push(c),
        }
    }
    out
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

    fn out(text: impl Into<String>) -> LogLine {
        LogLine {
            level: LogLevel::Out,
            text: text.into(),
        }
    }

    #[test]
    fn a_long_line_is_cut_to_the_limit() {
        let (link, _) = counting();
        let max = VmLimits::default().max_log_line_bytes;
        link.log(out("x".repeat(10 * max)));
        link.log(out("short"));
        let lines = link.drain_log();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].text.len() <= max);
        assert!(lines[0].text.ends_with(" more characters)"));
        assert_eq!(lines[1].text, "short");
    }

    #[test]
    fn the_limits_of_a_link_set_the_cut() {
        let limits = VmLimits {
            max_log_line_bytes: 100,
            ..VmLimits::default()
        };
        let link = HostLink::with_limits(InstanceId::next(), None, limits);
        link.log(out("y".repeat(5000)));
        assert!(link.drain_log()[0].text.len() <= 100);
    }

    #[test]
    fn three_hundred_lines_of_a_megabyte_keep_at_most_a_megabyte() {
        let (link, _) = counting();
        for i in 0..300 {
            link.log(out(format!("{i} {}", "x".repeat(1 << 20))));
        }
        let lines = link.drain_log();
        let bytes: usize = lines.iter().map(|l| l.text.len()).sum();
        assert!(bytes <= MAX_LOG_BYTES, "{bytes} bytes");
        assert!(lines.len() >= 250, "{} lines", lines.len());
        // The newest are the ones kept.
        assert!(lines.last().unwrap().text.starts_with("299 "));
        assert!(!lines[0].text.starts_with("0 "));
    }

    #[test]
    fn a_drain_empties_the_byte_budget() {
        let (link, _) = counting();
        let line = "z".repeat(4000);
        for _ in 0..400 {
            link.log(out(line.clone()));
        }
        assert!(link.drain_log().len() < 400);
        for _ in 0..100 {
            link.log(out(line.clone()));
        }
        assert_eq!(link.drain_log().len(), 100);
    }

    #[test]
    fn control_characters_in_a_line_are_escaped() {
        let (link, _) = counting();
        // Clear the screen, set the title, write to the clipboard, and a
        // line that pretends to come from the runtime.
        link.log(out(
            "\u{1b}[2J\u{1b}]0;owned\u{7}\u{1b}]52;c;AAAA\u{7}ok\r\ninfo: the game asked to open https://example.com/",
        ));
        link.log(out("tab\there, \u{85} and \u{7f} and \u{9b}31m"));
        let lines = link.drain_log();
        assert_eq!(
            lines[0].text,
            "\u{fffd}[2J\u{fffd}]0;owned\u{fffd}\u{fffd}]52;c;AAAA\u{fffd}ok\\r\\ninfo: the game asked to open https://example.com/"
        );
        assert_eq!(
            lines[1].text,
            "tab\there, \u{fffd} and \u{fffd} and \u{fffd}31m"
        );
        for line in &lines {
            assert!(!line.text.chars().any(|c| c.is_control() && c != '\t'));
        }
    }

    #[test]
    fn a_line_that_grows_when_escaped_still_fits() {
        let (link, _) = counting();
        let max = VmLimits::default().max_log_line_bytes;
        // One byte becomes two, or three.
        link.log(out("\n".repeat(max)));
        link.log(out("\u{1}".repeat(max)));
        for line in link.drain_log() {
            assert!(line.text.len() <= max, "{} bytes", line.text.len());
            assert!(!line.text.chars().any(char::is_control), "{:?}", line.text);
        }
    }

    #[test]
    fn a_crash_message_is_cut_and_escaped() {
        let (link, _) = counting();
        let (tx, rx) = mpsc::channel();
        link.set_control(tx);
        link.control(Control::Crashed(format!(
            "boom\n\u{1b}[2J{}",
            "q".repeat(5000)
        )));
        link.control(Control::Stop);
        let Ok(Control::Crashed(message)) = rx.try_recv() else {
            panic!("a crash message");
        };
        assert!(message.len() <= MAX_CRASH_MESSAGE_BYTES);
        assert!(message.starts_with("boom\\n\u{fffd}[2Jqqq"));
        assert!(matches!(rx.try_recv(), Ok(Control::Stop)));
    }
}
