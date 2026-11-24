// The order classes initialize in (JVMS 5.5): on first active use,
// superclass first, once, and not for compile-time constants.
public class StaticInit {
    static StringBuffer order = new StringBuffer();

    static void note(String s) {
        order.append(s).append(' ');
    }

    static class A {
        static int a = init("A");
        static final int CONSTANT = 7;
        static final String TEXT = "text";
        static final Integer BOXED = new Integer(init("A.BOXED"));

        static int init(String s) {
            note(s);
            return 1;
        }

        static void touch() {
            note("A.touch");
        }
    }

    static class B extends A {
        static int b = init("B");

        static {
            note("B.static-block");
        }
    }

    static class C extends B {
        static int c = init("C");
        int instance = init("C.instance");
    }

    static class Cyclic1 {
        static int x = Cyclic2.y + 1;
    }

    static class Cyclic2 {
        static int y = Cyclic1.x + 10;
    }

    interface I {
        int FROM_I = StaticInit.mark("I.FROM_I");
    }

    static class Impl implements I {
        static int impl = A.init("Impl");
    }

    static int mark(String s) {
        note(s);
        return 5;
    }

    static class Holder {
        static int[] table = new int[4];

        static {
            for (int i = 0; i < table.length; i++) {
                table[i] = i * i;
            }
            note("Holder");
        }
    }

    public static void main(String[] args) {
        note("main");
        int k = A.CONSTANT;
        String t = A.TEXT;
        note("constants " + k + t);
        System.out.println(order);
        order.setLength(0);
        new C();
        System.out.println(order);
        order.setLength(0);
        new C();
        A.touch();
        System.out.println(order);
        order.setLength(0);
        System.out.println(B.b + " " + C.c + " " + A.BOXED);
        System.out.println(Cyclic1.x + " " + Cyclic2.y);
        new Impl();
        System.out.println(order);
        order.setLength(0);
        System.out.println(I.FROM_I + " " + order);
        System.out.println(Holder.table[3] + " " + order);
    }
}
