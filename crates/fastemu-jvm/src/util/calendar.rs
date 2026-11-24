//! `java.util.Calendar`, with the Gregorian calendar `getInstance`
//! returns.
//!
//! The fields behave as on Java's lenient `GregorianCalendar`: `set`
//! only records a field, and the next `get` or `getTime` computes the
//! time from the fields (13 months is a year and a month, day 0 the last
//! day of the month before) and then every field from the time. The
//! hour comes from `HOUR` and `AM_PM` when one of them was set after
//! `HOUR_OF_DAY`, and the date from the week and `DAY_OF_WEEK` when that
//! was set after `DAY_OF_MONTH`, with weeks starting on Sunday as in the
//! US locale.
//!
//! As in CLDC 1.1, a game may extend `Calendar` and implement the
//! protected `computeFields` and `computeTime` over the protected
//! `fields`, `isSet` and `time`; `Calendar`'s own methods call them
//! through virtual dispatch.

use super::date::new_date;
use super::time::{DAY_MS, civil_from_days, day_of_week, days_from_civil};
use super::timezone;
use crate::class::ArrayType;
use crate::classfile::access::{FINAL, PROTECTED, PUBLIC};
use crate::ctx::{Ctx, FieldId, JResult};
use crate::heap::{Payload, Tracer};
use crate::lang::system::current_time_millis;
use crate::lang::{this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const CALENDAR: &str = "java/util/Calendar";
const IMPL: &str = "java/util/Calendar$Impl";

/// The number of fields, `ERA` (0) to `MILLISECOND` (14).
const FIELD_COUNT: usize = 15;
const ERA: usize = 0;
const YEAR: usize = 1;
const MONTH: usize = 2;
const WEEK_OF_YEAR: usize = 3;
const WEEK_OF_MONTH: usize = 4;
const DATE: usize = 5;
const DAY_OF_YEAR: usize = 6;
const DAY_OF_WEEK: usize = 7;
const DAY_OF_WEEK_IN_MONTH: usize = 8;
const AM_PM: usize = 9;
const HOUR: usize = 10;
const HOUR_OF_DAY: usize = 11;
const MINUTE: usize = 12;
const SECOND: usize = 13;
const MILLISECOND: usize = 14;

/// The stamp of a field the calendar computed, older than any `set`.
const COMPUTED: u32 = 1;

/// What a `Calendar` keeps beside its protected fields.
struct State {
    zone: Option<ObjRef>,
    /// Whether `time` matches the fields.
    time_valid: bool,
    /// Whether the fields match `time`.
    fields_valid: bool,
    /// When each field was last set, to tell which of two ways to give
    /// the hour or the date is the newer.
    stamps: [u32; FIELD_COUNT],
    next_stamp: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            zone: None,
            time_valid: false,
            fields_valid: false,
            stamps: [COMPUTED; FIELD_COUNT],
            next_stamp: COMPUTED + 1,
        }
    }
}

impl Payload for State {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.zone);
    }
}

/// The protected fields of a `Calendar`.
struct Ids {
    fields: FieldId,
    is_set: FieldId,
    time: FieldId,
}

fn ids(ctx: &mut Ctx<'_>) -> JResult<Ids> {
    Ok(Ids {
        fields: ctx.field_id(CALENDAR, "fields", "[I")?,
        is_set: ctx.field_id(CALENDAR, "isSet", "[Z")?,
        time: ctx.field_id(CALENDAR, "time", "J")?,
    })
}

fn state<'a>(ctx: &'a mut Ctx<'_>, cal: ObjRef) -> JResult<&'a mut State> {
    ctx.payload_mut::<State>(cal)
}

/// The calendar's zone: the default one for an object no constructor
/// ran on.
fn zone(ctx: &mut Ctx<'_>, cal: ObjRef) -> JResult<ObjRef> {
    if let Some(zone) = state(ctx, cal)?.zone {
        return Ok(ctx.root(zone));
    }
    let zone = timezone::default_zone(ctx)?;
    state(ctx, cal)?.zone = Some(zone);
    Ok(zone)
}

