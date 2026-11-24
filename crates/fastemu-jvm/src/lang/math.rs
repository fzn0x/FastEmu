//! `java.lang.Math`: the CLDC 1.1 methods, plus the few Java SE ones
//! programs built with a desktop `javac` reach for.

use crate::native::NativeClass;
use crate::value::Value;

/// `Math.max` for floating point: NaN wins, and 0.0 beats -0.0.
fn max(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() { b } else { a }
    } else if a >= b {
        a
    } else {
        b
    }
}

/// `Math.min` for floating point: NaN wins, and -0.0 beats 0.0.
fn min(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else if a == 0.0 && b == 0.0 {
        if a.is_sign_negative() { a } else { b }
    } else if a <= b {
        a
    } else {
        b
    }
}

/// `Math.round(double)`: the closest `long`, ties toward positive
/// infinity, NaN to 0, saturating.
fn round(x: f64) -> i64 {
    if x.is_nan() {
        return 0;
    }
    let floor = x.floor();
    let r = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    r as i64
}

fn round_f(x: f32) -> i32 {
    if x.is_nan() {
        return 0;
    }
    let floor = x.floor();
    let r = if x - floor >= 0.5 { floor + 1.0 } else { floor };
    r as i32
}

const DEGREES_TO_RADIANS: f64 = 0.017_453_292_519_943_295;
const RADIANS_TO_DEGREES: f64 = 57.295_779_513_082_32;

pub(crate) fn classes() -> Vec<NativeClass> {
    let d1 = |f: fn(f64) -> f64| {
        move |_: &mut crate::Ctx<'_>, a: &[Value]| Ok(Value::Double(f(a[0].double())))
    };
    let d2 = |f: fn(f64, f64) -> f64| {
        move |_: &mut crate::Ctx<'_>, a: &[Value]| {
            Ok(Value::Double(f(a[0].double(), a[1].double())))
        }
    };
    let f2 = |f: fn(f64, f64) -> f64| {
        move |_: &mut crate::Ctx<'_>, a: &[Value]| {
            Ok(Value::Float(
                f(f64::from(a[0].float()), f64::from(a[1].float())) as f32,
            ))
        }
    };
    vec![
        NativeClass::new("java/lang/Math")
            .final_class()
            .constant("E", "D", Value::Double(std::f64::consts::E))
            .constant("PI", "D", Value::Double(std::f64::consts::PI))
            .static_method("abs", "(I)I", |_, a| {
                Ok(Value::Int(a[0].int().wrapping_abs()))
            })
            .static_method("abs", "(J)J", |_, a| {
                Ok(Value::Long(a[0].long().wrapping_abs()))
            })
            .static_method("abs", "(F)F", |_, a| Ok(Value::Float(a[0].float().abs())))
            .static_method("abs", "(D)D", d1(f64::abs))
            .static_method("max", "(II)I", |_, a| {
                Ok(Value::Int(a[0].int().max(a[1].int())))
            })
            .static_method("max", "(JJ)J", |_, a| {
                Ok(Value::Long(a[0].long().max(a[1].long())))
            })
            .static_method("max", "(FF)F", f2(max))
            .static_method("max", "(DD)D", d2(max))
            .static_method("min", "(II)I", |_, a| {
                Ok(Value::Int(a[0].int().min(a[1].int())))
            })
            .static_method("min", "(JJ)J", |_, a| {
                Ok(Value::Long(a[0].long().min(a[1].long())))
            })
            .static_method("min", "(FF)F", f2(min))
            .static_method("min", "(DD)D", d2(min))
            .static_method("ceil", "(D)D", d1(f64::ceil))
            .static_method("floor", "(D)D", d1(f64::floor))
            .static_method("sqrt", "(D)D", d1(f64::sqrt))
            .static_method("sin", "(D)D", d1(f64::sin))
            .static_method("cos", "(D)D", d1(f64::cos))
            .static_method("tan", "(D)D", d1(f64::tan))
            .static_method("asin", "(D)D", d1(f64::asin))
            .static_method("acos", "(D)D", d1(f64::acos))
            .static_method("atan", "(D)D", d1(f64::atan))
            .static_method("atan2", "(DD)D", d2(f64::atan2))
            .static_method("exp", "(D)D", d1(f64::exp))
            .static_method("log", "(D)D", d1(f64::ln))
            .static_method("log10", "(D)D", d1(f64::log10))
            .static_method("pow", "(DD)D", d2(f64::powf))
            .static_method("toRadians", "(D)D", d1(|x| x * DEGREES_TO_RADIANS))
            .static_method("toDegrees", "(D)D", d1(|x| x * RADIANS_TO_DEGREES))
            .static_method("round", "(D)J", |_, a| {
                Ok(Value::Long(round(a[0].double())))
            })
            .static_method("round", "(F)I", |_, a| {
                Ok(Value::Int(round_f(a[0].float())))
            })
            .static_method("random", "()D", |ctx, _| {
                let seed = ctx.state.next_random();
                Ok(Value::Double(seed))
            }),
    ]
}

impl crate::vm::VmState {
    /// The next `Math.random()` value: a 53-bit fraction from a xorshift
    /// generator seeded from the clock.
    fn next_random(&mut self) -> f64 {
        if self.random == 0 {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64);
            self.random = nanos | 1;
        }
        let mut x = self.random;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.random = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_and_max_follow_java() {
        assert!(max(f64::NAN, 1.0).is_nan());
        assert!(min(1.0, f64::NAN).is_nan());
        assert!(max(-0.0, 0.0).is_sign_positive());
        assert!(min(0.0, -0.0).is_sign_negative());
        assert_eq!(max(2.0, 3.0), 3.0);
    }

    #[test]
    fn rounding_follows_java() {
        assert_eq!(round(2.5), 3);
        assert_eq!(round(-2.5), -2);
        assert_eq!(round(0.499_999_999_999_999_94), 0);
        assert_eq!(round(f64::NAN), 0);
        assert_eq!(round(1e300), i64::MAX);
        assert_eq!(round_f(-0.5), 0);
    }
}
