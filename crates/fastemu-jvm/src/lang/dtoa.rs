//! Java's classic conversion of `double` and `float` to decimal digits:
//! the algorithm of `sun.misc.FloatingDecimal` that every Java before 19
//! used, CLDC included.
//!
//! It is not the shortest-digits algorithm Java 19 switched to: for an
//! integer-valued number it prints the integer's digits (so `(float)
//! 2390378240.0` prints as `2.39037824E9`), and its stopping test can
//! leave one digit more than needed (`1.0E23` prints as
//! `9.999999999999999E22`). Games ran on phones with this algorithm, so
//! FastEmu prints numbers the way they saw them.

use std::cmp::Ordering;

/// An unsigned integer of any size, in little-endian 32-bit limbs.
#[derive(Clone, Debug)]
struct Big(Vec<u32>);

impl Big {
    fn from_u64(v: u64) -> Self {
        let mut b = Big(vec![v as u32, (v >> 32) as u32]);
        b.trim();
        b
    }

    /// `value * 5^p5 * 2^p2`.
    fn mul_pow52(value: u64, p5: u32, p2: u32) -> Self {
        let mut b = Big::from_u64(value);
        let mut p = p5;
        while p >= 13 {
            b.mul_small(1_220_703_125);
            p -= 13;
        }
        b.mul_small(5u32.pow(p));
        b.shl(p2);
        b
    }

    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn mul_small(&mut self, m: u32) {
        let mut carry = 0u64;
        for limb in &mut self.0 {
            let v = u64::from(*limb) * u64::from(m) + carry;
            *limb = v as u32;
            carry = v >> 32;
        }
        if carry != 0 {
            self.0.push(carry as u32);
        }
        self.trim();
    }

    fn shl(&mut self, bits: u32) {
        if self.is_zero() {
            return;
        }
        let (words, bits) = ((bits / 32) as usize, bits % 32);
        if bits != 0 {
            let mut carry = 0u32;
            for limb in &mut self.0 {
                let v = *limb;
                *limb = v << bits | carry;
                carry = v >> (32 - bits);
            }
            if carry != 0 {
                self.0.push(carry);
            }
        }
        self.0.splice(0..0, std::iter::repeat_n(0, words));
    }

    fn cmp(&self, other: &Big) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }

    fn add(&self, other: &Big) -> Big {
        let n = self.0.len().max(other.0.len());
        let mut out = Vec::with_capacity(n + 1);
        let mut carry = 0u64;
        for i in 0..n {
            let v = u64::from(*self.0.get(i).unwrap_or(&0))
                + u64::from(*other.0.get(i).unwrap_or(&0))
                + carry;
            out.push(v as u32);
            carry = v >> 32;
        }
        if carry != 0 {
            out.push(carry as u32);
        }
        let mut b = Big(out);
        b.trim();
        b
    }

    /// `self -= other`, where `self >= other`.
    fn sub_assign(&mut self, other: &Big) {
        let mut borrow = 0i64;
        for (i, limb) in self.0.iter_mut().enumerate() {
            let v = i64::from(*limb) - i64::from(*other.0.get(i).unwrap_or(&0)) - borrow;
            if v < 0 {
                *limb = (v + (1 << 32)) as u32;
                borrow = 1;
            } else {
                *limb = v as u32;
                borrow = 0;
            }
        }
        self.trim();
    }

    /// Returns `self / s` (a digit) and leaves `10 * (self % s)` in self.
    fn quo_rem_iteration(&mut self, s: &Big) -> i32 {
        let mut q = 0;
        while self.cmp(s) != Ordering::Less {
            self.sub_assign(s);
            q += 1;
        }
        self.mul_small(10);
        q
    }
}

/// `5^i` for `i` below 27, and how many bits each takes.
fn pow5(i: i32) -> i64 {
    5i64.pow(i as u32)
}

