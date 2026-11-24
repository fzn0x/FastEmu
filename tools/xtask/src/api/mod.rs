//! The API FastEmu offers games, and the API the inventory says they use.
//!
//! [`runtime`] reads back every class the runtime registers, [`inventory`]
//! parses `docs/compat/api/*.md`, and [`stubs`] writes Java stubs of the
//! first, with throws clauses from the second, and compiles them.
//!
//! This module refers only to its own submodules, never to the rest of
//! the xtask, because the integration tests of `fastemu-midp` include it
//! with `#[path]` to build the stubs their test MIDlets compile against.

pub mod inventory;
pub mod java;
pub mod runtime;
pub mod stubs;
