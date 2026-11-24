//! `javax.microedition.lcdui.DateField`: a date, a time, or both, edited
//! a field at a time: left and right pick the day, month, year, hour or
//! minute, digits type it, and select sets an empty field to now.
//!
//! Dates are milliseconds since 1970 in UTC, as `java.util.Date` holds
//! them; the fields show them in the field's time zone (its raw offset,
//! read once from the `TimeZone`).

use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::{Handled, ItemData, Kind, Needs, changed, data, data_mut};
use crate::lcdui::Rect;
use crate::lcdui::highlevel::draw::{Fonts, Painter, width};
use crate::lcdui::highlevel::text::INSET;
use crate::lcdui::keys;
use crate::lcdui::screen::{KeyEvent, KeyPhase};

const CLASS: &str = "javax/microedition/lcdui/DateField";

/// `DateField.DATE`.
pub const DATE: i32 = 1;
/// `DateField.TIME`.
pub const TIME: i32 = 2;
/// `DateField.DATE_TIME`.
pub const DATE_TIME: i32 = 3;

const DAY_MS: i64 = 86_400_000;

/// One editable part of the shown date.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Day,
    Month,
    Year,
    Hour,
    Minute,
}

impl Part {
    fn digits(self) -> usize {
        if self == Part::Year { 4 } else { 2 }
    }
}

fn parts(mode: i32) -> &'static [Part] {
    match mode {
        DATE => &[Part::Day, Part::Month, Part::Year],
        TIME => &[Part::Hour, Part::Minute],
        _ => &[Part::Day, Part::Month, Part::Year, Part::Hour, Part::Minute],
    }
}

/// The civil date of a day count since 1970-01-01 (Howard Hinnant's
/// algorithm, proleptic Gregorian).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

/// The day count since 1970-01-01 of a civil date.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let m = i64::from(month);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The local fields of a moment.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fields {
    year: i64,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    /// Seconds and milliseconds, kept as they were.
    rest: i64,
}

impl Fields {
    fn of(millis: i64, offset: i64) -> Self {
        let local = millis + offset;
        let (year, month, day) = civil_from_days(local.div_euclid(DAY_MS));
        let in_day = local.rem_euclid(DAY_MS);
        Self {
            year,
            month,
            day,
            hour: (in_day / 3_600_000) as u32,
            minute: (in_day / 60_000 % 60) as u32,
            rest: in_day % 60_000,
        }
    }

    fn millis(self, offset: i64) -> i64 {
        let day = self.day.clamp(1, days_in_month(self.year, self.month));
        let days = days_from_civil(self.year, self.month, day);
        days * DAY_MS
            + i64::from(self.hour) * 3_600_000
            + i64::from(self.minute) * 60_000
            + self.rest
            - offset
    }

    fn get(self, part: Part) -> i64 {
        match part {
            Part::Day => self.day.into(),
            Part::Month => self.month.into(),
            Part::Year => self.year,
            Part::Hour => self.hour.into(),
            Part::Minute => self.minute.into(),
        }
    }

    /// Sets `part` to `value` kept in its range.
    fn set(&mut self, part: Part, value: i64) {
        match part {
            Part::Day => self.day = value.clamp(1, 31) as u32,
            Part::Month => self.month = value.clamp(1, 12) as u32,
            Part::Year => self.year = value.clamp(1, 9999),
            Part::Hour => self.hour = value.clamp(0, 23) as u32,
            Part::Minute => self.minute = value.clamp(0, 59) as u32,
        }
    }
}

/// What a `DateField` keeps.
#[derive(Clone, Copy, Debug)]
pub struct DateState {
    pub mode: i32,
    /// The date, or `None` before one is set.
    pub millis: Option<i64>,
    /// The time zone's offset from UTC in milliseconds.
    pub offset: i64,
    /// The part being edited, an index into the mode's parts.
    pub part: usize,
    /// How many digits were typed into that part so far.
    typed: usize,
    /// Their value: a part's field is clamped, so a month typed as `0`
    /// then `2` needs the digits themselves.
    entry: i64,
}

impl Default for DateState {
    fn default() -> Self {
        Self {
            mode: DATE,
            millis: None,
            offset: 0,
            part: 0,
            typed: 0,
            entry: 0,
        }
    }
}

