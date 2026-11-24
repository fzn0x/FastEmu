//! `java.util.TimeZone`: GMT and UTC, custom `GMT+hh:mm` zones, and the
//! host's own zone as the default, whose daylight saving time comes from
//! the host. As CLDC says, an ID this virtual machine does not know gives
//! GMT.
//!
//! The default zone's ID is the `GMT+hh:mm` form of the host's standard
//! offset (`GMT` for a host on UTC without daylight saving time), so the
//! host's own zone name never reaches the game.

use super::time::{
    self, civil_from_days, custom_id, day_of_week, host_offset_at, host_offset_for_local,
    host_raw_offset, host_uses_daylight_time, month_length,
};
use crate::ctx::{Ctx, JResult};
use crate::heap::Payload;
use crate::lang::{string_value, this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const TIMEZONE: &str = "java/util/TimeZone";
const IMPL: &str = "java/util/TimeZone$Impl";

/// How a zone of this library computes its offsets.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Rule {
    /// The same offset all year.
    Fixed(i64),
    /// The host's zone.
    Host,
}

/// What a `TimeZone` keeps: its ID and rule, both `None` for a game's
/// own subclass.
#[derive(Default)]
pub(super) struct Zone {
    id: Option<String>,
    rule: Option<Rule>,
}

impl Payload for Zone {}

fn rule(ctx: &Ctx<'_>, zone: ObjRef) -> JResult<Option<Rule>> {
    Ok(ctx.payload::<Zone>(zone)?.rule)
}

/// The ID of the host's zone.
fn default_id() -> String {
    let raw = host_raw_offset();
    if raw == 0 && !host_uses_daylight_time() {
        "GMT".to_owned()
    } else {
        custom_id(raw)
    }
}

fn new_zone(ctx: &mut Ctx<'_>, id: String, rule: Rule) -> JResult<ObjRef> {
    let class = ctx.find_class(IMPL)?;
    let zone = ctx.new_object(class)?;
    ctx.set_payload(
        zone,
        Zone {
            id: Some(id),
            rule: Some(rule),
        },
    );
    Ok(zone)
}

/// `TimeZone.getDefault()`: the host's zone.
pub(super) fn default_zone(ctx: &mut Ctx<'_>) -> JResult<ObjRef> {
    new_zone(ctx, default_id(), Rule::Host)
}

/// `TimeZone.getTimeZone(id)`.
fn zone_named(ctx: &mut Ctx<'_>, id: &str) -> JResult<ObjRef> {
    match id {
        "GMT" | "UTC" => new_zone(ctx, id.to_owned(), Rule::Fixed(0)),
        _ if id == default_id() => default_zone(ctx),
        _ => match time::parse_custom_id(id) {
            Some(offset) => new_zone(ctx, custom_id(offset), Rule::Fixed(offset)),
            None => new_zone(ctx, "GMT".to_owned(), Rule::Fixed(0)),
        },
    }
}

/// The offset from UTC of `zone` at the instant `utc`. A game's own zone
/// is asked through its `getRawOffset` and `getOffset`, with the date in
/// standard time, as Java's calendar asks it.
pub(super) fn offset_at(ctx: &mut Ctx<'_>, zone: ObjRef, utc: i64) -> JResult<i64> {
    match rule(ctx, zone)? {
        Some(Rule::Fixed(offset)) => Ok(offset),
        Some(Rule::Host) => Ok(host_offset_at(utc)),
        None => {
            let raw = i64::from(ctx.call_virtual(zone, "getRawOffset", "()I", &[])?.int());
            let standard = utc + raw;
            let days = standard.div_euclid(time::DAY_MS);
            let (year, month, day) = civil_from_days(days);
            let (era, year) = if year > 0 { (1, year) } else { (0, 1 - year) };
            let args = [
                Value::Int(era),
                Value::Int(year as i32),
                Value::Int(month as i32 - 1),
                Value::Int(day as i32),
                Value::Int(day_of_week(days) as i32),
                Value::Int(standard.rem_euclid(time::DAY_MS) as i32),
            ];
            let offset = ctx.call_virtual(zone, "getOffset", "(IIIIII)I", &args)?;
            Ok(i64::from(offset.int()))
        }
    }
}

/// The offset of `zone` to take from the wall-clock time `local` to get
/// UTC.
pub(super) fn offset_for_local(ctx: &mut Ctx<'_>, zone: ObjRef, local: i64) -> JResult<i64> {
    match rule(ctx, zone)? {
        Some(Rule::Fixed(offset)) => Ok(offset),
        Some(Rule::Host) => Ok(host_offset_for_local(local)),
        None => {
            let raw = i64::from(ctx.call_virtual(zone, "getRawOffset", "()I", &[])?.int());
            offset_at(ctx, zone, local - raw)
        }
    }
}

