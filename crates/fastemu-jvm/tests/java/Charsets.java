import java.io.ByteArrayInputStream;
import java.io.ByteArrayOutputStream;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.io.UnsupportedEncodingException;

// The charsets games from Asia and Europe use, through String, the
// readers and the writers, under Java's names and aliases. The
// multi-byte charsets are checked on text of their national standards,
// where Java's tables and the WHATWG ones FastEmu uses agree; the
// single-byte ones and UTF-16 on every byte and on malformed input.
public class Charsets {
    static String hex(byte[] b) {
        StringBuffer s = new StringBuffer();
        for (int i = 0; i < b.length; i++) {
            s.append(Integer.toHexString((b[i] & 0xFF) | 0x100).substring(1));
        }
        return s.toString();
    }

    static String units(String t) {
        StringBuffer s = new StringBuffer();
        for (int i = 0; i < t.length(); i++) {
            s.append(Integer.toHexString(t.charAt(i) | 0x10000).substring(1)).append(' ');
        }
        return s.toString().trim();
    }

    static byte[] bytes(int[] v) {
        byte[] r = new byte[v.length];
        for (int i = 0; i < v.length; i++) {
            r[i] = (byte) v[i];
        }
        return r;
    }

    static void names() {
        String[] names = {
            "Shift_JIS", "SJIS", "shift-jis", "x-sjis", "MS932", "windows-31j", "csShiftJIS",
            "EUC-JP", "EUC_JP", "eucjis", "x-euc-jp", "ISO-2022-JP", "JIS", "iso2022jp",
            "GBK", "CP936", "GB2312", "EUC_CN", "gb2312-80", "GB18030", "Big5", "csBig5",
            "Big5-HKSCS", "MS950", "EUC-KR", "EUC_KR", "KSC5601", "ks_c_5601-1987", "MS949",
            "UTF-16", "UnicodeBig", "UTF-16BE", "UnicodeBigUnmarked", "UTF-16LE",
            "UnicodeLittleUnmarked", "x-UTF-16LE-BOM", "UnicodeLittle", "ISO-8859-3",
            "ISO8859_4", "ISO-8859-5", "cyrillic", "ISO-8859-6", "ISO-8859-7", "greek",
            "ISO-8859-8", "ISO-8859-9", "latin5", "ISO-8859-13", "ISO-8859-15", "LATIN9",
            "ISO-8859-16", "ISO-8859-10", "ISO-8859-14", "x-unknown"
        };
        for (int i = 0; i < names.length; i++) {
            try {
                new String(new byte[] {65}, names[i]);
                System.out.println(names[i] + " ok");
            } catch (UnsupportedEncodingException e) {
                System.out.println(names[i] + " unsupported");
            }
        }
    }

    // Every byte of a single-byte charset, and what every character up to
    // U+2FFF encodes to.
    static void singleByte(String cs) throws Exception {
        byte[] all = new byte[128];
        for (int i = 0; i < 128; i++) {
            all[i] = (byte) (0x80 + i);
        }
        System.out.println(cs + " high " + units(new String(all, cs)));
        char[] chars = new char[0x3000];
        for (int i = 0; i < chars.length; i++) {
            chars[i] = (char) i;
        }
        byte[] enc = new String(chars).getBytes(cs);
        int hash = 0;
        for (int i = 0; i < enc.length; i++) {
            hash = hash * 31 + (enc[i] & 0xFF);
        }
        System.out.println(cs + " encodes " + enc.length + " bytes, hash " + hash);
    }

    static void utf16(String cs) throws Exception {
        int[][] ins = {
            {0xFE, 0xFF, 0x00, 0x41}, {0xFF, 0xFE, 0x41, 0x00}, {0x00, 0x41, 0x00},
            {0xD8, 0x00, 0x00, 0x41}, {0x00, 0x41, 0xDC, 0x00}, {0xD8, 0x3D, 0xDE, 0x00},
            {0xFE, 0xFF, 0xFE, 0xFF, 0x00, 0x42}, {}, {0xFE}, {0xFE, 0xFF},
            {0x41, 0x00, 0xD8, 0x00}, {0xD8, 0x00}, {0xFF, 0xFE, 0x3D, 0xD8, 0x00, 0xDE},
            {0xD8, 0x00, 0xDC}, {0xD8, 0x00, 0xD8, 0x00, 0xDC, 0x00}
        };
        for (int i = 0; i < ins.length; i++) {
            byte[] in = bytes(ins[i]);
            System.out.println(cs + " decode " + hex(in) + " = " + units(new String(in, cs)));
        }
        String[] outs = {"A", "", "\uD800", "A\uD800B", "\uDC00", "\uD83D\uDE00", "\uFEFF", "\u3042x"};
        for (int i = 0; i < outs.length; i++) {
            System.out.println(cs + " encode " + units(outs[i]) + " = " + hex(outs[i].getBytes(cs)));
        }
    }