/// The `field` argument of `get` and `set`, checked as Java's array
/// access checks it.
fn field_index(ctx: &mut Ctx<'_>, field: i32) -> JResult<usize> {
    match usize::try_from(field).ok().filter(|&f| f < FIELD_COUNT) {
        Some(f) => Ok(f),
        None => {
            let message = format!("Index {field} out of bounds for length {FIELD_COUNT}");
            throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message)
        }
    }
}

/// The protected `fields` array, which must hold every field.
fn fields_array(ctx: &mut Ctx<'_>, ids: &Ids, cal: ObjRef) -> JResult<ObjRef> {
    let arr = ctx.get_field(cal, ids.fields);
    let arr = ctx.non_null(arr)?;
    let len = ctx.array_len(arr)?;
    if len < FIELD_COUNT {
        let message = format!("Index {} out of bounds for length {len}", FIELD_COUNT - 1);
        return throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message);
    }
    Ok(arr)
}

fn is_impl(ctx: &Ctx<'_>, cal: ObjRef) -> bool {
    ctx.class_name(ctx.class_of(cal)) == IMPL
}

/// `computeTime()`: in Rust for this library's calendar, through the
/// game's override for its own.
fn compute_time(ctx: &mut Ctx<'_>, cal: ObjRef) -> JResult<()> {
    if is_impl(ctx, cal) {
        return gregorian_time(ctx, cal);
    }
    ctx.call_virtual(cal, "computeTime", "()V", &[])?;
    Ok(())
}

/// `computeFields()`, likewise; every field then counts as computed.
fn compute_fields(ctx: &mut Ctx<'_>, cal: ObjRef) -> JResult<()> {
    if is_impl(ctx, cal) {
        gregorian_fields(ctx, cal)?;
    } else {
        ctx.call_virtual(cal, "computeFields", "()V", &[])?;
    }
    let st = state(ctx, cal)?;
    st.stamps = [COMPUTED; FIELD_COUNT];
    st.next_stamp = COMPUTED + 1;
    Ok(())
}

/// Brings the time and then the fields up to date, as Java's `complete`.
fn complete(ctx: &mut Ctx<'_>, cal: ObjRef) -> JResult<()> {
    time_in_millis(ctx, cal)?;
    if !state(ctx, cal)?.fields_valid {
        compute_fields(ctx, cal)?;
        state(ctx, cal)?.fields_valid = true;
    }
    Ok(())
}

/// `getTimeInMillis()`.
fn time_in_millis(ctx: &mut Ctx<'_>, cal: ObjRef) -> JResult<i64> {
    if !state(ctx, cal)?.time_valid {
        compute_time(ctx, cal)?;
        state(ctx, cal)?.time_valid = true;
    }
    let ids = ids(ctx)?;
    Ok(ctx.get_field(cal, ids.time).long())
}

/// `setTimeInMillis(millis)`.
fn set_time_in_millis(ctx: &mut Ctx<'_>, cal: ObjRef, millis: i64) -> JResult<()> {
    let ids = ids(ctx)?;
    ctx.set_field(cal, ids.time, Value::Long(millis));
    let st = state(ctx, cal)?;
    st.time_valid = true;
    st.fields_valid = false;
    compute_fields(ctx, cal)?;
    state(ctx, cal)?.fields_valid = true;
    Ok(())
}

