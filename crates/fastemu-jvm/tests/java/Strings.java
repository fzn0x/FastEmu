// String, StringBuffer and StringBuilder methods, encodings and interning.
public class Strings {
    static String codes(String s) {
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < s.length(); i++) {
            if (i > 0) {
                sb.append(',');
            }
            sb.append(Integer.toHexString(s.charAt(i)));
        }
        return sb.toString();
    }

    static String bytes(byte[] b) {
        StringBuffer sb = new StringBuffer();
        for (int i = 0; i < b.length; i++) {
            sb.append(b[i] & 0xff).append(' ');
        }
        return sb.toString().trim();
    }

    public static void main(String[] args) throws Exception {
        String s = "Hello, World";
        System.out.println(s.length() + " " + s.charAt(4) + " " + s.indexOf('o') + " " + s.indexOf('o', 5) + " " + s.lastIndexOf('o'));
        System.out.println(s.indexOf("World") + " " + s.indexOf("xyz") + " " + s.lastIndexOf("l") + " " + s.lastIndexOf("l", 5) + " " + s.indexOf("", 3));
        System.out.println(s.substring(7) + "|" + s.substring(0, 5) + "|" + s.substring(5, 5) + "|");
        System.out.println(s.toUpperCase() + " " + s.toLowerCase() + " " + "  padded \t\n".trim() + "|" + "".trim() + "|");
        System.out.println(s.startsWith("Hell") + " " + s.startsWith("World", 7) + " " + s.endsWith("ld") + " " + s.endsWith(""));
        System.out.println(s.replace('l', 'L') + " " + s.concat("!") + " " + "a".compareTo("b") + " " + "abc".compareTo("ab") + " " + "B".compareTo("a"));
        System.out.println(s.equals("Hello, World") + " " + s.equals(null) + " " + s.equalsIgnoreCase("HELLO, world") + " " + "".isEmpty());
        System.out.println(s.regionMatches(7, "World!", 0, 5) + " " + s.regionMatches(true, 0, "hELLO", 0, 5) + " " + s.regionMatches(false, 0, "hELLO", 0, 5));
        System.out.println("hello".hashCode() + " " + "".hashCode() + " " + "polygenelubricants".hashCode() + " " + "é中".hashCode());
        char[] chars = s.toCharArray();
        chars[0] = 'J';
        System.out.println(new String(chars) + " " + new String(chars, 7, 5) + " " + String.valueOf(chars, 0, 3) + " " + String.copyValueOf(chars));
        char[] dst = new char[8];
        s.getChars(7, 12, dst, 1);
        System.out.println(codes(new String(dst)));
        System.out.println(String.valueOf(true) + String.valueOf('c') + String.valueOf(42) + String.valueOf(-7L) + String.valueOf((Object) null));
        String a = "inter" + "ned";
        String b = new String("interned");
        System.out.println((a == "interned") + " " + (b == "interned") + " " + (b.intern() == a) + " " + b.equals(a));
        System.out.println("abc".contains("bc") + " " + "x".contains("y"));

        StringBuffer sb = new StringBuffer("start");
        sb.append(1).append('c').append(2L).append(1.5f).append(2.5).append(true).append((Object) null).append(new char[] {'x', 'y'});
        System.out.println(sb + " " + sb.length());
        sb.insert(0, "<").insert(sb.length(), '>').insert(3, 99).insert(1, 1.25);
        System.out.println(sb);
        sb.reverse();
        System.out.println(sb);
        sb.setLength(5);
        sb.setCharAt(0, 'Z');
        sb.deleteCharAt(1);
        System.out.println(sb + " " + sb.charAt(2));
        sb.delete(1, 100);
        System.out.println(sb + "|" + sb.length());
        StringBuilder sb2 = new StringBuilder();
        for (int i = 0; i < 20; i++) {
            sb2.append(i % 10);
        }
        sb2.replace(2, 18, "--");
        System.out.println(sb2 + " " + sb2.indexOf("-") + " " + sb2.lastIndexOf("-") + " " + sb2.substring(1, 3));
        StringBuilder sb3 = new StringBuilder("ab😀cd");
        System.out.println(codes(sb3.reverse().toString()));

        int n = 5;
        String concat = "n=" + n + ", half=" + n / 2.0 + ", c=" + 'q' + ", b=" + (n > 3) + ", null=" + null + ", long=" + (1L << 40);
        System.out.println(concat);

        String unicode = "Жёé€😀!";
        System.out.println(codes(unicode) + " " + unicode.length());
        System.out.println(bytes(unicode.getBytes("UTF-8")));
        System.out.println(bytes(unicode.getBytes("ISO-8859-1")));
        System.out.println(bytes(unicode.getBytes("US-ASCII")));
        System.out.println(bytes("Привет Ёё".getBytes("windows-1251")));
        byte[] cp1251 = new byte[128];
        for (int i = 0; i < 128; i++) {
            cp1251[i] = (byte) (128 + i);
        }
        System.out.println(codes(new String(cp1251, "Cp1251")));
        System.out.println(codes(new String(cp1251, 0, 16, "ISO-8859-1")));
        System.out.println(codes(new String(new byte[] {(byte) 0xe2, (byte) 0x82, (byte) 0xac, 0x41, (byte) 0xff, 0x42}, "UTF-8")));
        try {
            "x".getBytes("NO-SUCH-CHARSET");
        } catch (java.io.UnsupportedEncodingException e) {
            System.out.println("unsupported " + e.getMessage());
        }
        System.out.println(codes("İßǅ".toLowerCase()) + " " + codes("éǅ".toUpperCase()));

        try {
            s.charAt(99);
        } catch (StringIndexOutOfBoundsException e) {
            System.out.println("charAt: StringIndexOutOfBoundsException");
        }
        try {
            s.substring(5, 2);
        } catch (IndexOutOfBoundsException e) {
            System.out.println("substring: " + e.getClass().getName());
        }
        try {
            new StringBuffer("ab").insert(5, "x");
        } catch (StringIndexOutOfBoundsException e) {
            System.out.println("insert: StringIndexOutOfBoundsException");
        }
        String nul = null;
        try {
            nul.length();
        } catch (NullPointerException e) {
            System.out.println("null.length: NullPointerException");
        }
    }
}
