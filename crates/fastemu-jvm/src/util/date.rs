//! `java.util.Date`: an instant in milliseconds since 1970, which
//! `toString` writes in the default time zone as Java does.

use super::time::{DAY_MS, civil_from_days, day_of_week};
use super::timezone;
use crate::ctx::{Ctx, JResult};
use crate::heap::Payload;
use crate::lang::system::current_time_millis;
use crate::lang::{string_value, this};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const DATE: &str = "java/util/Date";

/// The instant a `Date` stands for.
#[derive(Default)]
struct Instant(i64);

impl Payload for Instant {}

/// The milliseconds of the `Date` `date`.
pub(super) fn millis_of(ctx: &Ctx<'_>, date: ObjRef) -> JResult<i64> {
    Ok(ctx.payload::<Instant>(date)?.0)
}

/// A new `Date` for `millis`.
pub(super) fn new_date(ctx: &mut Ctx<'_>, millis: i64) -> JResult<ObjRef> {
    ctx.construct(DATE, "(J)V", &[Value::Long(millis)])
}

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// `EEE MMM dd HH:mm:ss zzz yyyy` for the wall-clock time `local` in the
/// zone named `zone`, as `Date.toString` writes it.
pub(super) fn format(local: i64, zone: &str) -> String {
    let days = local.div_euclid(DAY_MS);
    let (year, month, day) = civil_from_days(days);
    let ms = local.rem_euclid(DAY_MS) / 1000;
    format!(
        "{} {} {day:02} {:02}:{:02}:{:02} {zone} {year}",
        DAYS[(day_of_week(days) - 1) as usize],
        MONTHS[(month - 1) as usize],
        ms / 3600,
        ms / 60 % 60,
        ms % 60,
    )
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new(DATE)
            .payload::<Instant>()
            .method("<init>", "()V", |ctx, a| {
                ctx.payload_mut::<Instant>(this(a))?.0 = current_time_millis();
                Ok(Value::Void)
            })
            .method("<init>", "(J)V", |ctx, a| {
                ctx.payload_mut::<Instant>(this(a))?.0 = a[1].long();
                Ok(Value::Void)
            })
            .method("getTime", "()J", |ctx, a| {
                Ok(Value::Long(millis_of(ctx, this(a))?))
            })
            .method("setTime", "(J)V", |ctx, a| {
                ctx.payload_mut::<Instant>(this(a))?.0 = a[1].long();
                Ok(Value::Void)
            })
            .method("equals", "(Ljava/lang/Object;)Z", |ctx, a| {
                let date = ctx.find_class(DATE)?;
                let Some(other) = a[1].obj().filter(|&o| ctx.is_instance(o, date)) else {
                    return Ok(Value::from(false));
                };
                let mine = ctx.call_virtual(this(a), "getTime", "()J", &[])?.long();
                let theirs = ctx.call_virtual(other, "getTime", "()J", &[])?.long();
                Ok(Value::from(mine == theirs))
            })
            .method("hashCode", "()I", |ctx, a| {
                let t = ctx.call_virtual(this(a), "getTime", "()J", &[])?.long();
                Ok(Value::Int((t ^ (t >> 32)) as i32))
            })
            .method("toString", "()Ljava/lang/String;", |ctx, a| {
                let t = ctx.call_virtual(this(a), "getTime", "()J", &[])?.long();
                let zone = timezone::default_zone(ctx)?;
                let offset = timezone::offset_at(ctx, zone, t)?;
                let id = timezone::id_of(ctx, zone)?.unwrap_or_else(|| "GMT".to_owned());
                string_value(ctx, &format(t + offset, &id))
            }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_print_as_java_prints_them() {
        assert_eq!(format(0, "GMT"), "Thu Jan 01 00:00:00 GMT 1970");
        assert_eq!(
            format(1_795_000_000_123 + 7 * 3_600_000, "GMT+07:00"),
            "Wed Nov 18 18:06:40 GMT+07:00 2026"
        );
        assert_eq!(format(-1, "GMT"), "Wed Dec 31 23:59:59 GMT 1969");
    }
}