/// Every field of the Gregorian calendar for the wall-clock time
/// `local`, with weeks starting on Sunday and week 1 the one holding the
/// first day, as in the US locale.
fn gregorian(local: i64) -> [i32; FIELD_COUNT] {
    let days = local.div_euclid(DAY_MS);
    let ms = local.rem_euclid(DAY_MS);
    let (year, month, day) = civil_from_days(days);
    let dow = day_of_week(days);
    let year_start = days_from_civil(year, 1, 1);
    let day_of_year = days - year_start + 1;
    let week_of_year = {
        // The Sunday that starts this week; if the week holds the next
        // 1 January, it is week 1 of the next year.
        let week_start = days - (dow - 1);
        if week_start + 6 >= days_from_civil(year + 1, 1, 1) {
            1
        } else {
            (day_of_year - 1 + day_of_week(year_start) - 1) / 7 + 1
        }
    };
    let month_start_dow = day_of_week(days_from_civil(year, month, 1));
    let hour = ms / 3_600_000;
    let (era, era_year) = if year > 0 { (1, year) } else { (0, 1 - year) };
    let mut f = [0i64; FIELD_COUNT];
    f[ERA] = era;
    f[YEAR] = era_year;
    f[MONTH] = month - 1;
    f[WEEK_OF_YEAR] = week_of_year;
    f[WEEK_OF_MONTH] = (day - 1 + month_start_dow - 1) / 7 + 1;
    f[DATE] = day;
    f[DAY_OF_YEAR] = day_of_year;
    f[DAY_OF_WEEK] = dow;
    f[DAY_OF_WEEK_IN_MONTH] = (day - 1) / 7 + 1;
    f[AM_PM] = hour / 12;
    f[HOUR] = hour % 12;
    f[HOUR_OF_DAY] = hour;
    f[MINUTE] = ms / 60_000 % 60;
    f[SECOND] = ms / 1000 % 60;
    f[MILLISECOND] = ms % 1000;
    f.map(|v| v as i32)
}

/// The wall-clock time the fields give, leniently, as Java's
/// `GregorianCalendar.computeTime` reads them.
fn gregorian_local(fields: &[i32; FIELD_COUNT], stamps: &[u32; FIELD_COUNT]) -> i64 {
    let field = |i: usize| i64::from(fields[i]);
    let mut year = field(YEAR);
    if field(ERA) == 0 {
        year = 1 - year;
    }
    let month = field(MONTH);
    let year = year + month.div_euclid(12);
    let month = month.rem_euclid(12) + 1;
    let month_start = days_from_civil(year, month, 1);
    let days = if stamps[DAY_OF_WEEK] > stamps[DATE] {
        let first_week = month_start - (day_of_week(month_start) - 1);
        first_week + 7 * (field(WEEK_OF_MONTH) - 1) + (field(DAY_OF_WEEK) - 1).rem_euclid(7)
    } else {
        month_start + field(DATE) - 1
    };
    let twelve_hour = stamps[HOUR].max(stamps[AM_PM]) > stamps[HOUR_OF_DAY];
    let hour = if twelve_hour {
        field(HOUR) + 12 * field(AM_PM)
    } else {
        field(HOUR_OF_DAY)
    };
    let time_of_day =
        ((hour * 60 + field(MINUTE)) * 60 + field(SECOND)) * 1000 + field(MILLISECOND);
    days * DAY_MS + time_of_day
}

/// `computeFields()` of this library's calendar.
fn gregorian_fields(ctx: &mut Ctx<'_>, cal: ObjRef) -> JResult<()> {
    let ids = ids(ctx)?;
    let time = ctx.get_field(cal, ids.time).long();
    let zone = zone(ctx, cal)?;
    let offset = timezone::offset_at(ctx, zone, time)?;
    let values = gregorian(time.saturating_add(offset));
    let arr = fields_array(ctx, &ids, cal)?;
    ctx.array_mut::<i32>(arr)?[..FIELD_COUNT].copy_from_slice(&values);
    let set = ctx.get_field(cal, ids.is_set);
    if let Some(set) = set.obj() {
        for flag in ctx.array_mut::<i8>(set)?.iter_mut().take(FIELD_COUNT) {
            *flag = 1;
        }
    }
    Ok(())
}

