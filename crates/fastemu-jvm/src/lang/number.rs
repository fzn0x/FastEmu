//! The wrapper classes (`Integer`, `Long`, `Short`, `Byte`, `Boolean`,
//! `Character`, `Float`, `Double`), `Number`, and the character and
//! integer conversions they share with `String`.

use super::float_fmt::{double_to_string, float_to_string, parse_double, parse_float};
use super::{string_value, this, throw};
use crate::classfile::access;
use crate::ctx::{Ctx, JResult};
use crate::heap::Body;
use crate::native::NativeClass;
use crate::value::Value;

/// The first code point of each run of ten decimal digits (Unicode `Nd`)
/// in the BMP.
const DIGIT_ZEROS: [u16; 37] = [
    0x0030, 0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66, 0x0BE6, 0x0C66, 0x0CE6,
    0x0D66, 0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80,
    0x1A90, 0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0,
    0xFF10,
];

/// The decimal value of a Unicode digit, as `Character.isDigit` sees one.
fn decimal_value(c: u16) -> Option<u32> {
    let i = DIGIT_ZEROS.partition_point(|&z| z <= c);
    let zero = *DIGIT_ZEROS.get(i.checked_sub(1)?)?;
    (c - zero < 10).then(|| u32::from(c - zero))
}

fn as_char(c: u16) -> Option<char> {
    char::from_u32(u32::from(c))
}

pub(crate) fn is_digit(c: u16) -> bool {
    decimal_value(c).is_some()
}

/// `Character.digit(char, int)`.
pub(crate) fn digit(c: u16, radix: i32) -> i32 {
    if !(2..=36).contains(&radix) {
        return -1;
    }
    let value = match c {
        0x41..=0x5A => u32::from(c - 0x41) + 10,
        0x61..=0x7A => u32::from(c - 0x61) + 10,
        0xFF21..=0xFF3A => u32::from(c - 0xFF21) + 10,
        0xFF41..=0xFF5A => u32::from(c - 0xFF41) + 10,
        _ => match decimal_value(c) {
            Some(v) => v,
            None => return -1,
        },
    };
    if value < radix as u32 {
        value as i32
    } else {
        -1
    }
}

/// `Character.toLowerCase(char)`: the one-to-one mapping.
pub(crate) fn to_lower(c: u16) -> u16 {
    if c < 0x80 {
        return u16::from((c as u8).to_ascii_lowercase());
    }
    if c == 0x0130 {
        return 0x0069;
    }
    single_mapping(c, char::to_lowercase)
}

/// `Character.toUpperCase(char)`: the one-to-one mapping.
pub(crate) fn to_upper(c: u16) -> u16 {
    if c < 0x80 {
        return u16::from((c as u8).to_ascii_uppercase());
    }
    single_mapping(c, char::to_uppercase)
}

fn single_mapping<I: Iterator<Item = char>>(c: u16, map: fn(char) -> I) -> u16 {
    let Some(ch) = as_char(c) else { return c };
    let mut mapped = map(ch);
    match (mapped.next(), mapped.next()) {
        (Some(m), None) => u16::try_from(u32::from(m)).unwrap_or(c),
        _ => c,
    }
}

fn is_whitespace(c: u16) -> bool {
    match c {
        0x09..=0x0D | 0x1C..=0x1F => true,
        0x00A0 | 0x2007 | 0x202F => false,
        _ => as_char(c).is_some_and(char::is_whitespace) && c != 0x85,
    }
}

fn is_space_char(c: u16) -> bool {
    matches!(
        c,
        0x20 | 0xA0 | 0x1680 | 0x2000..=0x200A | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000
    )
}

/// Java's integer parsing (`Integer.parseInt`, `Long.parseLong` and the
/// smaller types), with `min` and `max` the type's range.
pub(crate) fn parse_integer(s: &[u16], radix: i32, min: i64, max: i64) -> Option<i64> {
    if !(2..=36).contains(&radix) {
        return None;
    }
    let (negative, digits) = match s.first() {
        Some(&0x2D) => (true, &s[1..]),
        Some(&0x2B) => (false, &s[1..]),
        _ => (false, s),
    };
    if digits.is_empty() {
        return None;
    }
    let mut value: i128 = 0;
    for &c in digits {
        let d = digit(c, radix);
        if d < 0 {
            return None;
        }
        value = value * i128::from(radix) + i128::from(d);
        if value > i128::from(max) + 1 {
            return None;
        }
    }
    let value = if negative { -value } else { value };
    (i128::from(min)..=i128::from(max))
        .contains(&value)
        .then_some(value as i64)
}

