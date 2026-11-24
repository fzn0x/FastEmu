//! Conformance MIDlets: small MIDlets for the behaviours
//! `docs/compat/notes/11-test-material.md` lists, compiled against stubs
//! of exactly the API FastEmu registers (see `conformance_support`),
//! packed into JARs and run through the launcher like real games, with
//! their logs and frames checked.
//!
//! The Java sources are in `tests/conformance/<area>/`, one folder per
//! module here.

#[path = "../conformance_support/mod.rs"]
mod conformance_support;
#[path = "../support/mod.rs"]
mod support;

mod resources;
mod suites;