    static void text(String cs, String text) throws Exception {
        byte[] enc = text.getBytes(cs);
        String back = new String(enc, cs);
        System.out.println(cs + " " + hex(enc) + " round trip " + back.equals(text));
    }

    static void decodes(String cs, int[] in) throws Exception {
        byte[] b = bytes(in);
        System.out.println(cs + " decode " + hex(b) + " = " + units(new String(b, cs)));
    }

    // A reader that takes the bytes as they come, and a writer that
    // writes in pieces.
    static void streams(String cs, String text) throws Exception {
        ByteArrayOutputStream out = new ByteArrayOutputStream();
        OutputStreamWriter w = new OutputStreamWriter(out, cs);
        for (int i = 0; i < text.length(); i += 3) {
            w.write(text, i, Math.min(3, text.length() - i));
        }
        w.close();
        byte[] written = out.toByteArray();
        System.out.println(cs + " writer " + hex(written));
        InputStreamReader r = new InputStreamReader(new ByteArrayInputStream(written), cs);
        StringBuffer read = new StringBuffer();
        int c;
        while ((c = r.read()) != -1) {
            read.append((char) c);
        }
        System.out.println(cs + " reader " + read.toString().equals(text) + " " + units(read.toString()));
    }

    public static void main(String[] args) throws Exception {
        names();
        String[] single = {
            "ISO-8859-2", "ISO-8859-3", "ISO-8859-4", "ISO-8859-5", "ISO-8859-6", "ISO-8859-7",
            "ISO-8859-8", "ISO-8859-9", "ISO-8859-13", "ISO-8859-15", "ISO-8859-16",
            "windows-1250", "windows-1251", "windows-1252", "KOI8-R"
        };
        for (int i = 0; i < single.length; i++) {
            singleByte(single[i]);
        }
        String[] u16 = {"UTF-16", "UTF-16BE", "UTF-16LE", "x-UTF-16LE-BOM"};
        for (int i = 0; i < u16.length; i++) {
            utf16(u16[i]);
        }

        String japanese = "ゲームを始める。スコア：１２３　ﾊｲｽｺｱ 日本語 ABC";
        // WHATWG's ISO-2022-JP writes halfwidth katakana as fullwidth,
        // where Java switches to JIS X 0201 with ESC ( I.
        String jis = "ゲームを始める。スコア：１２３　日本語 ABC";
        String simplified = "你好，世界。开始游戏！得分：１２３";
        String traditional = "你好，世界。開始遊戲！得分";
        String korean = "안녕하세요. 게임 시작! 점수";
        String[] jp = {"Shift_JIS", "MS932", "EUC-JP"};
        for (int i = 0; i < jp.length; i++) {
            text(jp[i], japanese);
        }
        text("ISO-2022-JP", jis);
        String[] cn = {"GBK", "GB2312", "GB18030"};
        for (int i = 0; i < cn.length; i++) {
            text(cn[i], simplified);
        }
        text("Big5", traditional);
        text("EUC-KR", korean);
        text("Shift_JIS", korean);
        text("EUC-KR", "x\uD83D\uDE00y\uD800z");

        // Malformed input where the two agree.
        decodes("Shift_JIS", new int[] {0x41, 0x81});
        decodes("Shift_JIS", new int[] {0x81, 0x20});
        decodes("Shift_JIS", new int[] {0xA0, 0x41});
        decodes("EUC-KR", new int[] {0xB0, 0x20});
        decodes("GBK", new int[] {0x81, 0x7F});
        decodes("Big5", new int[] {0xA4});
        decodes("ISO-2022-JP", new int[] {0x1B, 0x24, 0x42, 0x30, 0x21, 0x1B, 0x28, 0x42, 0x41});

        streams("Shift_JIS", japanese);
        streams("EUC-JP", japanese);
        streams("ISO-2022-JP", jis);
        streams("GBK", simplified);
        streams("Big5", traditional);
        streams("EUC-KR", korean);
        streams("UTF-16", japanese);
        streams("UTF-16LE", "a\uD83D\uDE00b");
        streams("x-UTF-16LE-BOM", korean);
        streams("ISO-8859-7", "Καλημέρα");
    }
}