fn bits_of_pow5(i: i32) -> i32 {
    if i == 0 {
        0
    } else if i < 27 {
        64 - pow5(i).leading_zeros() as i32
    } else {
        i * 3
    }
}

/// Decimal digits and the exponent that places them: the value is
/// `0.d1 d2 d3 ... * 10^dec_exponent`.
struct Digits {
    chars: [u8; 20],
    first: usize,
    count: usize,
    dec_exponent: i32,
}

impl Digits {
    fn round_up(&mut self) {
        let mut i = self.first + self.count - 1;
        let mut q = self.chars[i];
        if q == b'9' {
            while q == b'9' && i > self.first {
                self.chars[i] = b'0';
                i -= 1;
                q = self.chars[i];
            }
            if q == b'9' {
                self.dec_exponent += 1;
                self.chars[self.first] = b'1';
                return;
            }
        }
        self.chars[i] = q + 1;
    }

    /// `FloatingDecimal.developLongDigits`: the digits of an integer,
    /// after dropping `insignificant` low digits with rounding.
    fn from_long(dec_exponent: i32, mut value: i64, insignificant: i32) -> Digits {
        let mut dec_exponent = dec_exponent;
        if insignificant != 0 {
            let pow10 = pow5(insignificant) << insignificant;
            let residue = value % pow10;
            value /= pow10;
            dec_exponent += insignificant;
            if residue >= pow10 >> 1 {
                value += 1;
            }
        }
        let mut d = Digits {
            chars: [b'0'; 20],
            first: 0,
            count: 0,
            dec_exponent: 0,
        };
        let mut at = d.chars.len() - 1;
        let mut c = value % 10;
        value /= 10;
        while c == 0 {
            dec_exponent += 1;
            c = value % 10;
            value /= 10;
        }
        while value != 0 {
            d.chars[at] = b'0' + c as u8;
            at -= 1;
            dec_exponent += 1;
            c = value % 10;
            value /= 10;
        }
        d.chars[at] = b'0' + c as u8;
        d.dec_exponent = dec_exponent + 1;
        d.first = at;
        d.count = d.chars.len() - at;
        d
    }
}

const EXP_SHIFT: i32 = 52;

