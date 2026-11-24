// Float and double arithmetic, conversions, comparisons and printing.
public class FloatMath {
    static void show(String label, double d) {
        System.out.println(label + " " + d + " " + (float) d);
    }

    public static void main(String[] args) {
        double[] ds = {
            0.0, -0.0, 1.0, -1.5, 0.1, 0.2, 0.1 + 0.2, 1.0 / 3, 2.0 / 3, 100.0, 1e7, 9999999.0,
            12345678.9, 1e-3, 9.99e-4, 1e-4, 123.456, 1e21, 1e22, 1e23, 3.14159, Math.PI, Math.E,
            Double.MAX_VALUE, Double.MIN_VALUE, 4.35, 0.5, 1234567.0, 0.001234, 5e-5, 2.5e-10,
            1.7976931348623157E308, 4.9E-324 * 3, 1e300 * 1e10, -1e300 * 1e10, 0.0 / 0.0
        };
        for (int i = 0; i < ds.length; i++) {
            show("d" + i, ds[i]);
        }
        float[] fs = {0.1f, 0.2f, 0.1f + 0.2f, 1f / 3, 16777216f, 16777217f, 3.4e38f, 1.4e-45f, 1e-5f, 1e7f, 123.456f, 0.001f, 7.0E-4f};
        for (int i = 0; i < fs.length; i++) {
            System.out.println("f" + i + " " + fs[i] + " " + (double) fs[i]);
        }

        double nan = Double.NaN;
        double inf = Double.POSITIVE_INFINITY;
        System.out.println((nan == nan) + " " + (nan != nan) + " " + (nan < 1) + " " + (nan > 1) + " " + (nan >= nan));
        System.out.println((inf > Double.MAX_VALUE) + " " + (-inf < -Double.MAX_VALUE) + " " + (0.0 == -0.0));
        System.out.println((int) nan + " " + (long) nan + " " + (int) inf + " " + (long) -inf);
        System.out.println((int) 3.99 + " " + (int) -3.99 + " " + (int) 1e20 + " " + (long) 1e20 + " " + (long) -1e20);
        System.out.println((int) Float.NaN + " " + (int) 3e10f + " " + (long) -3e30f + " " + (char) 70000.5);
        System.out.println((float) 1e40 + " " + (float) -1e-50 + " " + (double) (float) 0.1);
        System.out.println(5.0 % 3 + " " + -5.0 % 3 + " " + 5.5 % -2 + " " + 1.0 % 0 + " " + 5.0f % 2.5f);
        System.out.println(1.0 / 0 + " " + -1.0 / 0 + " " + 0.0 / 0 + " " + -0.0 * 5 + " " + 1 / -inf);
        System.out.println(Math.sqrt(2) + " " + Math.sqrt(-1) + " " + Math.floor(-2.5) + " " + Math.ceil(-2.5) + " " + Math.ceil(-0.5));
        System.out.println(Math.abs(-0.0) + " " + Math.abs(-3.5f) + " " + Math.max(-0.0, 0.0) + " " + Math.min(-0.0, 0.0));
        System.out.println(Math.max(nan, 1) + " " + Math.min(1f, Float.NaN) + " " + Math.max(2.5f, 1.5f));
        System.out.println(Math.round(2.5) + " " + Math.round(-2.5) + " " + Math.round(0.49999999999999994) + " " + Math.round(-0.5f) + " " + Math.round(1e20));
        System.out.println(Math.toRadians(180) + " " + Math.toDegrees(Math.PI / 2));
        System.out.println((int) (Math.sin(1) * 1e6) + " " + (int) (Math.cos(1) * 1e6) + " " + (int) (Math.tan(1) * 1e6));

        float acc = 0;
        for (int i = 0; i < 1000; i++) {
            acc += 0.1f;
        }
        System.out.println(acc);
        double dacc = 0;
        for (int i = 0; i < 1000; i++) {
            dacc += 0.1;
        }
        System.out.println(dacc);
        float fx = 1.1f;
        double dx = fx;
        System.out.println(dx + " " + (fx * fx) + " " + (dx * dx) + " " + (fx / 3) + " " + (float) (dx / 3));
        System.out.println(Float.floatToIntBits(1.5f) + " " + Float.floatToIntBits(Float.NaN) + " " + Float.intBitsToFloat(0x40490fdb));
        System.out.println(Double.doubleToLongBits(-2.0) + " " + Double.longBitsToDouble(0x3ff8000000000000L));
        System.out.println(Double.isNaN(nan) + " " + Double.isInfinite(-inf) + " " + Float.isNaN(1f) + " " + new Float(2.5f).isNaN());
        System.out.println(new Double(0.0).equals(new Double(-0.0)) + " " + new Double(nan).equals(new Double(nan)) + " " + new Float(1.5f).hashCode() + " " + new Double(1.5).hashCode());

        String[] texts = {"1.5", "  -2.25  ", "1e3", ".5", "5.", "3.0f", "1D", "NaN", "-Infinity", "0x1.8p1", "1e400", "1e-400", "abc", "", "1.2.3", "infinity", "+7"};
        for (int i = 0; i < texts.length; i++) {
            try {
                System.out.println("parse '" + texts[i] + "' " + Double.parseDouble(texts[i]) + " " + Float.parseFloat(texts[i]));
            } catch (NumberFormatException e) {
                System.out.println("parse '" + texts[i] + "' NumberFormatException");
            }
        }
        System.out.println(Double.valueOf("2.5").doubleValue() + " " + Float.valueOf("0.25").floatValue() + " " + new Double(7.9).intValue() + " " + new Float(-7.9f).longValue());
        System.out.println(String.valueOf(1.0f) + String.valueOf(2.5) + Float.toString(-0.0f) + Double.toString(1e-7));
        long bits = Double.doubleToLongBits(0.1);
        System.out.println(Long.toHexString(bits) + " " + Integer.toHexString(Float.floatToIntBits(0.1f)));
        System.out.println(0.1f == 0.1 ? "same" : "different");
        System.out.println(Float.MAX_VALUE + " " + Float.MIN_VALUE + " " + Double.MIN_VALUE + " " + Float.NEGATIVE_INFINITY);
    }
}
