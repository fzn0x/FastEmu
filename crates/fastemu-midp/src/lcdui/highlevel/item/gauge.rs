//! `javax.microedition.lcdui.Gauge`: a bar the user sets (interactive) or
//! that shows progress, with the four states of an indefinite gauge.
//!
//! `CONTINUOUS_RUNNING` animates by itself: while one is shown, its
//! screen's timer repaints it every [`FRAME`]. `INCREMENTAL_UPDATING`
//! moves one step each time the game sets that value again.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

use fastemu_core::PointerAction;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::{Handled, ItemData, Kind, Needs, changed, data, data_mut};
use crate::lcdui::Rect;
use crate::lcdui::highlevel::draw::{self, Fonts, Painter};
use crate::lcdui::keys;
use crate::lcdui::screen::{KeyEvent, KeyPhase};

const CLASS: &str = "javax/microedition/lcdui/Gauge";

/// `Gauge.INDEFINITE`.
pub const INDEFINITE: i32 = -1;
/// `Gauge.CONTINUOUS_IDLE`.
pub const CONTINUOUS_IDLE: i32 = 0;
/// `Gauge.INCREMENTAL_IDLE`.
pub const INCREMENTAL_IDLE: i32 = 1;
/// `Gauge.CONTINUOUS_RUNNING`.
pub const CONTINUOUS_RUNNING: i32 = 2;
/// `Gauge.INCREMENTAL_UPDATING`.
pub const INCREMENTAL_UPDATING: i32 = 3;

/// How often a running gauge moves.
pub const FRAME: Duration = Duration::from_millis(80);
/// How many positions the block of an indefinite gauge passes through.
const POSITIONS: u32 = 12;

/// What a `Gauge` keeps.
#[derive(Clone, Copy, Debug, Default)]
pub struct GaugeState {
    pub interactive: bool,
    pub max: i32,
    pub value: i32,
    /// How many times `INCREMENTAL_UPDATING` was set.
    pub steps: u32,
}

impl GaugeState {
    /// A gauge as the constructor checks it.
    pub fn new(interactive: bool, max: i32, initial: i32) -> Result<Self, &'static str> {
        check_max(interactive, max)?;
        let mut gauge = Self {
            interactive,
            max,
            value: 0,
            steps: 0,
        };
        gauge.set_value(initial)?;
        Ok(gauge)
    }

    /// Whether the range is `INDEFINITE`.
    pub fn indefinite(&self) -> bool {
        self.max == INDEFINITE
    }

    /// `setValue`: clamped into a definite range; one of the four states
    /// for an indefinite one.
    pub fn set_value(&mut self, value: i32) -> Result<(), &'static str> {
        if self.indefinite() {
            if !(CONTINUOUS_IDLE..=INCREMENTAL_UPDATING).contains(&value) {
                return Err("an indefinite gauge takes one of its four states");
            }
            if value == INCREMENTAL_UPDATING {
                self.steps = self.steps.wrapping_add(1);
            }
            self.value = value;
        } else {
            self.value = value.clamp(0, self.max);
        }
        Ok(())
    }

    /// `setMaxValue`.
    pub fn set_max(&mut self, max: i32) -> Result<(), &'static str> {
        check_max(self.interactive, max)?;
        let was_indefinite = self.indefinite();
        self.max = max;
        self.value = match (was_indefinite, self.indefinite()) {
            (false, true) => CONTINUOUS_IDLE,
            (true, false) => 0,
            (true, true) => self.value,
            (false, false) => self.value.clamp(0, max),
        };
        Ok(())
    }

    /// Whether it moves by itself.
    pub fn running(&self) -> bool {
        self.indefinite() && self.value == CONTINUOUS_RUNNING
    }
}

fn check_max(interactive: bool, max: i32) -> Result<(), &'static str> {
    if max > 0 || (!interactive && max == INDEFINITE) {
        Ok(())
    } else if interactive {
        Err("an interactive gauge needs a positive maximum")
    } else {
        Err("a gauge's maximum must be positive or INDEFINITE")
    }
}

fn state(ctx: &Ctx<'_>, item: ObjRef) -> JResult<GaugeState> {
    match data(ctx, item)?.kind {
        Kind::Gauge(g) => Ok(g),
        _ => unreachable!("a Gauge's payload is made by its class"),
    }
}

fn state_mut<'a>(ctx: &'a mut Ctx<'_>, item: ObjRef) -> JResult<&'a mut GaugeState> {
    match &mut data_mut(ctx, item)?.kind {
        Kind::Gauge(g) => Ok(g),
        _ => unreachable!("a Gauge's payload is made by its class"),
    }
}

