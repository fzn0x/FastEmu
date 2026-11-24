//! The API FastEmu offers games, and the API the inventory says they use.
//!
//! [`runtime`] reads back every class the runtime registers, and
//! [`inventory`] parses `docs/compat/api/*.md`.
//!
//! This module refers only to its own submodules, never to the rest of
//! the xtask, so other crates' tests can include it with `#[path]`.

pub mod inventory;
pub mod runtime;