/// `computeTime()` of this library's calendar.
fn gregorian_time(ctx: &mut Ctx<'_>, cal: ObjRef) -> JResult<()> {
    let ids = ids(ctx)?;
    let arr = fields_array(ctx, &ids, cal)?;
    let mut fields = [0; FIELD_COUNT];
    fields.copy_from_slice(&ctx.array::<i32>(arr)?[..FIELD_COUNT]);
    let stamps = state(ctx, cal)?.stamps;
    let local = gregorian_local(&fields, &stamps);
    let zone = zone(ctx, cal)?;
    let offset = timezone::offset_for_local(ctx, zone, local)?;
    ctx.set_field(cal, ids.time, Value::Long(local - offset));
    Ok(())
}

fn init(ctx: &mut Ctx<'_>, cal: ObjRef) -> JResult<Value> {
    let ids = ids(ctx)?;
    let fields = ctx.new_array(ArrayType::Int, FIELD_COUNT as i32)?;
    let is_set = ctx.new_array(ArrayType::Boolean, FIELD_COUNT as i32)?;
    ctx.set_field(cal, ids.fields, fields.into());
    ctx.set_field(cal, ids.is_set, is_set.into());
    let zone = timezone::default_zone(ctx)?;
    state(ctx, cal)?.zone = Some(zone);
    Ok(Value::Void)
}

/// `Calendar.getInstance(zone)`: the Gregorian calendar at this moment.
fn instance(ctx: &mut Ctx<'_>, zone: Option<ObjRef>) -> JResult<Value> {
    let cal = ctx.construct(IMPL, "()V", &[])?;
    if let Some(zone) = zone {
        state(ctx, cal)?.zone = Some(zone);
    }
    set_time_in_millis(ctx, cal, current_time_millis())?;
    Ok(cal.into())
}

/// The time of `when` if it is a `Calendar`.
fn other_millis(ctx: &mut Ctx<'_>, when: Value) -> JResult<Option<i64>> {
    let calendar = ctx.find_class(CALENDAR)?;
    match when.obj().filter(|&o| ctx.is_instance(o, calendar)) {
        Some(other) => Ok(Some(time_in_millis(ctx, other)?)),
        None => Ok(None),
    }
}

