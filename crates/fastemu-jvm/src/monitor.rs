//! Java monitors: `synchronized`, `wait`, `notify` and `notifyAll`.
//!
//! A monitor exists only while some thread owns it or waits on it. Every
//! blocked thread waits on the VM's one condition variable, which releases
//! the VM lock, and checks its own condition again when woken.

use std::time::Instant;

use crate::ctx::{Ctx, JResult};
use crate::thread::ThreadId;
use crate::value::ObjRef;

#[derive(Debug, Default)]
pub(crate) struct Monitor {
    pub(crate) owner: Option<ThreadId>,
    /// How many times the owner entered.
    pub(crate) count: u32,
    /// Threads blocked trying to enter.
    pub(crate) entering: u32,
    /// Threads in `wait`, in arrival order.
    pub(crate) waiters: Vec<ThreadId>,
}

impl Monitor {
    fn is_idle(&self) -> bool {
        self.owner.is_none() && self.entering == 0 && self.waiters.is_empty()
    }
}

impl Ctx<'_> {
    /// Enters the monitor of `obj`, waiting while another thread owns it.
    pub fn monitor_enter(&mut self, obj: ObjRef) -> JResult<()> {
        let me = self.tid;
        loop {
            let m = self.state.monitors.entry(obj).or_default();
            match m.owner {
                None => {
                    m.owner = Some(me);
                    m.count = 1;
                    return Ok(());
                }
                Some(owner) if owner == me => {
                    m.count += 1;
                    return Ok(());
                }
                Some(_) => {
                    m.entering += 1;
                    let woke = self.wait_raw(None);
                    if let Some(m) = self.state.monitors.get_mut(&obj) {
                        m.entering -= 1;
                    }
                    woke?;
                }
            }
        }
    }

    /// Leaves the monitor of `obj`. Throws `IllegalMonitorStateException`
    /// if this thread does not own it.
    pub fn monitor_exit(&mut self, obj: ObjRef) -> JResult<()> {
        let me = self.tid;
        let Some(m) = self
            .state
            .monitors
            .get_mut(&obj)
            .filter(|m| m.owner == Some(me))
        else {
            return Err(self.new_exception("java/lang/IllegalMonitorStateException", None));
        };
        m.count -= 1;
        if m.count == 0 {
            m.owner = None;
            let wake = m.entering > 0;
            if m.is_idle() {
                self.state.monitors.remove(&obj);
            }
            if wake {
                self.shared.cond.notify_all();
            }
        }
        Ok(())
    }

    fn owned_monitor(&mut self, obj: ObjRef) -> JResult<&mut Monitor> {
        let me = self.tid;
        if self
            .state
            .monitors
            .get(&obj)
            .is_some_and(|m| m.owner == Some(me))
        {
            Ok(self.state.monitors.get_mut(&obj).expect("checked above"))
        } else {
            Err(self.new_exception(
                "java/lang/IllegalMonitorStateException",
                Some("current thread is not owner"),
            ))
        }
    }

    /// `Object.wait`: releases the monitor of `obj` until notified,
    /// interrupted or past `deadline`, then takes it back.
    pub fn monitor_wait(&mut self, obj: ObjRef, deadline: Option<Instant>) -> JResult<()> {
        let me = self.tid;
        self.owned_monitor(obj)?;
        if self.take_interrupt() {
            return Err(self.new_exception("java/lang/InterruptedException", None));
        }
        let m = self.owned_monitor(obj)?;
        let saved = m.count;
        m.owner = None;
        m.count = 0;
        m.waiters.push(me);
        let wake_entering = m.entering > 0;
        if let Some(e) = self.state.threads.get_mut(&me) {
            e.notified = false;
        }
        if wake_entering {
            self.shared.cond.notify_all();
        }
        let mut interrupted = false;
        let mut outcome = Ok(());
        loop {
            let entry = self
                .state
                .threads
                .get_mut(&me)
                .expect("a running thread has an entry");
            if entry.notified {
                break;
            }
            if entry.interrupted {
                entry.interrupted = false;
                interrupted = true;
                break;
            }
            if deadline.is_some_and(|d| Instant::now() >= d) {
                break;
            }
            if let Err(e) = self.wait_raw(deadline) {
                outcome = Err(e);
                break;
            }
        }
        if let Some(m) = self.state.monitors.get_mut(&obj) {
            m.waiters.retain(|&t| t != me);
        }
        outcome?;
        // A wait that ends while paused parks before it takes the monitor
        // back, so a host thread may still enter it meanwhile.
        self.park_while_paused()?;
        // Take the monitor back before returning, as Java requires.
        loop {
            let m = self.state.monitors.entry(obj).or_default();
            if m.owner.is_none() {
                m.owner = Some(me);
                m.count = saved;
                break;
            }
            m.entering += 1;
            let woke = self.wait_raw(None);
            if let Some(m) = self.state.monitors.get_mut(&obj) {
                m.entering -= 1;
            }
            woke?;
        }
        if interrupted {
            return Err(self.new_exception("java/lang/InterruptedException", None));
        }
        Ok(())
    }

    /// `Object.notify` (`all` false) and `Object.notifyAll` (`all` true).
    pub fn monitor_notify(&mut self, obj: ObjRef, all: bool) -> JResult<()> {
        let m = self.owned_monitor(obj)?;
        let woken: Vec<ThreadId> = if all {
            std::mem::take(&mut m.waiters)
        } else if m.waiters.is_empty() {
            Vec::new()
        } else {
            vec![m.waiters.remove(0)]
        };
        if woken.is_empty() {
            return Ok(());
        }
        for tid in woken {
            if let Some(e) = self.state.threads.get_mut(&tid) {
                e.notified = true;
            }
        }
        self.shared.cond.notify_all();
        Ok(())
    }

    /// Whether this thread owns the monitor of `obj`.
    pub fn holds_lock(&self, obj: ObjRef) -> bool {
        self.state
            .monitors
            .get(&obj)
            .is_some_and(|m| m.owner == Some(self.tid))
    }
}