/// Whether the gauge animates by itself, so its screen's timer should
/// repaint it.
pub(crate) fn running(ctx: &Ctx<'_>, item: ObjRef) -> JResult<bool> {
    Ok(state(ctx, item)?.running())
}

/// The bar takes the whole width and a line of text's height.
pub(super) fn needs(fonts: &Fonts, max: i32) -> Needs {
    Needs {
        min_width: (fonts.text.height() * 4).min(max),
        width: max,
        height: fonts.text.height() + 4,
    }
}

/// When the process started, for the running animation's phase.
fn epoch() -> Instant {
    static EPOCH: OnceLock<Instant> = OnceLock::new();
    *EPOCH.get_or_init(Instant::now)
}

/// Draws a gauge in `area` (an item's content area, or an `Alert`'s
/// indicator).
pub(crate) fn draw(p: &mut Painter<'_>, g: GaugeState, area: Rect, focused: bool) {
    let theme = *p.theme;
    let arrows = g.interactive && focused;
    let side = if arrows { area.height } else { 0 };
    let track = Rect::new(
        area.x + side,
        area.y + 2,
        area.width - 2 * side,
        area.height - 4,
    );
    if arrows {
        let color = theme.highlighted_border;
        p.arrow_side(Rect::new(area.x, area.y, side, area.height), true, color);
        let right = Rect::new(area.x + area.width - side, area.y, side, area.height);
        p.arrow_side(right, false, color);
    }
    let pale = draw::blend(theme.background, theme.border);
    p.round(track, draw::blend(theme.background, pale));
    p.frame(track, theme.border, 1);
    let inner = Rect::new(track.x + 2, track.y + 2, track.width - 4, track.height - 4);
    if g.indefinite() {
        let block = (inner.width / 4).max(4);
        let position = match g.value {
            CONTINUOUS_RUNNING => {
                let ticks = epoch().elapsed().as_millis() / FRAME.as_millis();
                (ticks % u128::from(POSITIONS)) as u32
            }
            INCREMENTAL_UPDATING => g.steps % POSITIONS,
            _ => 0,
        };
        let room = inner.width - block;
        let x = inner.x + room * position as i32 / (POSITIONS - 1) as i32;
        let color = if g.value == CONTINUOUS_RUNNING || g.value == INCREMENTAL_UPDATING {
            theme.highlighted_background
        } else {
            pale
        };
        p.fill(Rect::new(x, inner.y, block, inner.height), color);
    } else {
        let filled = (i64::from(inner.width) * i64::from(g.value) / i64::from(g.max.max(1))) as i32;
        p.fill(
            Rect::new(inner.x, inner.y, filled, inner.height),
            theme.highlighted_background,
        );
    }
}

pub(super) fn paint(
    ctx: &Ctx<'_>,
    item: ObjRef,
    p: &mut Painter<'_>,
    area: Rect,
    focused: bool,
) -> JResult<()> {
    draw(p, state(ctx, item)?, area, focused);
    Ok(())
}

/// How far one press of left or right moves an interactive gauge.
fn step(max: i32) -> i32 {
    (max / 20).max(1)
}

/// Left and right change an interactive gauge's value.
pub(super) fn key(ctx: &mut Ctx<'_>, item: ObjRef, event: KeyEvent) -> JResult<Handled> {
    let g = state(ctx, item)?;
    if !g.interactive || event.phase == KeyPhase::Released {
        return Ok(Handled::No);
    }
    let delta = match event.game_action {
        keys::LEFT => -step(g.max),
        keys::RIGHT => step(g.max),
        _ => return Ok(Handled::No),
    };
    let next = (g.value + delta).clamp(0, g.max);
    if next == g.value {
        return Ok(Handled::Yes);
    }
    state_mut(ctx, item)?.value = next;
    Ok(Handled::Changed)
}

/// A press or drag on an interactive gauge sets its value where it lands,
/// `x` pixels into a content area `area` wide and `height` tall.
pub(super) fn pointer(
    ctx: &mut Ctx<'_>,
    item: ObjRef,
    action: PointerAction,
    x: i32,
    (width, height): (i32, i32),
) -> JResult<Handled> {
    let g = state(ctx, item)?;
    if !g.interactive || action == PointerAction::Release {
        return Ok(Handled::No);
    }
    let side = height;
    let track = (width - 2 * side).max(1);
    let value = (i64::from(x - side) * i64::from(g.max) / i64::from(track)) as i32;
    let value = value.clamp(0, g.max);
    if value == g.value {
        return Ok(Handled::Yes);
    }
    state_mut(ctx, item)?.value = value;
    Ok(Handled::Changed)
}

