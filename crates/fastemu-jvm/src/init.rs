//! Class initialization (JVMS 5.5): superclass first, constant fields,
//! then the static initializer, once, with other threads waiting and the
//! initializing thread allowed back in recursively.

use std::sync::Arc;

use crate::class::{ClassId, ConstValue, InitState};
use crate::ctx::{Ctx, JResult, Throw};
use crate::lang;

impl Ctx<'_> {
    /// Initializes `class` if it is not yet, running its static
    /// initializer on this thread. Throws what the initializer throws
    /// (wrapped in `ExceptionInInitializerError` unless it is an `Error`),
    /// and `NoClassDefFoundError` for a class whose initializer failed
    /// before.
    #[inline]
    pub fn initialize(&mut self, class: ClassId) -> JResult<()> {
        if self.state.classes.init[class.index()] == InitState::Done {
            return Ok(());
        }
        self.initialize_slow(class)
    }

    fn initialize_slow(&mut self, class: ClassId) -> JResult<()> {
        loop {
            match self.state.classes.init[class.index()] {
                InitState::Done => return Ok(()),
                InitState::Running(t) if t == self.tid => return Ok(()),
                InitState::Running(_) => {
                    self.wait_raw(None)?;
                }
                InitState::Failed => {
                    let message = format!(
                        "Could not initialize class {}",
                        self.class_name(class).replace('/', ".")
                    );
                    return Err(
                        self.new_exception("java/lang/NoClassDefFoundError", Some(&message))
                    );
                }
                InitState::Linked => break,
            }
        }
        self.state.classes.init[class.index()] = InitState::Running(self.tid);
        let result = self.run_initializers(class);
        self.state.classes.init[class.index()] = if result.is_ok() {
            InitState::Done
        } else {
            InitState::Failed
        };
        self.shared.cond.notify_all();
        result
    }

    fn run_initializers(&mut self, class: ClassId) -> JResult<()> {
        let c = Arc::clone(self.state.classes.get(class));
        if !c.is_interface()
            && let Some(sup) = c.superclass
        {
            self.initialize(sup)?;
        }
        for field in c.fields.iter().filter(|f| f.is_static()) {
            let value = match &field.constant {
                Some(ConstValue::Value(v)) => *v,
                Some(ConstValue::String(chars)) => {
                    lang::string::intern(&mut self.state, chars).into()
                }
                None => continue,
            };
            self.state.classes.statics[field.slot as usize] = value;
        }
        let Some(clinit) = c.declared_method("<clinit>", "()V").cloned() else {
            return Ok(());
        };
        match self.invoke(&clinit, &[]) {
            Ok(_) => Ok(()),
            Err(Throw::Exception(e)) => {
                let error = self.find_class("java/lang/Error")?;
                if self.is_instance(e, error) {
                    return Err(Throw::Exception(e));
                }
                let wrapped = self.new_exception("java/lang/ExceptionInInitializerError", None);
                if let Throw::Exception(w) = wrapped {
                    lang::throwable::set_cause(self, w, e);
                }
                Err(wrapped)
            }
            Err(Throw::Stop) => Err(Throw::Stop),
        }
    }
}
