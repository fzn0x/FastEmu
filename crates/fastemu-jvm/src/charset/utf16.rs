//! UTF-16 in its four Java forms, converting exactly as the JDK's
//! `UnicodeDecoder` and `UnicodeEncoder` do, malformed input included.

use super::REPLACEMENT;

/// Which of Java's UTF-16 charsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Utf16 {
    /// `UTF-16`: a byte order mark at the start picks the order,
    /// big-endian without one; encodes big-endian after a mark.
    Marked,
    /// `UTF-16BE`: big-endian, a mark is an ordinary U+FEFF.
    Big,
    /// `UTF-16LE`: little-endian, a mark is an ordinary U+FEFF.
    Little,
    /// `x-UTF-16LE-BOM` (`UnicodeLittle`): as `UTF-16`, but little-endian
    /// without a mark and when encoding.
    MarkedLittle,
}

/// A byte order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Order {
    Big,
    Little,
}

const MARK: u16 = 0xFEFF;
const SWAPPED_MARK: u16 = 0xFFFE;

fn is_high(unit: u16) -> bool {
    (0xD800..0xDC00).contains(&unit)
}

fn is_low(unit: u16) -> bool {
    (0xDC00..0xE000).contains(&unit)
}

impl Utf16 {
    /// The order a charset without byte order detection always has.
    fn fixed(self) -> Option<Order> {
        match self {
            Utf16::Big => Some(Order::Big),
            Utf16::Little => Some(Order::Little),
            Utf16::Marked | Utf16::MarkedLittle => None,
        }
    }

    /// The order it encodes in, and decodes in without a mark.
    fn order(self) -> Order {
        match self {
            Utf16::Marked | Utf16::Big => Order::Big,
            Utf16::Little | Utf16::MarkedLittle => Order::Little,
        }
    }

    /// Whether encoding starts with a byte order mark.
    pub(super) fn writes_mark(self) -> bool {
        matches!(self, Utf16::Marked | Utf16::MarkedLittle)
    }

    /// Decodes `bytes` onto `out` and returns how many it used. `order`
    /// is the byte order the stream settled on, `None` at its start,
    /// where a charset that detects one reads the byte order mark.
    ///
    /// Unless `end` is set, a unit cut off at the end of `bytes`, or a
    /// high surrogate whose partner has not come yet, is left for the
    /// next call. With `end`, what is left becomes one U+FFFD. A high
    /// surrogate followed by anything but a low one becomes one U+FFFD
    /// with it, a lone low surrogate one U+FFFD, as in Java.
    pub(super) fn decode(
        self,
        order: &mut Option<Order>,
        bytes: &[u8],
        end: bool,
        out: &mut Vec<u16>,
    ) -> usize {
        let mut i = 0;
        while bytes.len() - i >= 2 {
            let pair = [bytes[i], bytes[i + 1]];
            let o = match (*order, self.fixed()) {
                (Some(o), _) | (None, Some(o)) => o,
                (None, None) => match u16::from_be_bytes(pair) {
                    MARK | SWAPPED_MARK => {
                        let mark = u16::from_be_bytes(pair) == MARK;
                        *order = Some(if mark { Order::Big } else { Order::Little });
                        i += 2;
                        continue;
                    }
                    _ => self.order(),
                },
            };
            *order = Some(o);
            let unit = read(o, pair);
            if is_high(unit) {
                if bytes.len() - i < 4 {
                    break;
                }
                let low = read(o, [bytes[i + 2], bytes[i + 3]]);
                if is_low(low) {
                    out.extend([unit, low]);
                } else {
                    out.push(REPLACEMENT);
                }
                i += 4;
            } else {
                out.push(if is_low(unit) { REPLACEMENT } else { unit });
                i += 2;
            }
        }
        if end && i < bytes.len() {
            out.push(REPLACEMENT);
            i = bytes.len();
        }
        i
    }

    /// Encodes `chars` onto `out`, after a byte order mark if `mark` is
    /// set and there is something to encode. An unpaired surrogate becomes
    /// U+FFFD, as in Java.
    pub(super) fn encode(self, chars: &[u16], mark: bool, out: &mut Vec<u8>) {
        let o = self.order();
        if mark && !chars.is_empty() {
            write(o, MARK, out);
        }
        let mut i = 0;
        while i < chars.len() {
            let unit = chars[i];
            if is_high(unit) && chars.get(i + 1).is_some_and(|&low| is_low(low)) {
                write(o, unit, out);
                write(o, chars[i + 1], out);
                i += 2;
                continue;
            }
            let lone = is_high(unit) || is_low(unit);
            write(o, if lone { REPLACEMENT } else { unit }, out);
            i += 1;
        }
    }
}

fn read(order: Order, pair: [u8; 2]) -> u16 {
    match order {
        Order::Big => u16::from_be_bytes(pair),
        Order::Little => u16::from_le_bytes(pair),
    }
}