/// The digit generation of `FloatingDecimal.BinaryToASCIIBuffer.dtoa`,
/// for `Double.toString` and `Float.toString` (compatible format).
/// `fract_bits` holds the significand with its high bit at bit 52.
fn dtoa(bin_exp: i32, mut fract_bits: u64, n_significant_bits: i32) -> Digits {
    let tail_zeros = fract_bits.trailing_zeros() as i32;
    let n_fract_bits = EXP_SHIFT + 1 - tail_zeros;
    let n_tiny_bits = (n_fract_bits - bin_exp - 1).max(0);
    if (-21..=62).contains(&bin_exp)
        && n_tiny_bits < 27
        && n_fract_bits + bits_of_pow5(n_tiny_bits) < 64
        && n_tiny_bits == 0
    {
        // An integer that fits a long: print its digits.
        let insignificant = if bin_exp > n_significant_bits {
            let p2 = bin_exp - n_significant_bits - 1;
            if p2 > 1 { p2 * 30_103 / 100_000 } else { 0 }
        } else {
            0
        };
        let value = if bin_exp >= EXP_SHIFT {
            fract_bits << (bin_exp - EXP_SHIFT)
        } else {
            fract_bits >> (EXP_SHIFT - bin_exp)
        };
        return Digits::from_long(0, value as i64, insignificant);
    }

    let mut dec_exp = estimate_dec_exp(fract_bits, bin_exp);
    let b5 = (-dec_exp).max(0);
    let mut b2 = b5 + n_tiny_bits + bin_exp;
    let s5 = dec_exp.max(0);
    let mut s2 = s5 + n_tiny_bits;
    let m5 = b5;
    let mut m2 = b2 - n_significant_bits;

    fract_bits >>= tail_zeros;
    b2 -= n_fract_bits - 1;
    let common = b2.min(s2);
    b2 -= common;
    s2 -= common;
    m2 -= common;
    // Below a power of two the next smaller number is only half as far.
    if n_fract_bits == 1 {
        m2 -= 1;
    }
    if m2 < 0 {
        b2 -= m2;
        s2 -= m2;
        m2 = 0;
    }

    let mut d = Digits {
        chars: [b'0'; 20],
        first: 0,
        count: 0,
        dec_exponent: 0,
    };
    fn push(d: &mut Digits, q: i32) {
        d.chars[d.count] = b'0' + q as u8;
        d.count += 1;
    }
    let low;
    let high;
    let low_digit_difference: i64;
    let b_bits = n_fract_bits + b2 + bits_of_pow5(b5);
    let ten_s_bits = s2 + 1 + bits_of_pow5(s5 + 1);
    if b_bits < 32 && ten_s_bits < 32 {
        let mut b = (fract_bits as i32)
            .wrapping_mul(pow5(b5) as i32)
            .wrapping_shl(b2 as u32);
        let s = (pow5(s5) as i32).wrapping_shl(s2 as u32);
        let mut m = (pow5(m5) as i32).wrapping_shl(m2 as u32);
        let tens = s.wrapping_mul(10);
        let q = b / s;
        b = (b % s).wrapping_mul(10);
        m = m.wrapping_mul(10);
        let mut lo = b < m;
        let mut hi = b.wrapping_add(m) > tens;
        if q == 0 && !hi {
            dec_exp -= 1;
        } else {
            push(&mut d, q);
        }
        if !(-3..8).contains(&dec_exp) {
            hi = false;
            lo = false;
        }
        while !lo && !hi {
            let q = b / s;
            b = (b % s).wrapping_mul(10);
            m = m.wrapping_mul(10);
            if m > 0 {
                lo = b < m;
                hi = b.wrapping_add(m) > tens;
            } else {
                lo = true;
                hi = true;
            }
            push(&mut d, q);
        }
        low = lo;
        high = hi;
        low_digit_difference = i64::from(b.wrapping_shl(1).wrapping_sub(tens));
    } else if b_bits < 64 && ten_s_bits < 64 {
        let mut b = (fract_bits as i64)
            .wrapping_mul(pow5(b5))
            .wrapping_shl(b2 as u32);
        let s = pow5(s5).wrapping_shl(s2 as u32);
        let mut m = pow5(m5).wrapping_shl(m2 as u32);
        let tens = s.wrapping_mul(10);
        let q = (b / s) as i32;
        b = (b % s).wrapping_mul(10);
        m = m.wrapping_mul(10);
        let mut lo = b < m;
        let mut hi = b.wrapping_add(m) > tens;
        if q == 0 && !hi {
            dec_exp -= 1;
        } else {
            push(&mut d, q);
        }
        if !(-3..8).contains(&dec_exp) {
            hi = false;
            lo = false;
        }
        while !lo && !hi {
            let q = (b / s) as i32;
            b = (b % s).wrapping_mul(10);
            m = m.wrapping_mul(10);
            if m > 0 {
                lo = b < m;
                hi = b.wrapping_add(m) > tens;
            } else {
                lo = true;
                hi = true;
            }
            push(&mut d, q);
        }
        low = lo;
        high = hi;
        low_digit_difference = b.wrapping_shl(1).wrapping_sub(tens);
    } else {
        let s = Big::mul_pow52(1, s5 as u32, s2 as u32);
        let mut b = Big::mul_pow52(fract_bits, b5 as u32, b2 as u32);
        let mut m = Big::mul_pow52(1, (m5 + 1) as u32, (m2 + 1) as u32);
        let ten_s = Big::mul_pow52(1, (s5 + 1) as u32, (s2 + 1) as u32);
        let q = b.quo_rem_iteration(&s);
        let mut lo = b.cmp(&m) == Ordering::Less;
        let mut hi = b.add(&m).cmp(&ten_s) != Ordering::Less;
        if q == 0 && !hi {
            dec_exp -= 1;
        } else {
            push(&mut d, q);
        }
        if !(-3..8).contains(&dec_exp) {
            hi = false;
            lo = false;
        }
        while !lo && !hi {
            let q = b.quo_rem_iteration(&s);
            m.mul_small(10);
            lo = b.cmp(&m) == Ordering::Less;
            hi = b.add(&m).cmp(&ten_s) != Ordering::Less;
            push(&mut d, q);
        }
        low = lo;
        high = hi;
        low_digit_difference = if hi && lo {
            b.shl(1);
            b.cmp(&ten_s) as i64
        } else {
            0
        };
    }
    d.dec_exponent = dec_exp + 1;
    if high {
        if low {
            if low_digit_difference == 0 {
                if d.chars[d.count - 1] & 1 != 0 {
                    d.round_up();
                }
            } else if low_digit_difference > 0 {
                d.round_up();
            }
        } else {
            d.round_up();
        }
    }
    d
}