/// `Long.toString(long, int)` (radix outside 2 to 36 means 10).
pub(crate) fn to_string_radix(v: i64, radix: i32) -> String {
    let radix = if (2..=36).contains(&radix) {
        radix as u64
    } else {
        10
    };
    let mut magnitude = v.unsigned_abs();
    let mut digits = Vec::new();
    loop {
        digits.push(
            char::from_digit((magnitude % radix) as u32, radix as u32)
                .expect("a digit below the radix"),
        );
        magnitude /= radix;
        if magnitude == 0 {
            break;
        }
    }
    if v < 0 {
        digits.push('-');
    }
    digits.iter().rev().collect()
}

/// `toHexString`, `toOctalString` and `toBinaryString`: the unsigned
/// value in base `1 << shift`.
fn unsigned_string(v: u64, shift: u32) -> String {
    let radix = 1u64 << shift;
    let mut v = v;
    let mut digits = Vec::new();
    loop {
        digits.push(
            char::from_digit((v % radix) as u32, radix as u32).expect("a digit below the radix"),
        );
        v >>= shift;
        if v == 0 {
            break;
        }
    }
    digits.iter().rev().collect()
}

/// Reads the `value` field of a wrapper, its only field.
fn value(ctx: &Ctx<'_>, a: &[Value]) -> Value {
    match &ctx.state.heap.get(this(a)).body {
        Body::Fields(f) => f[0],
        _ => Value::Void,
    }
}

#[allow(
    clippy::unnecessary_wraps,
    reason = "it has the shape of a native method body, so natives return it directly"
)]
fn set_value(ctx: &mut Ctx<'_>, a: &[Value], v: Value) -> JResult<Value> {
    if let Body::Fields(f) = &mut ctx.state.heap.get_mut(this(a)).body {
        f[0] = v;
    }
    Ok(Value::Void)
}

/// The text of a `String` argument, for parsing; `NumberFormatException`
/// for `null`.
fn parse_arg(ctx: &mut Ctx<'_>, v: Value) -> JResult<Vec<u16>> {
    match v.obj() {
        Some(s) => Ok(ctx.string_utf16(s)?.to_vec()),
        None => throw(ctx, "java/lang/NumberFormatException", "null"),
    }
}

fn number_format<T>(ctx: &mut Ctx<'_>, s: &[u16], radix: i32) -> JResult<T> {
    let text = String::from_utf16_lossy(s);
    let message = if radix == 10 {
        format!("For input string: \"{text}\"")
    } else {
        format!("For input string: \"{text}\" under radix {radix}")
    };
    throw(ctx, "java/lang/NumberFormatException", &message)
}

fn parse(ctx: &mut Ctx<'_>, v: Value, radix: i32, min: i64, max: i64) -> JResult<i64> {
    let s = parse_arg(ctx, v)?;
    if !(2..=36).contains(&radix) {
        let message = format!("radix {radix} out of range");
        return throw(ctx, "java/lang/NumberFormatException", &message);
    }
    match parse_integer(&s, radix, min, max) {
        Some(n) => Ok(n),
        None => number_format(ctx, &s, radix),
    }
}

/// Makes a new wrapper of class `class` around `v`, for `valueOf`.
fn boxed(ctx: &mut Ctx<'_>, class: &str, v: Value) -> JResult<Value> {
    let cid = ctx.find_class(class)?;
    let obj = ctx.new_object(cid)?;
    if let Body::Fields(f) = &mut ctx.state.heap.get_mut(obj).body {
        f[0] = v;
    }
    Ok(obj.into())
}

/// `equals`: same class and equal value bits.
fn wrapper_equals(ctx: &Ctx<'_>, a: &[Value]) -> Value {
    let Some(other) = a[1].obj() else {
        return Value::from(false);
    };
    if ctx.class_of(other) != ctx.class_of(this(a)) {
        return Value::from(false);
    }
    let mine = value(ctx, a);
    let theirs = value(ctx, &[Value::from(other)]);
    Value::from(match (mine, theirs) {
        (Value::Float(x), Value::Float(y)) => float_bits(x) == float_bits(y),
        (Value::Double(x), Value::Double(y)) => double_bits(x) == double_bits(y),
        (x, y) => x == y,
    })
}