pub(crate) fn classes() -> Vec<NativeClass> {
    let fin = PUBLIC | FINAL;
    vec![
        NativeClass::new(CALENDAR)
            .abstract_class()
            .payload::<State>()
            .field_with("fields", "[I", PROTECTED)
            .field_with("isSet", "[Z", PROTECTED)
            .field_with("time", "J", PROTECTED)
            .constant("ERA", "I", Value::Int(ERA as i32))
            .constant("YEAR", "I", Value::Int(YEAR as i32))
            .constant("MONTH", "I", Value::Int(MONTH as i32))
            .constant("DATE", "I", Value::Int(DATE as i32))
            .constant("DAY_OF_MONTH", "I", Value::Int(DATE as i32))
            .constant("DAY_OF_WEEK", "I", Value::Int(DAY_OF_WEEK as i32))
            .constant("AM_PM", "I", Value::Int(AM_PM as i32))
            .constant("HOUR", "I", Value::Int(HOUR as i32))
            .constant("HOUR_OF_DAY", "I", Value::Int(HOUR_OF_DAY as i32))
            .constant("MINUTE", "I", Value::Int(MINUTE as i32))
            .constant("SECOND", "I", Value::Int(SECOND as i32))
            .constant("MILLISECOND", "I", Value::Int(MILLISECOND as i32))
            .constant("SUNDAY", "I", Value::Int(1))
            .constant("MONDAY", "I", Value::Int(2))
            .constant("TUESDAY", "I", Value::Int(3))
            .constant("WEDNESDAY", "I", Value::Int(4))
            .constant("THURSDAY", "I", Value::Int(5))
            .constant("FRIDAY", "I", Value::Int(6))
            .constant("SATURDAY", "I", Value::Int(7))
            .constant("JANUARY", "I", Value::Int(0))
            .constant("FEBRUARY", "I", Value::Int(1))
            .constant("MARCH", "I", Value::Int(2))
            .constant("APRIL", "I", Value::Int(3))
            .constant("MAY", "I", Value::Int(4))
            .constant("JUNE", "I", Value::Int(5))
            .constant("JULY", "I", Value::Int(6))
            .constant("AUGUST", "I", Value::Int(7))
            .constant("SEPTEMBER", "I", Value::Int(8))
            .constant("OCTOBER", "I", Value::Int(9))
            .constant("NOVEMBER", "I", Value::Int(10))
            .constant("DECEMBER", "I", Value::Int(11))
            .constant("AM", "I", Value::Int(0))
            .constant("PM", "I", Value::Int(1))
            .method_with("<init>", "()V", PROTECTED, |ctx, a| init(ctx, this(a)))
            .static_method("getInstance", "()Ljava/util/Calendar;", |ctx, _| {
                instance(ctx, None)
            })
            .static_method(
                "getInstance",
                "(Ljava/util/TimeZone;)Ljava/util/Calendar;",
                |ctx, a| {
                    let zone = ctx.non_null(a[0])?;
                    instance(ctx, Some(zone))
                },
            )
            .method_with("getTime", "()Ljava/util/Date;", fin, |ctx, a| {
                let millis = time_in_millis(ctx, this(a))?;
                Ok(new_date(ctx, millis)?.into())
            })
            .method_with("setTime", "(Ljava/util/Date;)V", fin, |ctx, a| {
                let date = ctx.non_null(a[1])?;
                let millis = ctx.call_virtual(date, "getTime", "()J", &[])?.long();
                set_time_in_millis(ctx, this(a), millis)?;
                Ok(Value::Void)
            })
            .method_with("getTimeInMillis", "()J", PROTECTED, |ctx, a| {
                Ok(Value::Long(time_in_millis(ctx, this(a))?))
            })
            .method_with("setTimeInMillis", "(J)V", PROTECTED, |ctx, a| {
                set_time_in_millis(ctx, this(a), a[1].long())?;
                Ok(Value::Void)
            })
            .method_with("get", "(I)I", fin, |ctx, a| {
                let cal = this(a);
                let field = field_index(ctx, a[1].int())?;
                complete(ctx, cal)?;
                let ids = ids(ctx)?;
                let arr = fields_array(ctx, &ids, cal)?;
                Ok(Value::Int(ctx.array::<i32>(arr)?[field]))
            })
            .method_with("set", "(II)V", fin, |ctx, a| {
                let cal = this(a);
                let field = field_index(ctx, a[1].int())?;
                let ids = ids(ctx)?;
                let arr = fields_array(ctx, &ids, cal)?;
                ctx.array_mut::<i32>(arr)?[field] = a[2].int();
                if let Some(set) = ctx.get_field(cal, ids.is_set).obj()
                    && let Some(flag) = ctx.array_mut::<i8>(set)?.get_mut(field)
                {
                    *flag = 1;
                }
                let st = state(ctx, cal)?;
                st.time_valid = false;
                st.fields_valid = false;
                st.stamps[field] = st.next_stamp;
                st.next_stamp += 1;
                Ok(Value::Void)
            })
            .method("equals", "(Ljava/lang/Object;)Z", |ctx, a| {
                let cal = this(a);
                let (Some(theirs), Some(other)) = (other_millis(ctx, a[1])?, a[1].obj()) else {
                    return Ok(Value::from(false));
                };
                let mine = time_in_millis(ctx, cal)?;
                let (zone, other_zone) = (zone(ctx, cal)?, zone(ctx, other)?);
                let same_zone = timezone::id_of(ctx, zone)? == timezone::id_of(ctx, other_zone)?;
                Ok(Value::from(mine == theirs && same_zone))
            })
            .method("before", "(Ljava/lang/Object;)Z", |ctx, a| {
                let theirs = other_millis(ctx, a[1])?;
                let mine = time_in_millis(ctx, this(a))?;
                Ok(Value::from(theirs.is_some_and(|t| mine < t)))
            })
            .method("after", "(Ljava/lang/Object;)Z", |ctx, a| {
                let theirs = other_millis(ctx, a[1])?;
                let mine = time_in_millis(ctx, this(a))?;
                Ok(Value::from(theirs.is_some_and(|t| mine > t)))
            })
            .method("setTimeZone", "(Ljava/util/TimeZone;)V", |ctx, a| {
                let zone = ctx.non_null(a[1])?;
                let st = state(ctx, this(a))?;
                st.zone = Some(zone);
                st.fields_valid = false;
                Ok(Value::Void)
            })
            .method("getTimeZone", "()Ljava/util/TimeZone;", |ctx, a| {
                Ok(zone(ctx, this(a))?.into())
            })
            .abstract_method_with("computeFields", "()V", PROTECTED)
            .abstract_method_with("computeTime", "()V", PROTECTED),
        NativeClass::new(IMPL)
            .extends(CALENDAR)
            .final_class()
            .method("<init>", "()V", |ctx, a| {
                ctx.call_special(CALENDAR, this(a), "<init>", "()V", &[])?;
                Ok(Value::Void)
            })
            .method_with("computeFields", "()V", PROTECTED, |ctx, a| {
                gregorian_fields(ctx, this(a))?;
                Ok(Value::Void)
            })
            .method_with("computeTime", "()V", PROTECTED, |ctx, a| {
                gregorian_time(ctx, this(a))?;
                Ok(Value::Void)
            }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(year: i64, month: i64, day: i64, ms: i64) -> i64 {
        days_from_civil(year, month, day) * DAY_MS + ms
    }

    #[test]
    fn fields_follow_the_gregorian_calendar() {
        let f = gregorian(at(2026, 11, 24, 15 * 3_600_000 + 4 * 60_000 + 5_006));
        assert_eq!(f[YEAR], 2026);
        assert_eq!(f[MONTH], 10);
        assert_eq!(f[DATE], 24);
        assert_eq!(f[DAY_OF_WEEK], 3);
        assert_eq!((f[HOUR_OF_DAY], f[HOUR], f[AM_PM]), (15, 3, 1));
        assert_eq!((f[MINUTE], f[SECOND], f[MILLISECOND]), (4, 5, 6));
        assert_eq!(
            (f[DAY_OF_YEAR], f[WEEK_OF_YEAR], f[WEEK_OF_MONTH]),
            (328, 48, 4)
        );
        // 31 December 2026 is a Thursday in the week of 1 January 2027.
        assert_eq!(gregorian(at(2026, 12, 31, 0))[WEEK_OF_YEAR], 1);
    }

    #[test]
    fn lenient_fields_roll_over() {
        let mut f = gregorian(at(2025, 1, 31, 0));
        let stamps = [COMPUTED; FIELD_COUNT];
        f[MONTH] = 13;
        assert_eq!(
            gregorian_local(&f, &stamps),
            at(2026, 2, 28, 0) + 3 * DAY_MS
        );
        f[MONTH] = 0;
        f[DATE] = 0;
        assert_eq!(gregorian_local(&f, &stamps), at(2024, 12, 31, 0));
        f[DATE] = 1;
        f[HOUR_OF_DAY] = 25;
        assert_eq!(gregorian_local(&f, &stamps), at(2025, 1, 2, 3_600_000));
    }

    #[test]
    fn the_newer_hour_and_date_fields_win() {
        let mut f = gregorian(at(2026, 11, 25, 9 * 3_600_000));
        let mut stamps = [COMPUTED; FIELD_COUNT];
        f[HOUR] = 2;
        f[AM_PM] = 1;
        stamps[HOUR] = 2;
        assert_eq!(
            gregorian_local(&f, &stamps),
            at(2026, 11, 25, 14 * 3_600_000)
        );
        f[DAY_OF_WEEK] = 2;
        stamps[DAY_OF_WEEK] = 3;
        assert_eq!(
            gregorian_local(&f, &stamps),
            at(2026, 11, 23, 14 * 3_600_000)
        );
    }
}