/// `FloatingDecimal.estimateDecExp`: floor(log10(value)), estimated from
/// the significand and the binary exponent.
#[allow(
    clippy::approx_constant,
    reason = "the constants are FloatingDecimal's own literals, which the digits depend on"
)]
fn estimate_dec_exp(fract_bits: u64, bin_exp: i32) -> i32 {
    let d2 = f64::from_bits(0x3ff0_0000_0000_0000 | (fract_bits & 0x000f_ffff_ffff_ffff));
    let d = (d2 - 1.5) * 0.289_529_654 + 0.176_091_259 + f64::from(bin_exp) * 0.301_029_995_663_981;
    d.floor() as i32
}

/// Lays the digits out as `Double.toString` does.
fn layout(negative: bool, d: &Digits) -> String {
    let digits = &d.chars[d.first..d.first + d.count];
    let text = |b: &[u8]| std::str::from_utf8(b).expect("ASCII digits").to_owned();
    let mut out = String::with_capacity(26);
    if negative {
        out.push('-');
    }
    let e = d.dec_exponent;
    if e > 0 && e < 8 {
        let int_len = (e as usize).min(digits.len());
        out.push_str(&text(&digits[..int_len]));
        if int_len < e as usize {
            out.extend(std::iter::repeat_n('0', e as usize - int_len));
            out.push_str(".0");
        } else {
            out.push('.');
            if int_len < digits.len() {
                out.push_str(&text(&digits[int_len..]));
            } else {
                out.push('0');
            }
        }
    } else if e <= 0 && e > -3 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-e) as usize));
        out.push_str(&text(digits));
    } else {
        out.push(char::from(digits[0]));
        out.push('.');
        if digits.len() > 1 {
            out.push_str(&text(&digits[1..]));
        } else {
            out.push('0');
        }
        out.push('E');
        out.push_str(&(e - 1).to_string());
    }
    out
}

/// `Double.toString(double)` as Java before 19 (and CLDC) prints it.
pub(crate) fn double_to_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".to_owned();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let bits = v.to_bits();
    let negative = bits >> 63 != 0;
    let mut fract_bits = bits & 0x000f_ffff_ffff_ffff;
    let mut bin_exp = ((bits >> 52) & 0x7ff) as i32;
    let n_significant_bits;
    if bin_exp == 0 {
        if fract_bits == 0 {
            return if negative { "-0.0" } else { "0.0" }.to_owned();
        }
        let leading_zeros = fract_bits.leading_zeros() as i32;
        let shift = leading_zeros - (63 - EXP_SHIFT);
        fract_bits <<= shift;
        bin_exp = 1 - shift;
        n_significant_bits = 64 - leading_zeros;
    } else {
        fract_bits |= 1 << 52;
        n_significant_bits = EXP_SHIFT + 1;
    }
    layout(
        negative,
        &dtoa(bin_exp - 1023, fract_bits, n_significant_bits),
    )
}