/// `Float.floatToIntBits`: every NaN is 0x7fc00000.
pub(crate) fn float_bits(v: f32) -> i32 {
    if v.is_nan() {
        0x7fc0_0000
    } else {
        v.to_bits() as i32
    }
}

/// `Double.doubleToLongBits`.
pub(crate) fn double_bits(v: f64) -> i64 {
    if v.is_nan() {
        0x7ff8_0000_0000_0000
    } else {
        v.to_bits() as i64
    }
}

/// The conversions every numeric wrapper offers, from its `value`.
fn number_methods(class: NativeClass, convert: fn(Value) -> (i64, f64)) -> NativeClass {
    class
        .method("byteValue", "()B", move |ctx, a| {
            Ok(Value::Int(i32::from(convert(value(ctx, a)).0 as i8)))
        })
        .method("shortValue", "()S", move |ctx, a| {
            Ok(Value::Int(i32::from(convert(value(ctx, a)).0 as i16)))
        })
        .method("intValue", "()I", move |ctx, a| {
            Ok(Value::Int(convert(value(ctx, a)).0 as i32))
        })
        .method("longValue", "()J", move |ctx, a| {
            Ok(Value::Long(convert(value(ctx, a)).0))
        })
        .method("floatValue", "()F", move |ctx, a| {
            Ok(Value::Float(convert(value(ctx, a)).1 as f32))
        })
        .method("doubleValue", "()D", move |ctx, a| {
            Ok(Value::Double(convert(value(ctx, a)).1))
        })
        .method("equals", "(Ljava/lang/Object;)Z", |ctx, a| {
            Ok(wrapper_equals(ctx, a))
        })
}

fn integral(v: Value) -> (i64, f64) {
    match v {
        Value::Long(l) => (l, l as f64),
        v => (i64::from(v.int()), f64::from(v.int())),
    }
}

fn floating(v: Value) -> (i64, f64) {
    match v {
        Value::Float(f) => (f as i64, f64::from(f)),
        v => (v.double() as i64, v.double()),
    }
}

/// `Integer`, `Short` and `Byte`: the same shape with different ranges.
/// `narrow` truncates an `int` to the type, for `valueOf`.
fn small_integer(
    name: &str,
    desc: &str,
    parse_name: &str,
    min: i32,
    max: i32,
    narrow: fn(i32) -> i32,
) -> NativeClass {
    let class = format!("java/lang/{name}");
    let me = format!("L{class};");
    let for_parse = class.clone();
    let for_box = class.clone();
    let c = NativeClass::new(&class)
        .extends("java/lang/Number")
        .final_class()
        .field_with("value", desc, access::PRIVATE | access::FINAL)
        .constant("MIN_VALUE", desc, Value::Int(min))
        .constant("MAX_VALUE", desc, Value::Int(max))
        .method("<init>", &format!("({desc})V"), |ctx, a| {
            set_value(ctx, a, a[1])
        })
        .method("<init>", "(Ljava/lang/String;)V", move |ctx, a| {
            let n = parse(ctx, a[1], 10, i64::from(min), i64::from(max))?;
            set_value(ctx, a, Value::Int(n as i32))
        })
        .method("hashCode", "()I", |ctx, a| {
            Ok(Value::Int(value(ctx, a).int()))
        })
        .method("toString", "()Ljava/lang/String;", |ctx, a| {
            let v = value(ctx, a).int();
            string_value(ctx, &v.to_string())
        })
        .method("compareTo", &format!("({me})I"), |ctx, a| {
            let other = ctx.non_null(a[1])?;
            let x = value(ctx, a).int();
            let y = value(ctx, &[Value::from(other)]).int();
            Ok(Value::Int(x.cmp(&y) as i32))
        })
        .static_method(
            parse_name,
            &format!("(Ljava/lang/String;){desc}"),
            move |ctx, a| {
                Ok(Value::Int(
                    parse(ctx, a[0], 10, i64::from(min), i64::from(max))? as i32,
                ))
            },
        )
        .static_method(
            parse_name,
            &format!("(Ljava/lang/String;I){desc}"),
            move |ctx, a| {
                Ok(Value::Int(
                    parse(ctx, a[0], a[1].int(), i64::from(min), i64::from(max))? as i32,
                ))
            },
        )
        .static_method(
            "valueOf",
            &format!("(Ljava/lang/String;){me}"),
            move |ctx, a| {
                let n = parse(ctx, a[0], 10, i64::from(min), i64::from(max))?;
                boxed(ctx, &for_parse, Value::Int(n as i32))
            },
        )
        .static_method("valueOf", &format!("({desc}){me}"), move |ctx, a| {
            boxed(ctx, &for_box, Value::Int(narrow(a[0].int())))
        })
        .static_method(
            "toString",
            &format!("({desc})Ljava/lang/String;"),
            |ctx, a| string_value(ctx, &a[0].int().to_string()),
        );
    number_methods(c, integral)
}