fn illegal(ctx: &mut Ctx<'_>, why: &str) -> fastemu_jvm::Throw {
    ctx.new_exception("java/lang/IllegalArgumentException", Some(why))
}

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    NativeClass::new(CLASS)
        .extends(super::ITEM)
        .payload_with(|| Box::new(ItemData::new(Kind::Gauge(GaugeState::default()))))
        .constant("INDEFINITE", "I", int(INDEFINITE))
        .constant("CONTINUOUS_IDLE", "I", int(CONTINUOUS_IDLE))
        .constant("INCREMENTAL_IDLE", "I", int(INCREMENTAL_IDLE))
        .constant("CONTINUOUS_RUNNING", "I", int(CONTINUOUS_RUNNING))
        .constant("INCREMENTAL_UPDATING", "I", int(INCREMENTAL_UPDATING))
        .method("<init>", "(Ljava/lang/String;ZII)V", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            let g = match GaugeState::new(a[2].boolean(), a[3].int(), a[4].int()) {
                Ok(g) => g,
                Err(why) => return Err(illegal(ctx, why)),
            };
            let d = data_mut(ctx, item)?;
            d.label = a[1].obj();
            d.kind = Kind::Gauge(g);
            Ok(Value::Void)
        })
        .method("setValue", "(I)V", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            if let Err(why) = state_mut(ctx, item)?.set_value(a[1].int()) {
                return Err(illegal(ctx, why));
            }
            changed(ctx, item)?;
            Ok(Value::Void)
        })
        .method("getValue", "()I", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            Ok(Value::Int(state(ctx, item)?.value))
        })
        .method("setMaxValue", "(I)V", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            if let Err(why) = state_mut(ctx, item)?.set_max(a[1].int()) {
                return Err(illegal(ctx, why));
            }
            changed(ctx, item)?;
            Ok(Value::Void)
        })
        .method("getMaxValue", "()I", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            Ok(Value::Int(state(ctx, item)?.max))
        })
        .method("isInteractive", "()Z", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            Ok(Value::from(state(ctx, item)?.interactive))
        })
        .method("setLabel", "(Ljava/lang/String;)V", super::set_label)
        .method("setLayout", "(I)V", super::set_layout)
        .method(
            "addCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            super::add_command,
        )
        .method(
            "setItemCommandListener",
            "(Ljavax/microedition/lcdui/ItemCommandListener;)V",
            super::set_listener,
        )
        .method("setPreferredSize", "(II)V", super::set_preferred_size)
        .method(
            "setDefaultCommand",
            "(Ljavax/microedition/lcdui/Command;)V",
            super::set_default_command,
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn construction_checks_the_range() {
        assert!(GaugeState::new(true, 0, 0).is_err());
        assert!(GaugeState::new(true, INDEFINITE, 0).is_err());
        assert!(GaugeState::new(false, -5, 0).is_err());
        assert!(GaugeState::new(false, INDEFINITE, 7).is_err());
        let g = GaugeState::new(false, INDEFINITE, CONTINUOUS_RUNNING).unwrap();
        assert!(g.running());
        let g = GaugeState::new(true, 10, 50).unwrap();
        assert_eq!(g.value, 10, "values clamp into the range");
    }

    #[test]
    fn incremental_updating_counts_steps() {
        let mut g = GaugeState::new(false, INDEFINITE, INCREMENTAL_IDLE).unwrap();
        g.set_value(INCREMENTAL_UPDATING).unwrap();
        g.set_value(INCREMENTAL_UPDATING).unwrap();
        assert_eq!((g.value, g.steps), (INCREMENTAL_UPDATING, 2));
        assert!(g.set_value(9).is_err());
    }

    #[test]
    fn changing_the_range_resets_the_value() {
        let mut g = GaugeState::new(false, 10, 7).unwrap();
        g.set_max(5).unwrap();
        assert_eq!(g.value, 5);
        g.set_max(INDEFINITE).unwrap();
        assert_eq!(g.value, CONTINUOUS_IDLE);
        g.set_max(8).unwrap();
        assert_eq!(g.value, 0);
        let mut i = GaugeState::new(true, 10, 3).unwrap();
        assert!(i.set_max(INDEFINITE).is_err());
    }
}
