// Deep recursion, mutual recursion, and recovering from StackOverflowError.
public class Recursion {
    static int depth;

    static long sum(int n) {
        return n == 0 ? 0 : n + sum(n - 1);
    }

    static boolean isEven(int n) {
        return n == 0 || isOdd(n - 1);
    }

    static boolean isOdd(int n) {
        return n != 0 && isEven(n - 1);
    }

    static int fib(int n) {
        return n < 2 ? n : fib(n - 1) + fib(n - 2);
    }

    static int ackermann(int m, int n) {
        if (m == 0) {
            return n + 1;
        }
        if (n == 0) {
            return ackermann(m - 1, 1);
        }
        return ackermann(m - 1, ackermann(m, n - 1));
    }

    static class Nest {
        final int depth;

        Nest(int depth) {
            this.depth = depth;
        }

        public String toString() {
            return depth == 0 ? "x" : "(" + new Nest(depth - 1) + ")";
        }
    }

    static void forever() {
        depth++;
        forever();
    }

    static String hanoi(int n, char from, char to, char via) {
        if (n == 0) {
            return "";
        }
        return hanoi(n - 1, from, via, to) + from + to + hanoi(n - 1, via, to, from);
    }

    public static void main(String[] args) {
        System.out.println(sum(3000));
        System.out.println(isEven(2001) + " " + isOdd(2001));
        System.out.println(fib(24));
        System.out.println(ackermann(2, 300));
        System.out.println(hanoi(4, 'A', 'C', 'B'));
        String nested = new Nest(150).toString();
        System.out.println(nested.length() + " " + nested.substring(0, 5) + " " + nested.substring(148, 153));
        try {
            forever();
        } catch (StackOverflowError e) {
            System.out.println("stack overflow caught, deep enough: " + (depth > 1000));
        }
        depth = 0;
        try {
            forever();
        } catch (StackOverflowError e) {
            System.out.println("caught again");
        }
        System.out.println(sum(100));
    }
}