/// The zone's ID, or `None` for a game's zone that has none.
pub(super) fn id_of(ctx: &mut Ctx<'_>, zone: ObjRef) -> JResult<Option<String>> {
    if rule(ctx, zone)?.is_some() {
        return Ok(ctx.payload::<Zone>(zone)?.id.clone());
    }
    let id = ctx.call_virtual(zone, "getID", "()Ljava/lang/String;", &[])?;
    ctx.opt_string(id)
}

/// `getOffset(era, year, month, day, dayOfWeek, millis)` with Java's
/// argument checks.
fn get_offset(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let (era, year, month, day, dow, millis) = (
        a[1].int(),
        i64::from(a[2].int()),
        a[3].int(),
        i64::from(a[4].int()),
        a[5].int(),
        a[6].int(),
    );
    let bad = if !(0..=1).contains(&era) {
        Some(format!("Illegal era {era}"))
    } else if !(0..=11).contains(&month) {
        Some(format!("Illegal month {month}"))
    } else if day < 1 || day > month_length(year, i64::from(month)) {
        Some(format!("Illegal day {day}"))
    } else if !(1..=7).contains(&dow) {
        Some(format!("Illegal day of week {dow}"))
    } else if !(0..time::DAY_MS as i32).contains(&millis) {
        Some(format!("Illegal millis {millis}"))
    } else {
        None
    };
    if let Some(message) = bad {
        return throw(ctx, "java/lang/IllegalArgumentException", &message);
    }
    let rule = rule(ctx, this(a))?.expect("TimeZone$Impl has a rule");
    let offset = match rule {
        Rule::Fixed(offset) => offset,
        Rule::Host => {
            let year = if era == 0 { 1 - year } else { year };
            let days = time::days_from_civil(year, i64::from(month) + 1, day);
            let standard = days * time::DAY_MS + i64::from(millis);
            host_offset_at(standard - host_raw_offset())
        }
    };
    Ok(Value::Int(offset as i32))
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new(TIMEZONE)
            .abstract_class()
            .payload::<Zone>()
            .method("<init>", "()V", |_, _| Ok(Value::Void))
            .abstract_method("getOffset", "(IIIIII)I")
            .abstract_method("getRawOffset", "()I")
            .abstract_method("useDaylightTime", "()Z")
            .method("getID", "()Ljava/lang/String;", |ctx, a| {
                match ctx.payload::<Zone>(this(a))?.id.clone() {
                    Some(id) => string_value(ctx, &id),
                    None => Ok(Value::NULL),
                }
            })
            .static_method(
                "getTimeZone",
                "(Ljava/lang/String;)Ljava/util/TimeZone;",
                |ctx, a| {
                    let id = ctx.non_null(a[0])?;
                    let id = ctx.string(id)?;
                    Ok(zone_named(ctx, &id)?.into())
                },
            )
            .static_method("getDefault", "()Ljava/util/TimeZone;", |ctx, _| {
                Ok(default_zone(ctx)?.into())
            })
            .static_method("getAvailableIDs", "()[Ljava/lang/String;", |ctx, _| {
                let mut ids = vec!["GMT".to_owned(), "UTC".to_owned()];
                let host = default_id();
                if !ids.contains(&host) {
                    ids.push(host);
                }
                let string = ctx.find_class("java/lang/String")?;
                let arr = ctx.new_object_array(string, ids.len() as i32)?;
                for (i, id) in ids.iter().enumerate() {
                    let s = ctx.new_string(id)?;
                    ctx.array_mut::<Option<ObjRef>>(arr)?[i] = Some(s);
                }
                Ok(arr.into())
            }),
        NativeClass::new(IMPL)
            .extends(TIMEZONE)
            .final_class()
            .method("getOffset", "(IIIIII)I", get_offset)
            .method("getRawOffset", "()I", |ctx, a| {
                let offset = match rule(ctx, this(a))? {
                    Some(Rule::Fixed(offset)) => offset,
                    _ => host_raw_offset(),
                };
                Ok(Value::Int(offset as i32))
            })
            .method("useDaylightTime", "()Z", |ctx, a| {
                let host = rule(ctx, this(a))? == Some(Rule::Host);
                Ok(Value::from(host && host_uses_daylight_time()))
            }),
    ]
}