fn integer() -> NativeClass {
    let c = small_integer("Integer", "I", "parseInt", i32::MIN, i32::MAX, |n| n);
    c.static_method(
        "valueOf",
        "(Ljava/lang/String;I)Ljava/lang/Integer;",
        |ctx, a| {
            let n = parse(
                ctx,
                a[0],
                a[1].int(),
                i64::from(i32::MIN),
                i64::from(i32::MAX),
            )?;
            boxed(ctx, "java/lang/Integer", Value::Int(n as i32))
        },
    )
    .static_method("toString", "(II)Ljava/lang/String;", |ctx, a| {
        string_value(ctx, &to_string_radix(i64::from(a[0].int()), a[1].int()))
    })
    .static_method("toHexString", "(I)Ljava/lang/String;", |ctx, a| {
        string_value(ctx, &unsigned_string(u64::from(a[0].int() as u32), 4))
    })
    .static_method("toOctalString", "(I)Ljava/lang/String;", |ctx, a| {
        string_value(ctx, &unsigned_string(u64::from(a[0].int() as u32), 3))
    })
    .static_method("toBinaryString", "(I)Ljava/lang/String;", |ctx, a| {
        string_value(ctx, &unsigned_string(u64::from(a[0].int() as u32), 1))
    })
}

fn long() -> NativeClass {
    let c = NativeClass::new("java/lang/Long")
        .extends("java/lang/Number")
        .final_class()
        .field_with("value", "J", access::PRIVATE | access::FINAL)
        .constant("MIN_VALUE", "J", Value::Long(i64::MIN))
        .constant("MAX_VALUE", "J", Value::Long(i64::MAX))
        .method("<init>", "(J)V", |ctx, a| set_value(ctx, a, a[1]))
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            let n = parse(ctx, a[1], 10, i64::MIN, i64::MAX)?;
            set_value(ctx, a, Value::Long(n))
        })
        .method("hashCode", "()I", |ctx, a| {
            let v = value(ctx, a).long();
            Ok(Value::Int((v ^ (v >> 32)) as i32))
        })
        .method("toString", "()Ljava/lang/String;", |ctx, a| {
            let v = value(ctx, a).long();
            string_value(ctx, &v.to_string())
        })
        .method("compareTo", "(Ljava/lang/Long;)I", |ctx, a| {
            let other = ctx.non_null(a[1])?;
            let x = value(ctx, a).long();
            let y = value(ctx, &[Value::from(other)]).long();
            Ok(Value::Int(x.cmp(&y) as i32))
        })
        .static_method("parseLong", "(Ljava/lang/String;)J", |ctx, a| {
            Ok(Value::Long(parse(ctx, a[0], 10, i64::MIN, i64::MAX)?))
        })
        .static_method("parseLong", "(Ljava/lang/String;I)J", |ctx, a| {
            Ok(Value::Long(parse(
                ctx,
                a[0],
                a[1].int(),
                i64::MIN,
                i64::MAX,
            )?))
        })
        .static_method("valueOf", "(J)Ljava/lang/Long;", |ctx, a| {
            boxed(ctx, "java/lang/Long", a[0])
        })
        .static_method(
            "valueOf",
            "(Ljava/lang/String;)Ljava/lang/Long;",
            |ctx, a| {
                let n = parse(ctx, a[0], 10, i64::MIN, i64::MAX)?;
                boxed(ctx, "java/lang/Long", Value::Long(n))
            },
        )
        .static_method("toString", "(J)Ljava/lang/String;", |ctx, a| {
            string_value(ctx, &a[0].long().to_string())
        })
        .static_method("toString", "(JI)Ljava/lang/String;", |ctx, a| {
            string_value(ctx, &to_string_radix(a[0].long(), a[1].int()))
        })
        .static_method("toHexString", "(J)Ljava/lang/String;", |ctx, a| {
            string_value(ctx, &unsigned_string(a[0].long() as u64, 4))
        })
        .static_method("toOctalString", "(J)Ljava/lang/String;", |ctx, a| {
            string_value(ctx, &unsigned_string(a[0].long() as u64, 3))
        })
        .static_method("toBinaryString", "(J)Ljava/lang/String;", |ctx, a| {
            string_value(ctx, &unsigned_string(a[0].long() as u64, 1))
        });
    number_methods(c, integral)
}

