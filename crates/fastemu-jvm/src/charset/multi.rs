//! The multi-byte charsets of East Asia, through the encoding_rs crate:
//! Shift_JIS, EUC-JP, ISO-2022-JP, GBK (and GB2312), GB18030, Big5 and
//! EUC-KR.
//!
//! encoding_rs follows the WHATWG Encoding Standard, whose tables are the
//! ones Windows and the web use: Shift_JIS is Microsoft's code page 932,
//! GBK code page 936, Big5 with the Hong Kong additions and EUC-KR code
//! page 949. Each is a superset of the national standard Java's charset of
//! that name follows, so text in the standard decodes as in Java, and the
//! vendor rows a strict Java charset rejects (NEC's circled numbers in
//! Shift_JIS, say) decode instead of becoming U+FFFD. Malformed bytes
//! become U+FFFD as the standard says, which differs from Java's count for
//! some sequences. What a charset cannot encode becomes `?`, as in Java.

use encoding_rs::{CoderResult, Decoder, Encoder, EncoderResult, Encoding};

/// How many units a conversion takes at once, so its buffer stays small.
const CHUNK: usize = 4096;

/// Decodes `bytes` onto `out`. Unless `last` is set, a sequence cut off
/// at the end waits in `decoder` for the next call.
pub(super) fn decode(decoder: &mut Decoder, bytes: &[u8], last: bool, out: &mut Vec<u16>) {
    let mut src = bytes;
    loop {
        let take = src.len().min(CHUNK);
        let room = decoder
            .max_utf16_buffer_length(take)
            .unwrap_or(CHUNK * 2)
            .max(4);
        let at = out.len();
        out.resize(at + room, 0);
        let (result, read, written, _) = decoder.decode_to_utf16(src, &mut out[at..], last);
        out.truncate(at + written);
        src = &src[read..];
        if result == CoderResult::InputEmpty {
            return;
        }
    }
}

/// A fresh decoder for `encoding`, which leaves byte order marks alone,
/// as Java's charsets do.
pub(super) fn decoder(encoding: &'static Encoding) -> Decoder {
    encoding.new_decoder_without_bom_handling()
}

/// Encodes `chars` onto `out`. Unpaired surrogates and characters the
/// charset lacks become `?`, which goes through the encoder too, so a
/// charset with shift states (ISO-2022-JP) switches back to ASCII for it.
/// With `last`, a stateful encoder returns to its initial state.
pub(super) fn encode(encoder: &mut Encoder, chars: &[u16], last: bool, out: &mut Vec<u8>) {
    let mut start = 0;
    let mut i = 0;
    while i < chars.len() {
        let unit = chars[i];
        let high = (0xD800..0xDC00).contains(&unit);
        if high
            && chars
                .get(i + 1)
                .is_some_and(|c| (0xDC00..0xE000).contains(c))
        {
            i += 2;
            continue;
        }
        if (0xD800..0xE000).contains(&unit) {
            encode_run(encoder, &chars[start..i], false, out);
            encode_run(encoder, &[u16::from(b'?')], false, out);
            start = i + 1;
        }
        i += 1;
    }
    encode_run(encoder, &chars[start..], last, out);
}

