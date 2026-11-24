// try, catch and finally: ordering, nesting, rethrowing, exceptions the
// virtual machine raises, and failing static initializers.
public class Exceptions {
    static class AppException extends Exception {
        final int code;

        AppException(String message, int code) {
            super(message);
            this.code = code;
        }

        public String getMessage() {
            return super.getMessage() + " (code " + code + ")";
        }
    }

    static class Quiet extends RuntimeException {
        public String toString() {
            return "Quiet!";
        }
    }

    static class BadInit {
        static int value = compute();

        static int compute() {
            throw new IllegalStateException("cannot start");
        }
    }

    static StringBuffer log = new StringBuffer();

    static int finallyOverrides() {
        try {
            return 1;
        } finally {
            log.append("[finally runs]");
        }
    }

    @SuppressWarnings("finally")
    static int finallyReturns() {
        try {
            throw new RuntimeException("lost");
        } finally {
            return 2;
        }
    }

    static void nested(int depth) throws AppException {
        try {
            try {
                if (depth == 0) {
                    throw new AppException("deep", 42);
                }
                nested(depth - 1);
            } finally {
                log.append(depth);
            }
        } catch (IllegalArgumentException e) {
            log.append("wrong catch");
        }
    }

    static String kind(Throwable t) {
        return t.getClass().getName();
    }

    static void vmErrors() {
        Object[] tests = new Object[9];
        for (int i = 0; i < tests.length; i++) {
            try {
                switch (i) {
                    case 0: { Object o = null; o.toString(); break; }
                    case 1: { int[] a = new int[2]; a[2] = 0; break; }
                    case 2: { int z = 0; System.out.println(1 / z); break; }
                    case 3: { Object o = "s"; Integer n = (Integer) o; System.out.println(n); break; }
                    case 4: { Object[] a = new String[1]; a[0] = new Integer(1); break; }
                    case 5: { int[] a = new int[-1]; System.out.println(a.length); break; }
                    case 6: { long z = 0; System.out.println(5L % z); break; }
                    case 7: { throw null; }
                    default: { int[][] m = new int[2][-3]; System.out.println(m.length); }
                }
                System.out.println(i + " no exception");
            } catch (RuntimeException e) {
                System.out.println(i + " " + kind(e));
            }
        }
    }

    public static void main(String[] args) {
        System.out.println(finallyOverrides() + " " + log);
        System.out.println(finallyReturns());
        log.setLength(0);
        try {
            nested(3);
        } catch (AppException e) {
            System.out.println("caught " + e.getMessage() + " code=" + e.code + " log=" + log + " " + e);
        }
        try {
            throw new Quiet();
        } catch (RuntimeException e) {
            System.out.println("toString override: " + e + " message=" + e.getMessage());
        }
        try {
            try {
                throw new IllegalArgumentException("first");
            } catch (IllegalArgumentException e) {
                throw new IllegalStateException("second from " + e.getMessage());
            } finally {
                System.out.println("inner finally");
            }
        } catch (IllegalStateException e) {
            System.out.println("outer caught " + e.getMessage());
        }
        try {
            Integer.parseInt("12x");
        } catch (NumberFormatException e) {
            System.out.println("parse: " + e.getMessage());
        }
        try {
            Object o = new Object();
            o.wait();
        } catch (IllegalMonitorStateException e) {
            System.out.println("wait without lock: " + kind(e));
        } catch (InterruptedException e) {
            System.out.println("unexpected interrupt");
        }
        vmErrors();
        for (int round = 0; round < 2; round++) {
            try {
                System.out.println(BadInit.value);
            } catch (Throwable t) {
                System.out.println("init round " + round + ": " + kind(t));
                if (t instanceof ExceptionInInitializerError) {
                    Throwable cause = ((ExceptionInInitializerError) t).getCause();
                    System.out.println("  cause " + cause);
                }
            }
        }
        Throwable plain = new Throwable();
        System.out.println(plain + " " + plain.getMessage() + " " + new Error("e").getMessage() + " " + new Exception((String) null));
        RuntimeException wrapped = new RuntimeException(new ArithmeticException("inner"));
        System.out.println(wrapped.getMessage() + " | " + wrapped.getCause());
        int caught = 0;
        for (int i = 0; i < 1000; i++) {
            try {
                if (i % 3 == 0) {
                    throw new IndexOutOfBoundsException("i=" + i);
                }
            } catch (IndexOutOfBoundsException e) {
                caught++;
            } finally {
                caught += 2;
            }
        }
        System.out.println("loop caught " + caught);
        try {
            synchronized (log) {
                throw new UnsupportedOperationException("from synchronized");
            }
        } catch (UnsupportedOperationException e) {
            System.out.println(e.getMessage());
        }
        synchronized (log) {
            log.append("relocked");
        }
        System.out.println(log);
        System.out.println(new ArrayIndexOutOfBoundsException(-3).getMessage());
        System.out.println(new StringIndexOutOfBoundsException(9).getMessage());
        java.io.InterruptedIOException timeout = new java.io.InterruptedIOException("t");
        timeout.bytesTransferred = 12;
        System.out.println(timeout.getMessage() + " " + timeout.bytesTransferred);
        Error vm = new OutOfMemoryError("full");
        System.out.println((vm instanceof VirtualMachineError) + " " + vm.getMessage());
    }
}