fn float() -> NativeClass {
    let c = NativeClass::new("java/lang/Float")
        .extends("java/lang/Number")
        .final_class()
        .field_with("value", "F", access::PRIVATE | access::FINAL)
        .constant("MAX_VALUE", "F", Value::Float(f32::MAX))
        .constant("MIN_VALUE", "F", Value::Float(f32::from_bits(1)))
        .constant("NaN", "F", Value::Float(f32::NAN))
        .constant("POSITIVE_INFINITY", "F", Value::Float(f32::INFINITY))
        .constant("NEGATIVE_INFINITY", "F", Value::Float(f32::NEG_INFINITY))
        .method("<init>", "(F)V", |ctx, a| set_value(ctx, a, a[1]))
        .method("<init>", "(D)V", |ctx, a| {
            set_value(ctx, a, Value::Float(a[1].double() as f32))
        })
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            let f = parse_f32(ctx, a[1])?;
            set_value(ctx, a, Value::Float(f))
        })
        .method("hashCode", "()I", |ctx, a| {
            Ok(Value::Int(float_bits(value(ctx, a).float())))
        })
        .method("toString", "()Ljava/lang/String;", |ctx, a| {
            let v = value(ctx, a).float();
            string_value(ctx, &float_to_string(v))
        })
        .method("isNaN", "()Z", |ctx, a| {
            Ok(Value::from(value(ctx, a).float().is_nan()))
        })
        .method("isInfinite", "()Z", |ctx, a| {
            Ok(Value::from(value(ctx, a).float().is_infinite()))
        })
        .static_method("isNaN", "(F)Z", |_, a| {
            Ok(Value::from(a[0].float().is_nan()))
        })
        .static_method("isInfinite", "(F)Z", |_, a| {
            Ok(Value::from(a[0].float().is_infinite()))
        })
        .static_method("floatToIntBits", "(F)I", |_, a| {
            Ok(Value::Int(float_bits(a[0].float())))
        })
        .static_method("floatToRawIntBits", "(F)I", |_, a| {
            Ok(Value::Int(a[0].float().to_bits() as i32))
        })
        .static_method("intBitsToFloat", "(I)F", |_, a| {
            Ok(Value::Float(f32::from_bits(a[0].int() as u32)))
        })
        .static_method("parseFloat", "(Ljava/lang/String;)F", |ctx, a| {
            Ok(Value::Float(parse_f32(ctx, a[0])?))
        })
        .static_method(
            "valueOf",
            "(Ljava/lang/String;)Ljava/lang/Float;",
            |ctx, a| {
                let f = parse_f32(ctx, a[0])?;
                boxed(ctx, "java/lang/Float", Value::Float(f))
            },
        )
        .static_method("valueOf", "(F)Ljava/lang/Float;", |ctx, a| {
            boxed(ctx, "java/lang/Float", a[0])
        })
        .static_method("toString", "(F)Ljava/lang/String;", |ctx, a| {
            string_value(ctx, &float_to_string(a[0].float()))
        })
        .static_method("compare", "(FF)I", |_, a| {
            Ok(Value::Int(compare_float(
                f64::from(a[0].float()),
                f64::from(a[1].float()),
            )))
        });
    number_methods(c, floating)
}

