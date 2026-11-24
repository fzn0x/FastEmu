/** Output helpers shared by the record store test programs. */
public class RmsTest {
    static void say(String line) {
        System.out.println(line);
    }

    /** The simple name of an exception's class. */
    static String name(Throwable t) {
        String n = t.getClass().getName();
        return n.substring(n.lastIndexOf('.') + 1);
    }

    /** Text with every character outside printable ASCII as \\uXXXX. */
    static String esc(String s) {
        StringBuffer out = new StringBuffer();
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            if (c >= 0x20 && c < 0x7f) {
                out.append(c);
            } else {
                String hex = Integer.toHexString(c);
                out.append("\\u");
                for (int pad = hex.length(); pad < 4; pad++) {
                    out.append('0');
                }
                out.append(hex);
            }
        }
        return out.toString();
    }

    /** The names listRecordStores gives, escaped, or "null". */
    static String stores(String[] names) {
        if (names == null) {
            return "null";
        }
        StringBuffer out = new StringBuffer();
        for (int i = 0; i < names.length; i++) {
            if (i > 0) {
                out.append(' ');
            }
            out.append(esc(names[i]));
        }
        return out.toString();
    }
}
