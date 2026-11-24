// The wrapper classes: parsing, printing, conversions and characters.
public class Wrappers {
    static void parse(String s, int radix) {
        try {
            System.out.println("parseInt(" + s + "," + radix + ") = " + Integer.parseInt(s, radix));
        } catch (NumberFormatException e) {
            System.out.println("parseInt(" + s + "," + radix + ") NumberFormatException");
        }
    }

    static void parseLong(String s) {
        try {
            System.out.println("parseLong(" + s + ") = " + Long.parseLong(s));
        } catch (NumberFormatException e) {
            System.out.println("parseLong(" + s + ") NumberFormatException");
        }
    }

    public static void main(String[] args) {
        String[] ints = {"0", "-0", "+5", "2147483647", "-2147483648", "2147483648", "", "-", "12 ", "0x10", "١٢"};
        for (int i = 0; i < ints.length; i++) {
            parse(ints[i], 10);
        }
        parse("ff", 16);
        parse("-FF", 16);
        parse("777", 8);
        parse("1010", 2);
        parse("zz", 36);
        parse("12", 1);
        parseLong("9223372036854775807");
        parseLong("-9223372036854775808");
        parseLong("9223372036854775808");
        try {
            Short.parseShort("40000");
        } catch (NumberFormatException e) {
            System.out.println("short overflow");
        }
        System.out.println(Short.parseShort("-123") + " " + Byte.parseByte("7f", 16) + " " + Short.MAX_VALUE + " " + Byte.MIN_VALUE);
        System.out.println(Integer.toString(255, 16) + " " + Integer.toString(-255, 2) + " " + Integer.toString(10, 99) + " " + Long.toString(-1L, 36));
        System.out.println(Integer.toHexString(-1) + " " + Integer.toOctalString(8) + " " + Integer.toBinaryString(10) + " " + Long.toHexString(Long.MIN_VALUE));
        Integer a = new Integer(1000);
        Integer b = new Integer(1000);
        System.out.println(a.equals(b) + " " + (a == b) + " " + a.hashCode() + " " + a.compareTo(new Integer(5)) + " " + a.toString() + " " + a.byteValue());
        Long l = new Long(1L << 33);
        System.out.println(l.hashCode() + " " + l.intValue() + " " + l.doubleValue() + " " + l.equals(new Long(1L << 33)) + " " + l.equals(a));
        Integer boxed = 42;
        int unboxed = boxed + 1;
        System.out.println(unboxed + " " + Integer.valueOf(7).intValue() + " " + Integer.valueOf("-12") + " " + Long.valueOf(99L));
        System.out.println(Boolean.TRUE + " " + Boolean.FALSE.booleanValue() + " " + new Boolean(true).hashCode() + " " + new Boolean(false).hashCode() + " " + Boolean.valueOf(true).equals(Boolean.TRUE));
        Character c = new Character('x');
        System.out.println(c.charValue() + " " + c.hashCode() + " " + c.equals(new Character('x')) + " " + c);
        char[] cs = {'a', 'Z', '5', ' ', '\t', 'é', 'Ж', '١', '_', ' ', ' ', '１'};
        for (int i = 0; i < cs.length; i++) {
            char ch = cs[i];
            System.out.println(Integer.toHexString(ch) + ": digit=" + Character.isDigit(ch) + " letter=" + Character.isLetter(ch)
                + " lower=" + Character.isLowerCase(ch) + " upper=" + Character.isUpperCase(ch) + " ws=" + Character.isWhitespace(ch)
                + " space=" + Character.isSpaceChar(ch) + " lc=" + Integer.toHexString(Character.toLowerCase(ch))
                + " uc=" + Integer.toHexString(Character.toUpperCase(ch)) + " d16=" + Character.digit(ch, 16));
        }
        System.out.println(Character.forDigit(11, 16) + " " + (int) Character.forDigit(5, 2) + " " + Character.forDigit(7, 10) + " " + Character.digit('z', 36) + " " + Character.MAX_RADIX);
        System.out.println(Integer.MAX_VALUE + " " + Integer.MIN_VALUE + " " + Long.MAX_VALUE + " " + (int) Character.MAX_VALUE);
        Object[] numbers = {new Integer(3), new Long(4), new Float(5.5f), new Double(6.25), new Short((short) 7), new Byte((byte) 8)};
        double total = 0;
        for (int i = 0; i < numbers.length; i++) {
            total += ((Number) numbers[i]).doubleValue();
            System.out.print(((Number) numbers[i]).intValue() + " ");
        }
        System.out.println(total);
    }
}