/// Encodes `chars`, which hold no unpaired surrogate.
fn encode_run(encoder: &mut Encoder, chars: &[u16], last: bool, out: &mut Vec<u8>) {
    let mut src = chars;
    loop {
        let take = src.len().min(CHUNK);
        let room = encoder
            .max_buffer_length_from_utf16_without_replacement(take)
            .unwrap_or(CHUNK * 8)
            .max(16);
        let at = out.len();
        out.resize(at + room, 0);
        let (result, read, written) =
            encoder.encode_from_utf16_without_replacement(src, &mut out[at..], last);
        out.truncate(at + written);
        src = &src[read..];
        match result {
            EncoderResult::InputEmpty => return,
            EncoderResult::OutputFull => {}
            EncoderResult::Unmappable(_) => encode_run(encoder, &[u16::from(b'?')], false, out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_all(encoding: &'static Encoding, bytes: &[u8]) -> Vec<u16> {
        let mut out = Vec::new();
        decode(&mut decoder(encoding), bytes, true, &mut out);
        out
    }

    fn encode_all(encoding: &'static Encoding, text: &str) -> Vec<u8> {
        let chars: Vec<u16> = text.encode_utf16().collect();
        let mut out = Vec::new();
        encode(&mut encoding.new_encoder(), &chars, true, &mut out);
        out
    }

    fn utf16(s: &str) -> Vec<u16> {
        s.encode_utf16().collect()
    }

    /// Bytes and text as the JDK converts them.
    #[test]
    fn common_text_converts_as_in_java() {
        let cases: &[(&'static Encoding, &str, &[u8])] = &[
            (
                encoding_rs::SHIFT_JIS,
                "\u{3042}\u{30a2}\u{ff71}",
                &[0x82, 0xA0, 0x83, 0x41, 0xB1],
            ),
            (
                encoding_rs::SHIFT_JIS,
                "\u{4e9c}\u{6f22}\u{5b57}",
                &[0x88, 0x9F, 0x8A, 0xBF, 0x8E, 0x9A],
            ),
            (
                encoding_rs::EUC_JP,
                "\u{3042}\u{30a2}\u{ff71}",
                &[0xA4, 0xA2, 0xA5, 0xA2, 0x8E, 0xB1],
            ),
            (
                encoding_rs::GBK,
                "\u{4e9c}\u{6f22}\u{5b57}",
                &[0x81, 0x84, 0x9D, 0x68, 0xD7, 0xD6],
            ),
            (encoding_rs::BIG5, "\u{4e00}", &[0xA4, 0x40]),
            (
                encoding_rs::EUC_KR,
                "\u{ac00}\u{d55c}",
                &[0xB0, 0xA1, 0xC7, 0xD1],
            ),
            (
                encoding_rs::ISO_2022_JP,
                "\u{4e9c}\u{6f22}\u{5b57}",
                &[
                    0x1B, 0x24, 0x42, 0x30, 0x21, 0x34, 0x41, 0x3B, 0x7A, 0x1B, 0x28, 0x42,
                ],
            ),
        ];
        for &(encoding, text, bytes) in cases {
            assert_eq!(encode_all(encoding, text), bytes, "{encoding:?} {text}");
            assert_eq!(decode_all(encoding, bytes), utf16(text), "{encoding:?}");
        }
    }

    #[test]
    fn what_a_charset_lacks_becomes_a_question_mark() {
        assert_eq!(encode_all(encoding_rs::SHIFT_JIS, "\u{ac00}x"), b"?x");
        assert_eq!(encode_all(encoding_rs::EUC_KR, "\u{1F600}"), b"?");
        let mut out = Vec::new();
        let lone = [0xD800, u16::from(b'x')];
        encode(
            &mut encoding_rs::GB18030.new_encoder(),
            &lone,
            true,
            &mut out,
        );
        assert_eq!(
            out, b"?x",
            "a lone surrogate, which GB18030 could encode as U+FFFD"
        );
        // In ISO-2022-JP the ? comes back to ASCII first.
        assert_eq!(
            encode_all(encoding_rs::ISO_2022_JP, "\u{3042}\u{ac00}"),
            [0x1B, 0x24, 0x42, 0x24, 0x22, 0x1B, 0x28, 0x42, b'?']
        );
    }

    #[test]
    fn a_sequence_cut_off_waits_for_the_next_bytes() {
        let mut d = decoder(encoding_rs::SHIFT_JIS);
        let mut out = Vec::new();
        decode(&mut d, &[0x41, 0x82], false, &mut out);
        assert_eq!(out, [0x41]);
        decode(&mut d, &[0xA0], false, &mut out);
        assert_eq!(out, [0x41, 0x3042]);
        decode(&mut d, &[0x81], true, &mut out);
        assert_eq!(out, [0x41, 0x3042, 0xFFFD], "cut off at the end");
        assert_eq!(
            decode_all(encoding_rs::SHIFT_JIS, &[0x81, 0x20]),
            [0xFFFD, 0x20]
        );
    }
}
