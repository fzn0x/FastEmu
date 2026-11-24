public class Hello {
    static int fib(int n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }
    public static void main(String[] args) {
        System.out.println("Hello, world");
        int sum = 0;
        for (int i = 0; i < 100; i++) sum += i;
        System.out.println("sum=" + sum + " fib=" + fib(20));
        long l = 1L << 40;
        System.out.println(l + " " + (l * 3) + " " + 1.5 + " " + (1.0/3) + " " + 0.1f);
        StringBuilder sb = new StringBuilder();
        for (char c = 'a'; c <= 'e'; c++) sb.append(c);
        System.out.println(sb.reverse().toString());
        try {
            Object o = null;
            o.hashCode();
        } catch (NullPointerException e) {
            System.out.println("caught " + e.getClass().getName());
        }
        int[] arr = new int[3];
        try { arr[5] = 1; } catch (ArrayIndexOutOfBoundsException e) { System.out.println(e.getMessage()); }
        switch ("two") { case "one": System.out.println(1); break; case "two": System.out.println(2); break; default: System.out.println(0); }
        throw new RuntimeException("boom");
    }
}