impl DateState {
    /// `millis` with the parts the mode does not show reset: midnight for
    /// `DATE`, 1970-01-01 for `TIME`.
    fn normalize(&self, millis: i64) -> i64 {
        let local = millis + self.offset;
        match self.mode {
            DATE => local.div_euclid(DAY_MS) * DAY_MS - self.offset,
            TIME => local.rem_euclid(DAY_MS) - self.offset,
            _ => millis,
        }
    }

    /// Sets the date, as `setDate` does.
    pub fn set(&mut self, millis: Option<i64>) {
        self.millis = millis.map(|m| self.normalize(m));
        self.typed = 0;
    }

    /// `setInputMode`.
    pub fn set_mode(&mut self, mode: i32) {
        self.mode = mode;
        self.part = 0;
        self.typed = 0;
        self.millis = self.millis.map(|m| self.normalize(m));
    }

    /// The text shown, and the range of the part being edited.
    fn text(&self) -> (String, std::ops::Range<usize>) {
        let fields = self.millis.map(|m| Fields::of(m, self.offset));
        let mut out = String::new();
        let mut focus = 0..0;
        for (i, &part) in parts(self.mode).iter().enumerate() {
            let sep = match part {
                Part::Month | Part::Year => "/",
                Part::Hour if i > 0 => " ",
                Part::Minute => ":",
                _ => "",
            };
            out.push_str(sep);
            let start = out.len();
            match fields {
                Some(f) => {
                    let _ = write!(out, "{:0w$}", f.get(part), w = part.digits());
                }
                None => out.push_str(&"-".repeat(part.digits())),
            }
            if i == self.part {
                focus = start..out.len();
            }
        }
        (out, focus)
    }

    /// One digit typed into the current part. Returns whether the date
    /// changed.
    fn digit(&mut self, digit: u8, now: i64) -> bool {
        let part = parts(self.mode)[self.part.min(parts(self.mode).len() - 1)];
        let mut fields = Fields::of(self.millis.unwrap_or(self.normalize(now)), self.offset);
        self.entry = if self.typed == 0 {
            i64::from(digit)
        } else {
            self.entry * 10 + i64::from(digit)
        };
        fields.set(part, self.entry);
        self.typed += 1;
        if self.typed >= part.digits() {
            self.typed = 0;
            self.part = (self.part + 1).min(parts(self.mode).len() - 1);
        }
        let next = self.normalize(fields.millis(self.offset));
        let changed = self.millis != Some(next);
        self.millis = Some(next);
        changed
    }

    /// Moves to the previous (`-1`) or next part. Returns whether it moved.
    fn step(&mut self, delta: i32) -> bool {
        let last = parts(self.mode).len() as i32 - 1;
        let next = (self.part as i32 + delta).clamp(0, last) as usize;
        self.typed = 0;
        let moved = next != self.part;
        self.part = next;
        moved
    }
}

/// Milliseconds since 1970 now, by the host's clock.
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

fn state(ctx: &Ctx<'_>, item: ObjRef) -> JResult<DateState> {
    match data(ctx, item)?.kind {
        Kind::Date(s) => Ok(s),
        _ => unreachable!("a DateField's payload is made by its class"),
    }
}

fn state_mut<'a>(ctx: &'a mut Ctx<'_>, item: ObjRef) -> JResult<&'a mut DateState> {
    match &mut data_mut(ctx, item)?.kind {
        Kind::Date(s) => Ok(s),
        _ => unreachable!("a DateField's payload is made by its class"),
    }
}

/// One line in a frame.
pub(super) fn needs(ctx: &mut Ctx<'_>, item: ObjRef, fonts: &Fonts, max: i32) -> JResult<Needs> {
    let (text, _) = state(ctx, item)?.text();
    let units: Vec<u16> = text.encode_utf16().collect();
    let w = (width(&fonts.text, &units) + 2 * INSET + 2).min(max);
    Ok(Needs {
        min_width: w,
        width: max,
        height: fonts.text.height() + 2 * INSET,
    })
}

