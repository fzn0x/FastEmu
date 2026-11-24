//! `java.util.Random`: the 48-bit linear congruential generator the
//! javadoc fixes, so a game seeded with a constant builds the same levels
//! as on every phone.
//!
//! A game's subclass may override the protected `next(int)` that every
//! other method draws from, so for a subclass they call it through
//! virtual dispatch, as Java's do; a plain `Random` runs in Rust.

use crate::classfile::access::{PROTECTED, PUBLIC, SYNCHRONIZED};
use crate::ctx::{Ctx, JResult};
use crate::heap::Payload;
use crate::lang::system::current_time_millis;
use crate::lang::{this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const RANDOM: &str = "java/util/Random";
const MULTIPLIER: i64 = 0x5_DEEC_E66D;
const ADDEND: i64 = 0xB;
const MASK: i64 = (1 << 48) - 1;

/// The generator's 48 bits of state.
#[derive(Default)]
struct Seed(i64);

impl Payload for Seed {}

/// The state `setSeed(seed)` starts from.
fn scramble(seed: i64) -> i64 {
    (seed ^ MULTIPLIER) & MASK
}

/// Java's `next(bits)`: one step of the generator, and its top `bits`.
/// A subclass may pass any `bits`; the shift count wraps to 0..63 as
/// Java's `>>>` on a `long` does.
fn step(seed: &mut i64, bits: i32) -> i32 {
    *seed = (seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND)) & MASK;
    (*seed as u64).wrapping_shr(48i32.wrapping_sub(bits) as u32) as i32
}

fn is_plain(ctx: &Ctx<'_>, obj: ObjRef) -> bool {
    ctx.class_name(ctx.class_of(obj)) == RANDOM
}

/// `next(bits)` of `obj`: the game's override for a subclass.
fn next(ctx: &mut Ctx<'_>, obj: ObjRef, bits: i32) -> JResult<i32> {
    if is_plain(ctx, obj) {
        return Ok(step(&mut ctx.payload_mut::<Seed>(obj)?.0, bits));
    }
    Ok(ctx
        .call_virtual(obj, "next", "(I)I", &[Value::Int(bits)])?
        .int())
}

/// The constructors: a subclass gets its seed through its own `setSeed`,
/// as in Java.
fn init(ctx: &mut Ctx<'_>, obj: ObjRef, seed: i64) -> JResult<Value> {
    if is_plain(ctx, obj) {
        ctx.payload_mut::<Seed>(obj)?.0 = scramble(seed);
    } else {
        ctx.call_virtual(obj, "setSeed", "(J)V", &[Value::Long(seed)])?;
    }
    Ok(Value::Void)
}

/// `nextInt(bound)`: rejection sampling, as in Java.
fn next_int_below(ctx: &mut Ctx<'_>, obj: ObjRef, bound: i32) -> JResult<i32> {
    if bound <= 0 {
        return throw(
            ctx,
            "java/lang/IllegalArgumentException",
            "bound must be positive",
        );
    }
    let mut r = next(ctx, obj, 31)?;
    let m = bound - 1;
    if bound & m == 0 {
        return Ok(((i64::from(bound) * i64::from(r)) >> 31) as i32);
    }
    let mut u = r;
    loop {
        r = u % bound;
        if u.wrapping_sub(r).wrapping_add(m) >= 0 {
            return Ok(r);
        }
        u = next(ctx, obj, 31)?;
    }
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::new(RANDOM)
            .payload::<Seed>()
            .method("<init>", "()V", |ctx, a| {
                init(ctx, this(a), current_time_millis())
            })
            .method("<init>", "(J)V", |ctx, a| init(ctx, this(a), a[1].long()))
            .method_with("setSeed", "(J)V", PUBLIC | SYNCHRONIZED, |ctx, a| {
                ctx.payload_mut::<Seed>(this(a))?.0 = scramble(a[1].long());
                Ok(Value::Void)
            })
            .method_with("next", "(I)I", PROTECTED, |ctx, a| {
                let seed = &mut ctx.payload_mut::<Seed>(this(a))?.0;
                Ok(Value::Int(step(seed, a[1].int())))
            })
            .method("nextInt", "()I", |ctx, a| {
                Ok(Value::Int(next(ctx, this(a), 32)?))
            })
            .method("nextInt", "(I)I", |ctx, a| {
                Ok(Value::Int(next_int_below(ctx, this(a), a[1].int())?))
            })
            .method("nextLong", "()J", |ctx, a| {
                let high = i64::from(next(ctx, this(a), 32)?);
                let low = i64::from(next(ctx, this(a), 32)?);
                Ok(Value::Long((high << 32).wrapping_add(low)))
            })
            .method("nextFloat", "()F", |ctx, a| {
                let bits = next(ctx, this(a), 24)?;
                Ok(Value::Float(bits as f32 / (1 << 24) as f32))
            })
            .method("nextDouble", "()D", |ctx, a| {
                let high = i64::from(next(ctx, this(a), 26)?);
                let low = i64::from(next(ctx, this(a), 27)?);
                let bits = (high << 27) + low;
                Ok(Value::Double(bits as f64 * (1.0 / (1u64 << 53) as f64)))
            }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The values the JDK gives for `new Random(42)` and `setSeed(-7)`.
    #[test]
    fn the_sequence_is_javas() {
        let mut seed = scramble(42);
        assert_eq!(step(&mut seed, 32), -1_170_105_035);
        let mut seed = scramble(-7);
        assert_eq!(step(&mut seed, 32), 1_155_869_324);
    }

    /// `next` is protected, so a subclass can ask it for any number of
    /// bits; Java masks the shift count instead of failing.
    #[test]
    fn any_bit_count_shifts_as_java_does() {
        let draw = |bits| step(&mut scramble(42), bits);
        // The state after one step, whose low 32 bits a shift of 0 gives.
        let mut state = scramble(42);
        step(&mut state, 32);
        // 48 - bits, masked to 0..63: a shift of 48 or more leaves 0.
        for bits in [0, 49, 60, 64, -1] {
            assert_eq!(draw(bits), 0, "{bits}");
        }
        for bits in [-16, 112] {
            assert_eq!(draw(bits), state as i32, "{bits}");
        }
        assert_eq!(draw(16), (state >> 32) as i32);
        // The extremes do not overflow the subtraction.
        draw(i32::MIN);
        draw(i32::MAX);
    }

    #[test]
    fn every_method_draws_as_java_does() {
        let source = std::sync::Arc::new(crate::MemorySource::new());
        let vm = crate::Vm::new(crate::VmConfig::default(), source, Vec::new());
        let values = vm.attach("test", |ctx| {
            let r = ctx.construct(RANDOM, "(J)V", &[Value::Long(42)]).unwrap();
            let call = |ctx: &mut Ctx<'_>, name: &str, desc: &str, args: &[Value]| {
                ctx.call_virtual(r, name, desc, args).unwrap()
            };
            [
                call(ctx, "nextInt", "()I", &[]),
                call(ctx, "nextInt", "(I)I", &[Value::Int(100)]),
                call(ctx, "nextInt", "(I)I", &[Value::Int(1 << 20)]),
                call(ctx, "nextLong", "()J", &[]),
                call(ctx, "nextFloat", "()F", &[]),
                call(ctx, "nextDouble", "()D", &[]),
            ]
        });
        assert_eq!(
            values,
            [
                Value::Int(-1_170_105_035),
                Value::Int(63),
                Value::Int(716_411),
                Value::Long(884_324_181_205_335_268),
                Value::Float(0.942_073_5),
                Value::Double(0.277_078_490_074_136_65),
            ]
        );
    }
}
