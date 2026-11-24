//! The per-instance state of the display: its size and fonts. It lives in
//! the virtual machine as an extension, made on first use from the
//! instance's environment.

use std::collections::HashMap;
use std::sync::Arc;

use fastemu_gfx::{FontLibrary, TextRendering};
use fastemu_jvm::{Ctx, GlobalRef};

use crate::env::env;

/// The display of one instance.
pub(crate) struct LcduiState {
    pub fonts: Arc<FontLibrary>,
    /// `Font` objects by face, style and size, so `getFont` hands out one
    /// object per font as phones did.
    pub font_objects: HashMap<(i32, i32, i32), Arc<GlobalRef>>,
}

impl LcduiState {
    fn new(ctx: &Ctx<'_>) -> Self {
        let env = env(ctx);
        Self {
            fonts: Arc::new(FontLibrary::new(
                env.profile.font_sizes,
                TextRendering::Antialiased,
            )),
            font_objects: HashMap::new(),
        }
    }
}

/// The display state of the instance `ctx` runs in, made on first use.
pub(crate) fn state<'a>(ctx: &'a mut Ctx<'_>) -> &'a mut LcduiState {
    if ctx.extension::<LcduiState>().is_none() {
        let fresh = LcduiState::new(ctx);
        ctx.set_extension(fresh);
    }
    ctx.extension_mut::<LcduiState>()
        .expect("the display state was installed above")
}