fn double() -> NativeClass {
    let c = NativeClass::new("java/lang/Double")
        .extends("java/lang/Number")
        .final_class()
        .field_with("value", "D", access::PRIVATE | access::FINAL)
        .constant("MAX_VALUE", "D", Value::Double(f64::MAX))
        .constant("MIN_VALUE", "D", Value::Double(f64::from_bits(1)))
        .constant("NaN", "D", Value::Double(f64::NAN))
        .constant("POSITIVE_INFINITY", "D", Value::Double(f64::INFINITY))
        .constant("NEGATIVE_INFINITY", "D", Value::Double(f64::NEG_INFINITY))
        .method("<init>", "(D)V", |ctx, a| set_value(ctx, a, a[1]))
        .method("<init>", "(Ljava/lang/String;)V", |ctx, a| {
            let d = parse_f64(ctx, a[1])?;
            set_value(ctx, a, Value::Double(d))
        })
        .method("hashCode", "()I", |ctx, a| {
            let bits = double_bits(value(ctx, a).double());
            Ok(Value::Int((bits ^ (bits >> 32)) as i32))
        })
        .method("toString", "()Ljava/lang/String;", |ctx, a| {
            let v = value(ctx, a).double();
            string_value(ctx, &double_to_string(v))
        })
        .method("isNaN", "()Z", |ctx, a| {
            Ok(Value::from(value(ctx, a).double().is_nan()))
        })
        .method("isInfinite", "()Z", |ctx, a| {
            Ok(Value::from(value(ctx, a).double().is_infinite()))
        })
        .static_method("isNaN", "(D)Z", |_, a| {
            Ok(Value::from(a[0].double().is_nan()))
        })
        .static_method("isInfinite", "(D)Z", |_, a| {
            Ok(Value::from(a[0].double().is_infinite()))
        })
        .static_method("doubleToLongBits", "(D)J", |_, a| {
            Ok(Value::Long(double_bits(a[0].double())))
        })
        .static_method("doubleToRawLongBits", "(D)J", |_, a| {
            Ok(Value::Long(a[0].double().to_bits() as i64))
        })
        .static_method("longBitsToDouble", "(J)D", |_, a| {
            Ok(Value::Double(f64::from_bits(a[0].long() as u64)))
        })
        .static_method("parseDouble", "(Ljava/lang/String;)D", |ctx, a| {
            Ok(Value::Double(parse_f64(ctx, a[0])?))
        })
        .static_method(
            "valueOf",
            "(Ljava/lang/String;)Ljava/lang/Double;",
            |ctx, a| {
                let d = parse_f64(ctx, a[0])?;
                boxed(ctx, "java/lang/Double", Value::Double(d))
            },
        )
        .static_method("valueOf", "(D)Ljava/lang/Double;", |ctx, a| {
            boxed(ctx, "java/lang/Double", a[0])
        })
        .static_method("toString", "(D)Ljava/lang/String;", |ctx, a| {
            string_value(ctx, &double_to_string(a[0].double()))
        })
        .static_method("compare", "(DD)I", |_, a| {
            Ok(Value::Int(compare_float(a[0].double(), a[1].double())))
        });
    number_methods(c, floating)
}

/// `Double.compare`: NaN is greater than everything, -0.0 less than 0.0.
fn compare_float(x: f64, y: f64) -> i32 {
    if x < y {
        -1
    } else if x > y {
        1
    } else {
        double_bits(x).cmp(&double_bits(y)) as i32
    }
}

fn float_text(ctx: &mut Ctx<'_>, v: Value) -> JResult<String> {
    match v.obj() {
        Some(s) => ctx.string(s),
        None => throw(ctx, "java/lang/NullPointerException", "null"),
    }
}

fn parse_f32(ctx: &mut Ctx<'_>, v: Value) -> JResult<f32> {
    let text = float_text(ctx, v)?;
    match parse_float(&text) {
        Some(f) => Ok(f),
        None => throw(ctx, "java/lang/NumberFormatException", &float_error(&text)),
    }
}

fn parse_f64(ctx: &mut Ctx<'_>, v: Value) -> JResult<f64> {
    let text = float_text(ctx, v)?;
    match parse_double(&text) {
        Some(d) => Ok(d),
        None => throw(ctx, "java/lang/NumberFormatException", &float_error(&text)),
    }
}

fn float_error(text: &str) -> String {
    if text.trim().is_empty() {
        "empty String".to_owned()
    } else {
        format!("For input string: \"{text}\"")
    }
}