fn write(order: Order, unit: u16, out: &mut Vec<u8>) {
    out.extend(match order {
        Order::Big => unit.to_be_bytes(),
        Order::Little => unit.to_le_bytes(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(kind: Utf16, bytes: &[u8]) -> Vec<u16> {
        let mut out = Vec::new();
        assert_eq!(kind.decode(&mut None, bytes, true, &mut out), bytes.len());
        out
    }

    fn encode(kind: Utf16, chars: &[u16]) -> Vec<u8> {
        let mut out = Vec::new();
        kind.encode(chars, kind.writes_mark(), &mut out);
        out
    }

    /// What the JDK 17 and 21 decoders give for the same bytes.
    #[test]
    fn decoding_matches_the_jdk() {
        use Utf16::{Big, Little, Marked, MarkedLittle};
        let cases: &[(Utf16, &[u8], &[u16])] = &[
            (Marked, &[0xFE, 0xFF, 0x00, 0x41], &[0x41]),
            (Marked, &[0xFF, 0xFE, 0x41, 0x00], &[0x41]),
            (Marked, &[0x00, 0x41, 0x00], &[0x41, 0xFFFD]),
            (Marked, &[0xD8, 0x00, 0x00, 0x41], &[0xFFFD]),
            (Marked, &[0x00, 0x41, 0xDC, 0x00], &[0x41, 0xFFFD]),
            (Marked, &[0xD8, 0x3D, 0xDE, 0x00], &[0xD83D, 0xDE00]),
            (
                Marked,
                &[0xFE, 0xFF, 0xFE, 0xFF, 0x00, 0x42],
                &[0xFEFF, 0x42],
            ),
            (Marked, &[0xFE], &[0xFFFD]),
            (Marked, &[0xFE, 0xFF], &[]),
            (Marked, &[0x41, 0x00, 0xD8, 0x00], &[0x4100, 0xFFFD]),
            (Big, &[0xFE, 0xFF, 0x00, 0x41], &[0xFEFF, 0x41]),
            (Big, &[0xFF, 0xFE, 0x41, 0x00], &[0xFFFE, 0x4100]),
            (Little, &[0xFF, 0xFE, 0x41, 0x00], &[0xFEFF, 0x41]),
            (Little, &[0xD8, 0x00, 0x00, 0x41], &[0x00D8, 0x4100]),
            (Little, &[0xFE], &[0xFFFD]),
            (MarkedLittle, &[0xFE, 0xFF, 0x00, 0x41], &[0x41]),
            (MarkedLittle, &[0x00, 0x41, 0x00], &[0x4100, 0xFFFD]),
            (
                MarkedLittle,
                &[0xFF, 0xFE, 0x3D, 0xD8, 0x00, 0xDE],
                &[0xD83D, 0xDE00],
            ),
        ];
        for &(kind, bytes, chars) in cases {
            assert_eq!(decode(kind, bytes), chars, "{kind:?} {bytes:02x?}");
        }
    }

    #[test]
    fn encoding_matches_the_jdk() {
        use Utf16::{Big, Little, Marked, MarkedLittle};
        let cases: &[(Utf16, &[u16], &[u8])] = &[
            (Marked, &[0x41], &[0xFE, 0xFF, 0x00, 0x41]),
            (Marked, &[], &[]),
            (Marked, &[0xD800], &[0xFE, 0xFF, 0xFF, 0xFD]),
            (Marked, &[0xFEFF], &[0xFE, 0xFF, 0xFE, 0xFF]),
            (
                Big,
                &[0x41, 0xD800, 0x42],
                &[0x00, 0x41, 0xFF, 0xFD, 0x00, 0x42],
            ),
            (Big, &[0xD83D, 0xDE00], &[0xD8, 0x3D, 0xDE, 0x00]),
            (Little, &[0xDC00], &[0xFD, 0xFF]),
            (Little, &[0xD83D, 0xDE00], &[0x3D, 0xD8, 0x00, 0xDE]),
            (MarkedLittle, &[0x41], &[0xFF, 0xFE, 0x41, 0x00]),
        ];
        for &(kind, chars, bytes) in cases {
            assert_eq!(encode(kind, chars), bytes, "{kind:?} {chars:04x?}");
        }
    }

    /// A reader feeds bytes as they come: the mark is read once, and a
    /// pair split between reads waits for its second half.
    #[test]
    fn a_stream_decodes_a_byte_at_a_time() {
        let bytes = [0xFF, 0xFE, 0x41, 0x00, 0x3D, 0xD8, 0x00, 0xDE, 0xFF, 0xFE];
        let mut order = None;
        let mut pending = Vec::new();
        let mut out = Vec::new();
        for &b in &bytes {
            pending.push(b);
            let used = Utf16::Marked.decode(&mut order, &pending, false, &mut out);
            pending.drain(..used);
        }
        assert_eq!(out, [0x41, 0xD83D, 0xDE00, 0xFEFF]);
        assert!(pending.is_empty());
    }
}
