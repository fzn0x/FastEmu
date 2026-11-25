//! Garbage collection roots.
//!
//! A collection runs on the thread that holds the VM lock, at a safepoint.
//! Every other thread is then waiting with its stack parked in its table
//! entry, so the roots are this thread's stack plus what [`VmState`]
//! holds:
//!
//! - each thread's slots (locals and operand stacks) up to its top frame,
//!   its native locals and the monitors its synchronized frames hold;
//! - static fields, interned strings and `Class` objects;
//! - global roots, thread objects and objects whose monitor is in use.
//!
//! After the sweep, `util::timer::reap` marks the queues of the timers the
//! collection found dropped and wakes their threads.

use crate::ctx::Ctx;
use crate::heap::Tracer;
use crate::thread::ThreadStack;
use crate::value::Value;
use crate::vm::VmState;

fn mark_stack(t: &mut Tracer<'_>, stack: &ThreadStack) {
    let top = stack.top().min(stack.slots.len());
    for &v in &stack.slots[..top] {
        if let Value::Ref(Some(r)) = v {
            t.mark(r);
        }
    }
    for &r in &stack.locals {
        t.mark(r);
    }
    for frame in &stack.frames {
        t.mark_opt(frame.sync);
    }
}

impl Ctx<'_> {
    /// Runs a full collection now. Callers make sure this thread's current
    /// frame is written back, so its slots up to the top are exact.
    pub(crate) fn collect(&mut self) {
        self.release_dropped_globals();
        let Ctx {
            state,
            thread,
            shared,
            ..
        } = self;
        let VmState {
            heap,
            weak,
            classes,
            interned,
            globals,
            monitors,
            threads,
            ..
        } = &mut **state;
        heap.collect(
            |t| {
                mark_stack(t, thread);
                for entry in threads.values() {
                    t.mark(entry.java_thread);
                    mark_stack(t, &entry.stack);
                }
                for &v in &classes.statics {
                    t.mark_value(v);
                }
                for mirror in classes.mirrors.iter().flatten() {
                    t.mark(*mirror);
                }
                for &s in interned.values() {
                    t.mark(s);
                }
                for g in globals.iter().flatten() {
                    t.mark(*g);
                }
                for &obj in monitors.keys() {
                    t.mark(obj);
                }
            },
            weak,
        );
        // The threads of dropped timers end once no task is left.
        if crate::util::timer::reap(state) {
            shared.cond.notify_all();
        }
    }

    /// Collects now, at a native's request (`System.gc`). The calling
    /// native's arguments and locals are roots, so this is safe anywhere.
    pub fn gc(&mut self) {
        self.collect();
    }
}
