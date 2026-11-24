//! `com.nokia.mid.ui.FullCanvas`: a `Canvas` that always fills the
//! screen, for MIDP 1.0 Nokia phones.
//!
//! A `FullCanvas` has no commands (`addCommand` and `setCommandListener`
//! throw `IllegalStateException`, as on the phones and in J2ME Loader), so
//! the soft keys reach the game as key events with the codes of its
//! `KEY_SOFTKEY*` constants: the runtime passes the soft keys of a canvas
//! without a `CommandListener` to the game. The `Graphics` its `paint`
//! gets is also a `DirectGraphics` (see [`super::direct_graphics`]).

use fastemu_jvm::{NativeClass, Value, classfile::access};

use crate::lcdui::displayable::data_mut;

const CLASS: &str = "com/nokia/mid/ui/FullCanvas";

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    NativeClass::new(CLASS)
        .extends("javax/microedition/lcdui/Canvas")
        .abstract_class()
        .constant("KEY_SOFTKEY1", "I", int(-6))
        .constant("KEY_SOFTKEY2", "I", int(-7))
        .constant("KEY_SEND", "I", int(-10))
        .constant("KEY_END", "I", int(-11))
        .constant("KEY_SOFTKEY3", "I", int(-5))
        .constant("KEY_UP_ARROW", "I", int(-1))
        .constant("KEY_DOWN_ARROW", "I", int(-2))
        .constant("KEY_LEFT_ARROW", "I", int(-3))
        .constant("KEY_RIGHT_ARROW", "I", int(-4))
        .method_with("<init>", "()V", access::PROTECTED, |ctx, a| {
            // Not shown yet, so there is nothing to lay out again.
            let this = ctx.non_null(a[0])?;
            data_mut(ctx, this)?.full_screen = true;
            Ok(Value::Void)
        })
        .method(
            "addCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            |ctx, _| {
                ctx.throw(
                    "java/lang/IllegalStateException",
                    "a FullCanvas has no commands",
                )
            },
        )
        .method(
            "setCommandListener",
            "(Ljavax/microedition/lcdui/CommandListener;)V",
            |ctx, _| {
                ctx.throw(
                    "java/lang/IllegalStateException",
                    "a FullCanvas has no commands",
                )
            },
        )
}