pub(super) fn paint(
    ctx: &Ctx<'_>,
    item: ObjRef,
    p: &mut Painter<'_>,
    area: Rect,
    focused: bool,
) -> JResult<()> {
    let s = state(ctx, item)?;
    let theme = *p.theme;
    let (text, focus) = s.text();
    let font = p.fonts.text.clone();
    p.fill(area, theme.background);
    let (border, thickness) = if focused {
        (theme.highlighted_border, 2)
    } else {
        (theme.border, 1)
    };
    p.frame(area, border, thickness);
    let units: Vec<u16> = text.encode_utf16().collect();
    let (x, y) = (area.x + INSET, area.y + INSET);
    if focused {
        let left = x + width(&font, &units[..focus.start]);
        let w = width(&font, &units[focus.clone()]);
        let mark = Rect::new(left - 1, y, w + 2, font.height());
        p.fill(mark, theme.highlighted_background);
        p.text(&font, &units[..focus.start], x, y, theme.foreground);
        p.text(
            &font,
            &units[focus.clone()],
            left,
            y,
            theme.highlighted_foreground,
        );
        let after = left + w;
        p.text(&font, &units[focus.end..], after, y, theme.foreground);
    } else {
        p.text(&font, &units, x, y, theme.foreground);
    }
    Ok(())
}

/// Digits type into the part being edited, left and right pick another
/// part, and select sets an empty field to now.
pub(super) fn key(ctx: &mut Ctx<'_>, item: ObjRef, event: KeyEvent) -> JResult<Handled> {
    if event.phase == KeyPhase::Released {
        return Ok(Handled::No);
    }
    let codes = crate::env::env(ctx).profile.key_codes;
    let s = state_mut(ctx, item)?;
    if let Ok(digit @ b'0'..=b'9') = u8::try_from(event.code) {
        return Ok(if s.digit(digit - b'0', now()) {
            Handled::Changed
        } else {
            Handled::Yes
        });
    }
    if event.code == codes.left || event.code == codes.right {
        let delta = if event.code == codes.left { -1 } else { 1 };
        return Ok(if s.step(delta) {
            Handled::Yes
        } else {
            Handled::No
        });
    }
    if event.game_action == keys::FIRE && event.phase == KeyPhase::Pressed && s.millis.is_none() {
        let at = now();
        s.set(Some(at));
        return Ok(Handled::Changed);
    }
    Ok(Handled::No)
}

/// Digits typed on a host keyboard.
pub(super) fn text(ctx: &mut Ctx<'_>, item: ObjRef, typed: &str) -> JResult<Handled> {
    let s = state_mut(ctx, item)?;
    let mut changed = false;
    for digit in typed.bytes().filter(u8::is_ascii_digit) {
        changed |= s.digit(digit - b'0', now());
    }
    Ok(if changed {
        Handled::Changed
    } else {
        Handled::No
    })
}

/// The raw offset of `zone`, a `java.util.TimeZone`, or of the default
/// zone when it is null.
fn zone_offset(ctx: &mut Ctx<'_>, zone: Option<ObjRef>) -> JResult<i64> {
    let zone = match zone {
        Some(zone) => zone,
        None => {
            let zone = ctx.call_static(
                "java/util/TimeZone",
                "getDefault",
                "()Ljava/util/TimeZone;",
                &[],
            )?;
            match zone.obj() {
                Some(zone) => zone,
                None => return Ok(0),
            }
        }
    };
    let offset = ctx.call_virtual(zone, "getRawOffset", "()I", &[])?;
    Ok(i64::from(offset.int()))
}

fn init(ctx: &mut Ctx<'_>, a: &[Value], zone: Option<ObjRef>) -> JResult<Value> {
    let mode = a[2].int();
    if !(DATE..=DATE_TIME).contains(&mode) {
        return ctx.throw("java/lang/IllegalArgumentException", "invalid input mode");
    }
    let item = ctx.non_null(a[0])?;
    let offset = zone_offset(ctx, zone)?;
    let d = data_mut(ctx, item)?;
    d.label = a[1].obj();
    d.kind = Kind::Date(DateState {
        mode,
        offset,
        ..DateState::default()
    });
    Ok(Value::Void)
}