/// `Float.toString(float)` as Java before 19 (and CLDC) prints it.
pub(crate) fn float_to_string(v: f32) -> String {
    if v.is_nan() {
        return "NaN".to_owned();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let bits = v.to_bits();
    let negative = bits >> 31 != 0;
    let mut fract_bits = bits & 0x007f_ffff;
    let mut bin_exp = ((bits >> 23) & 0xff) as i32;
    let n_significant_bits;
    if bin_exp == 0 {
        if fract_bits == 0 {
            return if negative { "-0.0" } else { "0.0" }.to_owned();
        }
        let leading_zeros = fract_bits.leading_zeros() as i32;
        let shift = leading_zeros - (31 - 23);
        fract_bits <<= shift;
        bin_exp = 1 - shift;
        n_significant_bits = 32 - leading_zeros;
    } else {
        fract_bits |= 1 << 23;
        n_significant_bits = 24;
    }
    let fract = u64::from(fract_bits) << (EXP_SHIFT - 23);
    layout(negative, &dtoa(bin_exp - 127, fract, n_significant_bits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doubles_print_like_java_before_19() {
        let cases = [
            (0.1, "0.1"),
            (1.0, "1.0"),
            (-2.5, "-2.5"),
            (100.0, "100.0"),
            (1.0e7, "1.0E7"),
            (9_999_999.0, "9999999.0"),
            (0.001, "0.001"),
            (0.0001, "1.0E-4"),
            (1.0 / 3.0, "0.3333333333333333"),
            (2.0 / 3.0, "0.6666666666666666"),
            (0.1 + 0.2, "0.30000000000000004"),
            (123_456_789.0, "1.23456789E8"),
            (1e21, "1.0E21"),
            (1e23, "9.999999999999999E22"),
            (2e-3, "0.002"),
            (f64::MAX, "1.7976931348623157E308"),
            (f64::from_bits(1), "4.9E-324"),
            (f64::MIN_POSITIVE, "2.2250738585072014E-308"),
            (-0.0, "-0.0"),
            (f64::NAN, "NaN"),
            (f64::NEG_INFINITY, "-Infinity"),
            (4.714_066_081_575_076e17, "4.7140660815750758E17"),
            (std::f64::consts::PI, "3.141592653589793"),
        ];
        for (v, want) in cases {
            assert_eq!(double_to_string(v), want, "{v:e}");
        }
    }

    #[test]
    fn floats_print_like_java_before_19() {
        let cases = [
            (0.1f32, "0.1"),
            (1.0 / 3.0, "0.33333334"),
            (3.0e10, "3.0000001E10"),
            (1.0e-5, "1.0E-5"),
            (f32::MAX, "3.4028235E38"),
            (f32::from_bits(1), "1.4E-45"),
            (16_777_216.0, "1.6777216E7"),
            (123.456, "123.456"),
            (2_390_378_240.0, "2.39037824E9"),
            (-3.338_211_7e13, "-3.33821174E13"),
            (4.714_066e17, "4.71406608E17"),
        ];
        for (v, want) in cases {
            assert_eq!(float_to_string(v), want, "{v:e}");
        }
    }

    #[test]
    fn big_integers_do_digit_steps() {
        let mut b = Big::mul_pow52(7, 30, 70);
        let s = Big::mul_pow52(3, 30, 70);
        assert_eq!(b.quo_rem_iteration(&s), 2);
        let mut expect = Big::mul_pow52(1, 30, 70);
        expect.mul_small(10);
        assert_eq!(b.cmp(&expect), Ordering::Equal);
        assert_eq!(Big::from_u64(u64::MAX).add(&Big::from_u64(1)).0, [0, 0, 1]);
    }
}
