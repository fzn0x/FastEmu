// Arrays of every type, multianewarray, clone and System.arraycopy.
public class Arrays {
    static String dump(int[] a) {
        StringBuffer sb = new StringBuffer("[");
        for (int i = 0; i < a.length; i++) {
            sb.append(i == 0 ? "" : ",").append(a[i]);
        }
        return sb.append(']').toString();
    }

    static String dump(Object[] a) {
        StringBuffer sb = new StringBuffer("[");
        for (int i = 0; i < a.length; i++) {
            sb.append(i == 0 ? "" : ",").append(a[i]);
        }
        return sb.append(']').toString();
    }

    public static void main(String[] args) {
        boolean[] bools = new boolean[2];
        byte[] bytes = {(byte) 255, 127, -128};
        char[] chars = new char[] {'a', 'Ж', 0xFFFF};
        short[] shorts = {-1, 32767, (short) 40000};
        long[] longs = {Long.MIN_VALUE, 0, 1L << 50};
        float[] floats = {1.5f, -0.0f, Float.NaN};
        double[] doubles = new double[3];
        doubles[1] = 2.25;
        bools[1] = true;
        System.out.println(bools[0] + " " + bools[1] + " " + bytes[0] + " " + (int) chars[1] + " " + (int) chars[2] + " " + shorts[2]);
        System.out.println(longs[0] + " " + longs[2] + " " + floats[1] + " " + floats[2] + " " + doubles[0] + " " + doubles[1]);
        bytes[1]++;
        shorts[0] += 2;
        longs[1] -= 5;
        chars[0] += 1;
        System.out.println(bytes[1] + " " + shorts[0] + " " + longs[1] + " " + chars[0]);

        int[][] grid = new int[3][4];
        for (int i = 0; i < 3; i++) {
            for (int j = 0; j < 4; j++) {
                grid[i][j] = i * 10 + j;
            }
        }
        System.out.println(dump(grid[2]) + " " + grid.length + " " + grid[0].length);
        long[][][] cube = new long[2][3][4];
        cube[1][2][3] = 99;
        System.out.println(cube[1][2][3] + " " + cube[0][1][2] + " " + cube[1].length + " " + cube[1][2].length);
        String[][] partial = new String[3][];
        System.out.println((partial[0] == null) + " " + partial.length);
        partial[1] = new String[] {"x", "y"};
        System.out.println(dump(partial[1]));
        int[][] jagged = {{1}, {2, 3}, {}, {4, 5, 6}};
        int total = 0;
        for (int i = 0; i < jagged.length; i++) {
            for (int j = 0; j < jagged[i].length; j++) {
                total += jagged[i][j];
            }
        }
        System.out.println(total + " " + jagged[2].length);
        Object[][] zero = new Object[0][5];
        System.out.println(zero.length);
        char[][][] ccc = new char[2][0][7];
        System.out.println(ccc.length + " " + ccc[1].length);

        int[] src = {1, 2, 3, 4, 5, 6, 7, 8};
        int[] copy = (int[]) src.clone();
        copy[0] = 100;
        System.out.println(dump(src) + " " + dump(copy));
        System.arraycopy(src, 0, src, 2, 5);
        System.out.println(dump(src));
        System.arraycopy(src, 3, src, 0, 5);
        System.out.println(dump(src));
        int[] dst = new int[4];
        System.arraycopy(src, 4, dst, 1, 3);
        System.out.println(dump(dst));
        String[] names = {"a", "b", "c"};
        Object[] objs = new Object[4];
        System.arraycopy(names, 0, objs, 1, 3);
        System.out.println(dump(objs));
        Object[] mixed = {"s", new Integer(1), "t"};
        String[] strs = new String[3];
        try {
            System.arraycopy(mixed, 0, strs, 0, 3);
        } catch (ArrayStoreException e) {
            System.out.println("ArrayStoreException after " + dump(strs));
        }
        String[][] nested = {{"p"}, {"q", "r"}};
        String[][] shallow = (String[][]) nested.clone();
        shallow[1][0] = "changed";
        System.out.println(nested[1][0] + " " + (shallow[0] == nested[0]));
        int[] errors = new int[0];
        String[] tests = {"null src", "negative length", "src overflow", "dst overflow", "type mismatch", "not an array"};
        for (int i = 0; i < tests.length; i++) {
            try {
                switch (i) {
                    case 0: System.arraycopy(null, 0, dst, 0, 1); break;
                    case 1: System.arraycopy(src, 0, dst, 0, -1); break;
                    case 2: System.arraycopy(src, 6, dst, 0, 3); break;
                    case 3: System.arraycopy(src, 0, dst, 2, 3); break;
                    case 4: System.arraycopy(src, 0, new long[8], 0, 1); break;
                    default: System.arraycopy("str", 0, dst, 0, 1);
                }
                System.out.println(tests[i] + ": no error");
            } catch (RuntimeException e) {
                System.out.println(tests[i] + ": " + e.getClass().getName());
            }
        }
        System.out.println(errors.length + " " + dump(dst));
        Object o = new int[5];
        int[] back = (int[]) o;
        System.out.println(back.length);
        Runnable[] tasks = new Runnable[2];
        System.out.println(tasks.getClass().getName() + " " + (tasks[0] == null));
        int sum = 0;
        int[] big = new int[10000];
        for (int i = 0; i < big.length; i++) {
            big[i] = i * i;
        }
        for (int i = 0; i < big.length; i += 7) {
            sum += big[i];
        }
        System.out.println(sum);
        for (int v : new int[] {3, 1, 4, 1, 5}) {
            System.out.print(v);
        }
        System.out.println();
    }
}
