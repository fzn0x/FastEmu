//! `javax.microedition.lcdui.Ticker`: text that scrolls across the top of
//! a screen.
//!
//! The runtime draws the ticker line (`lcdui::frame`); this module keeps
//! the text and says where it is. The text moves left at a steady speed
//! from the moment the ticker is made or given new text, so a ticker set
//! on several screens runs on as the user moves between them. While a
//! screen with a ticker is shown, its timer presents a new frame every
//! [`STEP`].

use std::time::{Duration, Instant};

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Payload, Tracer, Value};

const CLASS: &str = "javax/microedition/lcdui/Ticker";

/// How far the text moves each second, in pixels.
const SPEED: u128 = 40;
/// How often a shown ticker moves.
pub const STEP: Duration = Duration::from_millis(50);
/// The gap after the text before it comes round again, in pixels.
const GAP: i32 = 24;

/// What a Java `Ticker` holds.
pub struct TickerData {
    text: Option<ObjRef>,
    since: Instant,
}

impl Default for TickerData {
    fn default() -> Self {
        Self {
            text: None,
            since: Instant::now(),
        }
    }
}

impl Payload for TickerData {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.text);
    }
}

/// Where the ticker's text starts in a line `width` pixels wide, for text
/// `text_width` pixels wide, at `now`: it enters at the right edge and
/// leaves at the left, then comes round again.
pub fn scroll_x(since: Instant, now: Instant, width: i32, text_width: i32) -> i32 {
    let lap = i64::from(width.max(1)) + i64::from(text_width.max(0)) + i64::from(GAP);
    let travelled = now.saturating_duration_since(since).as_millis() * SPEED / 1000;
    let into = (travelled % lap as u128) as i64;
    (i64::from(width) - into) as i32
}

/// Where the text of `ticker` starts now, for the runtime's ticker line.
/// A game's own `Ticker` subclass without the payload scrolls from when
/// the instance started.
pub(crate) fn position(ctx: &Ctx<'_>, ticker: ObjRef, width: i32, text_width: i32) -> i32 {
    let since = ctx
        .try_payload::<TickerData>(ticker)
        .map_or_else(Instant::now, |t| t.since);
    scroll_x(since, Instant::now(), width, text_width)
}

fn set_text(ctx: &mut Ctx<'_>, this: Value, text: Value) -> JResult<Value> {
    let this = ctx.non_null(this)?;
    let text = ctx.non_null(text)?;
    let data = ctx.payload_mut::<TickerData>(this)?;
    data.text = Some(text);
    data.since = Instant::now();
    Ok(Value::Void)
}

/// The class.
pub fn class() -> NativeClass {
    NativeClass::new(CLASS)
        .payload::<TickerData>()
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            set_text(ctx, a[0], a[1])
        })
        .method("setString", "(Ljava/lang/String;)V", |ctx, a| {
            set_text(ctx, a[0], a[1])
        })
        .method("getString", "()Ljava/lang/String;", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            Ok(ctx.payload::<TickerData>(this)?.text.into())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_enters_at_the_right_and_comes_round() {
        let start = Instant::now();
        assert_eq!(scroll_x(start, start, 100, 50), 100);
        let one_second = start + Duration::from_secs(1);
        assert_eq!(scroll_x(start, one_second, 100, 50), 100 - SPEED as i32);
        let lap = (100 + 50 + GAP) as u64 * 1000 / SPEED as u64;
        let round = start + Duration::from_millis(lap);
        assert!(scroll_x(start, round, 100, 50) >= 99);
    }
}
