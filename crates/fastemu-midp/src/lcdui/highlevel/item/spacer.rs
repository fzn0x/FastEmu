//! `javax.microedition.lcdui.Spacer`: empty space of a minimum size,
//! with no label and no commands.

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::{ItemData, Kind, MAX_SIDE, Needs, changed, data, data_mut};

const CLASS: &str = "javax/microedition/lcdui/Spacer";

/// What a `Spacer` keeps: its minimum size.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpacerState {
    pub width: i32,
    pub height: i32,
}

/// Exactly the minimum size.
pub(super) fn needs(ctx: &Ctx<'_>, item: ObjRef) -> JResult<Needs> {
    let Kind::Spacer(s) = data(ctx, item)?.kind else {
        unreachable!("a Spacer's payload is made by its class")
    };
    Ok(Needs {
        min_width: s.width.min(MAX_SIDE),
        width: s.width.min(MAX_SIDE),
        height: s.height.min(MAX_SIDE),
    })
}

fn set_size(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let (width, height) = (a[1].int(), a[2].int());
    if width < 0 || height < 0 {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "a negative minimum size",
        );
    }
    let item = ctx.non_null(a[0])?;
    data_mut(ctx, item)?.kind = Kind::Spacer(SpacerState { width, height });
    changed(ctx, item)?;
    Ok(Value::Void)
}

fn refuse(ctx: &mut Ctx<'_>) -> JResult<Value> {
    ctx.throw(
        "java/lang/IllegalStateException",
        "a Spacer has no label and no commands",
    )
}

/// The class.
pub fn class() -> NativeClass {
    NativeClass::new(CLASS)
        .extends(super::ITEM)
        .payload_with(|| Box::new(ItemData::new(Kind::Spacer(SpacerState::default()))))
        .method("<init>", "(II)V", set_size)
        .method("setMinimumSize", "(II)V", set_size)
        .method(
            "addCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            |ctx, _| refuse(ctx),
        )
        .method(
            "setDefaultCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            |ctx, _| refuse(ctx),
        )
        .method("setLabel", "(Ljava/lang/String;)V", |ctx, _| refuse(ctx))
}