fn boolean() -> NativeClass {
    NativeClass::new("java/lang/Boolean")
        .final_class()
        .field_with("value", "Z", access::PRIVATE | access::FINAL)
        .static_field("TRUE", "Ljava/lang/Boolean;")
        .static_field("FALSE", "Ljava/lang/Boolean;")
        .static_init(|ctx, _| {
            for (name, v) in [("TRUE", true), ("FALSE", false)] {
                let b = boxed(ctx, "java/lang/Boolean", Value::from(v))?;
                let field =
                    ctx.static_field_id("java/lang/Boolean", name, "Ljava/lang/Boolean;")?;
                ctx.state.classes.statics[field.index as usize] = b;
            }
            Ok(Value::Void)
        })
        .method("<init>", "(Z)V", |ctx, a| set_value(ctx, a, a[1]))
        .method("booleanValue", "()Z", |ctx, a| Ok(value(ctx, a)))
        .method("hashCode", "()I", |ctx, a| {
            Ok(Value::Int(if value(ctx, a).boolean() {
                1231
            } else {
                1237
            }))
        })
        .method("equals", "(Ljava/lang/Object;)Z", |ctx, a| {
            Ok(wrapper_equals(ctx, a))
        })
        .method("toString", "()Ljava/lang/String;", |ctx, a| {
            let v = value(ctx, a).boolean();
            string_value(ctx, if v { "true" } else { "false" })
        })
        .static_method("valueOf", "(Z)Ljava/lang/Boolean;", |ctx, a| {
            let name = if a[0].boolean() { "TRUE" } else { "FALSE" };
            let field = ctx.static_field_id("java/lang/Boolean", name, "Ljava/lang/Boolean;")?;
            ctx.get_static(field)
        })
        .static_method("parseBoolean", "(Ljava/lang/String;)Z", |ctx, a| {
            let s = ctx.opt_string(a[0])?;
            Ok(Value::from(
                s.is_some_and(|s| s.eq_ignore_ascii_case("true")),
            ))
        })
        .static_method("toString", "(Z)Ljava/lang/String;", |ctx, a| {
            string_value(ctx, if a[0].boolean() { "true" } else { "false" })
        })
}

