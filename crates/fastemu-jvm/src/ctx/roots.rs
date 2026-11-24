//! Keeping references alive: native locals and global roots.

use std::fmt;
use std::sync::Weak;

use super::Ctx;
use crate::value::{ObjRef, Value};
use crate::vm::VmShared;

/// A reference the host keeps across calls: the current `Displayable`, an
/// event queue, a listener. The object stays alive until the handle is
/// dropped. A `GlobalRef` is `Send` and `Sync`, so it may live in host
/// structures and in tasks posted from other threads.
pub struct GlobalRef {
    index: u32,
    vm: u64,
    owner: Weak<VmShared>,
}

impl GlobalRef {
    /// The object, rooted as a native local of `ctx`'s current frame.
    ///
    /// # Panics
    ///
    /// When `ctx` belongs to another virtual machine.
    pub fn get(&self, ctx: &mut Ctx<'_>) -> ObjRef {
        ctx.global(self)
    }
}

impl fmt::Debug for GlobalRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GlobalRef({} in vm {})", self.index, self.vm)
    }
}

impl Drop for GlobalRef {
    fn drop(&mut self) {
        // The lock may be held by the dropping thread, so the slot is only
        // queued here and cleared at the next collection.
        if let Some(shared) = self.owner.upgrade() {
            shared.released_globals.lock().push(self.index);
        }
    }
}

impl Ctx<'_> {
    /// Roots `obj` until the current native frame returns, and returns it.
    /// References the context hands out are rooted this way already; use
    /// it for references read from array slices.
    #[inline]
    pub fn root(&mut self, obj: ObjRef) -> ObjRef {
        self.thread.locals.push(obj);
        obj
    }

    /// Roots `v` if it is a reference.
    #[inline]
    pub(crate) fn root_value(&mut self, v: Value) -> Value {
        if let Value::Ref(Some(r)) = v {
            self.thread.locals.push(r);
        }
        v
    }

    /// Runs `f` and then drops the native locals it created, so a long
    /// native loop (an event thread's) does not keep every object it
    /// touched alive. References `f` returns are not rooted afterwards;
    /// root them again with [`Ctx::root`] if they must survive.
    pub fn local_frame<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let mark = self.thread.locals.len();
        let r = f(self);
        self.thread.locals.truncate(mark);
        r
    }

    /// Makes a global root for `obj`.
    pub fn new_global(&mut self, obj: ObjRef) -> GlobalRef {
        let index = match self.state.free_globals.pop() {
            Some(i) => {
                self.state.globals[i as usize] = Some(obj);
                i
            }
            None => {
                self.state.globals.push(Some(obj));
                (self.state.globals.len() - 1) as u32
            }
        };
        GlobalRef {
            index,
            vm: self.shared.id,
            owner: self.shared.this.clone(),
        }
    }

    /// The object behind `global`, rooted as a native local.
    ///
    /// # Panics
    ///
    /// When `global` belongs to another virtual machine.
    pub fn global(&mut self, global: &GlobalRef) -> ObjRef {
        assert_eq!(
            global.vm, self.shared.id,
            "a GlobalRef used with another virtual machine"
        );
        let obj =
            self.state.globals[global.index as usize].expect("a live GlobalRef keeps its slot");
        self.root(obj)
    }

    /// Clears the slots of dropped [`GlobalRef`]s.
    pub(crate) fn release_dropped_globals(&mut self) {
        let released = std::mem::take(&mut *self.shared.released_globals.lock());
        for index in released {
            self.state.globals[index as usize] = None;
            self.state.free_globals.push(index);
        }
    }
}