/// The class.
pub fn class() -> NativeClass {
    let int = |v| Value::Int(v);
    NativeClass::new(CLASS)
        .extends(super::ITEM)
        .payload_with(|| Box::new(ItemData::new(Kind::Date(DateState::default()))))
        .constant("DATE", "I", int(DATE))
        .constant("TIME", "I", int(TIME))
        .constant("DATE_TIME", "I", int(DATE_TIME))
        .method("<init>", "(Ljava/lang/String;I)V", |ctx, a| {
            init(ctx, a, None)
        })
        .method(
            "<init>",
            "(Ljava/lang/String;ILjava/util/TimeZone;)V",
            |ctx, a| init(ctx, a, a[3].obj()),
        )
        .method("getDate", "()Ljava/util/Date;", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            match state(ctx, item)?.millis {
                Some(ms) => Ok(ctx
                    .construct("java/util/Date", "(J)V", &[Value::Long(ms)])?
                    .into()),
                None => Ok(Value::NULL),
            }
        })
        .method("setDate", "(Ljava/util/Date;)V", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            let millis = match a[1].obj() {
                Some(date) => Some(ctx.call_virtual(date, "getTime", "()J", &[])?.long()),
                None => None,
            };
            state_mut(ctx, item)?.set(millis);
            changed(ctx, item)?;
            Ok(Value::Void)
        })
        .method("getInputMode", "()I", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            Ok(Value::Int(state(ctx, item)?.mode))
        })
        .method("setInputMode", "(I)V", |ctx, a| {
            let item = ctx.non_null(a[0])?;
            let mode = a[1].int();
            if !(DATE..=DATE_TIME).contains(&mode) {
                return ctx.throw("java/lang/IllegalArgumentException", "invalid input mode");
            }
            state_mut(ctx, item)?.set_mode(mode);
            changed(ctx, item)?;
            Ok(Value::Void)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_and_civil_dates_agree() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(days_from_civil(2000, 2, 29), 11_016);
        assert_eq!(civil_from_days(11_016), (2000, 2, 29));
        assert_eq!(civil_from_days(-1), (1969, 12, 31));
        for days in [-800_000, -1, 0, 59, 365, 19_000, 2_932_896] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
    }

    #[test]
    fn the_mode_drops_the_parts_it_hides() {
        let moment = 1_000_000_000_123;
        let mut s = DateState::default();
        s.set(Some(moment));
        assert_eq!(s.millis, Some(moment / DAY_MS * DAY_MS));
        s.set_mode(TIME);
        assert_eq!(s.millis, Some(0));
        let mut t = DateState {
            mode: TIME,
            offset: 3_600_000,
            ..DateState::default()
        };
        t.set(Some(moment));
        let local = Fields::of(t.millis.unwrap(), t.offset);
        assert_eq!((local.year, local.month, local.day), (1970, 1, 1));
        assert_eq!(local.hour, Fields::of(moment, 3_600_000).hour);
    }

    #[test]
    fn digits_fill_a_part_and_move_on() {
        let mut s = DateState::default();
        s.set(Some(0));
        assert_eq!(s.text().0, "01/01/1970");
        s.digit(2, 0);
        s.digit(5, 0);
        assert_eq!((s.text().0.as_str(), s.part), ("25/01/1970", 1));
        s.digit(1, 0);
        s.digit(2, 0);
        for d in [2, 0, 2, 6] {
            s.digit(d, 0);
        }
        assert_eq!(s.text().0, "25/12/2026");
        assert!(s.step(-1));
        assert!(!s.step(-5) || s.part == 0);
    }

    #[test]
    fn an_empty_field_shows_dashes() {
        let s = DateState {
            mode: DATE_TIME,
            ..DateState::default()
        };
        let (text, focus) = s.text();
        assert_eq!(text, "--/--/---- --:--");
        assert_eq!(focus, 0..2);
    }

    #[test]
    fn the_day_is_kept_inside_its_month() {
        let mut s = DateState::default();
        let jan31 = days_from_civil(2025, 1, 31) * DAY_MS;
        s.set(Some(jan31));
        s.part = 1;
        s.digit(0, 0);
        s.digit(2, 0);
        assert_eq!(s.text().0, "28/02/2025");
    }
}