fn character() -> NativeClass {
    let predicate =
        |f: fn(u16) -> bool| move |_: &mut Ctx<'_>, a: &[Value]| Ok(Value::from(f(a[0].char())));
    NativeClass::new("java/lang/Character")
        .final_class()
        .field_with("value", "C", access::PRIVATE | access::FINAL)
        .constant("MIN_RADIX", "I", Value::Int(2))
        .constant("MAX_RADIX", "I", Value::Int(36))
        .constant("MIN_VALUE", "C", Value::Int(0))
        .constant("MAX_VALUE", "C", Value::Int(0xFFFF))
        .method("<init>", "(C)V", |ctx, a| set_value(ctx, a, a[1]))
        .method("charValue", "()C", |ctx, a| Ok(value(ctx, a)))
        .method("hashCode", "()I", |ctx, a| Ok(value(ctx, a)))
        .method("equals", "(Ljava/lang/Object;)Z", |ctx, a| {
            Ok(wrapper_equals(ctx, a))
        })
        .method("toString", "()Ljava/lang/String;", |ctx, a| {
            let c = value(ctx, a).char();
            Ok(ctx.new_string_utf16(vec![c])?.into())
        })
        .static_method("toString", "(C)Ljava/lang/String;", |ctx, a| {
            Ok(ctx.new_string_utf16(vec![a[0].char()])?.into())
        })
        .static_method("valueOf", "(C)Ljava/lang/Character;", |ctx, a| {
            boxed(ctx, "java/lang/Character", a[0])
        })
        .static_method("isDigit", "(C)Z", predicate(is_digit))
        .static_method(
            "isLetter",
            "(C)Z",
            predicate(|c| as_char(c).is_some_and(char::is_alphabetic)),
        )
        .static_method(
            "isLetterOrDigit",
            "(C)Z",
            predicate(|c| is_digit(c) || as_char(c).is_some_and(char::is_alphabetic)),
        )
        .static_method(
            "isLowerCase",
            "(C)Z",
            predicate(|c| as_char(c).is_some_and(char::is_lowercase)),
        )
        .static_method(
            "isUpperCase",
            "(C)Z",
            predicate(|c| as_char(c).is_some_and(char::is_uppercase)),
        )
        .static_method("isWhitespace", "(C)Z", predicate(is_whitespace))
        .static_method("isSpaceChar", "(C)Z", predicate(is_space_char))
        .static_method("toLowerCase", "(C)C", |_, a| {
            Ok(Value::Int(i32::from(to_lower(a[0].char()))))
        })
        .static_method("toUpperCase", "(C)C", |_, a| {
            Ok(Value::Int(i32::from(to_upper(a[0].char()))))
        })
        .static_method("digit", "(CI)I", |_, a| {
            Ok(Value::Int(digit(a[0].char(), a[1].int())))
        })
        .static_method("forDigit", "(II)C", |_, a| {
            let (d, radix) = (a[0].int(), a[1].int());
            let c = if !(2..=36).contains(&radix) || d < 0 || d >= radix {
                0
            } else if d < 10 {
                0x30 + d
            } else {
                0x61 + d - 10
            };
            Ok(Value::Int(c))
        })
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new("java/lang/Number")
            .abstract_class()
            .method("<init>", "()V", |_, _| Ok(Value::Void))
            .abstract_method("intValue", "()I")
            .abstract_method("longValue", "()J")
            .abstract_method("floatValue", "()F")
            .abstract_method("doubleValue", "()D")
            .method("byteValue", "()B", |ctx, a| {
                let v = ctx.call_virtual(this(a), "intValue", "()I", &[])?.int();
                Ok(Value::Int(i32::from(v as i8)))
            })
            .method("shortValue", "()S", |ctx, a| {
                let v = ctx.call_virtual(this(a), "intValue", "()I", &[])?.int();
                Ok(Value::Int(i32::from(v as i16)))
            }),
        integer(),
        small_integer(
            "Short",
            "S",
            "parseShort",
            i32::from(i16::MIN),
            i32::from(i16::MAX),
            |n| i32::from(n as i16),
        ),
        small_integer(
            "Byte",
            "B",
            "parseByte",
            i32::from(i8::MIN),
            i32::from(i8::MAX),
            |n| i32::from(n as i8),
        ),
        long(),
        float(),
        double(),
        boolean(),
        character(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    #[test]
    fn integers_parse_like_java() {
        let int = |s: &str, r| parse_integer(&u(s), r, i64::from(i32::MIN), i64::from(i32::MAX));
        assert_eq!(int("-2147483648", 10), Some(-2_147_483_648));
        assert_eq!(int("2147483648", 10), None);
        assert_eq!(int("+42", 10), Some(42));
        assert_eq!(int("-", 10), None);
        assert_eq!(int("", 10), None);
        assert_eq!(int("ff", 16), Some(255));
        assert_eq!(int("FF", 16), Some(255));
        assert_eq!(int("12a", 10), None);
        assert_eq!(int(" 1", 10), None);
        assert_eq!(int("\u{661}\u{662}", 10), Some(12), "Arabic-Indic digits");
        assert_eq!(int("zz", 36), Some(1295));
        assert_eq!(int("1", 37), None);
    }

    #[test]
    fn integers_print_like_java() {
        assert_eq!(to_string_radix(-255, 16), "-ff");
        assert_eq!(to_string_radix(i64::MIN, 2).len(), 65);
        assert_eq!(to_string_radix(7, 99), "7");
        assert_eq!(unsigned_string(u64::from(-1i32 as u32), 4), "ffffffff");
        assert_eq!(unsigned_string(8, 3), "10");
        assert_eq!(unsigned_string(0, 1), "0");
    }

    #[test]
    fn characters_follow_java() {
        assert_eq!(digit(u16::from(b'7'), 10), 7);
        assert_eq!(digit(u16::from(b'z'), 36), 35);
        assert_eq!(digit(u16::from(b'z'), 35), -1);
        assert_eq!(
            to_upper(0xDF),
            0xDF,
            "sharp s has no one-character upper case"
        );
        assert_eq!(to_lower(0x130), 0x69);
        assert_eq!(to_upper(u16::from(b'q')), u16::from(b'Q'));
        assert_eq!(to_lower(0x0416), 0x0436);
        assert!(is_whitespace(0x09) && !is_whitespace(0xA0) && is_space_char(0xA0));
        assert!(is_digit(0xFF19) && !is_digit(u16::from(b'a')));
    }

    #[test]
    fn float_bits_canonicalize_nan() {
        assert_eq!(float_bits(f32::from_bits(0x7f80_0001)), 0x7fc0_0000);
        assert_eq!(compare_float(0.0, -0.0), 1);
        assert_eq!(compare_float(f64::NAN, f64::INFINITY), 1);
    }
}
