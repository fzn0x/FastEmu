// Virtual and interface dispatch, super calls, abstract classes, inner and
// anonymous classes, overloading, instanceof and casts.
public class Inheritance {
    interface Shape {
        int SIDES_UNKNOWN = -1;

        double area();

        String name();
    }

    interface Named {
        String name();
    }

    interface Polygon extends Shape {
        int sides();
    }

    static abstract class Base implements Polygon, Named {
        protected String label;

        Base(String label) {
            this.label = label;
        }

        public String name() {
            return "base:" + label;
        }

        public String toString() {
            return name() + " sides=" + sides() + " area=" + area();
        }

        String greet() {
            return "Base.greet";
        }

        static String kind() {
            return "Base.kind";
        }
    }

    static class Rect extends Base {
        final int w, h;

        Rect(int w, int h) {
            super("rect");
            this.w = w;
            this.h = h;
        }

        public double area() {
            return w * h;
        }

        public int sides() {
            return 4;
        }

        String greet() {
            return "Rect.greet/" + super.greet();
        }

        static String kind() {
            return "Rect.kind";
        }
    }

    static class Square extends Rect {
        Square(int s) {
            super(s, s);
            label = "square";
        }

        public String name() {
            return "square(" + super.name() + ")";
        }

        String greet() {
            return "Square.greet/" + super.greet();
        }
    }

    static class Tri extends Base {
        Tri() {
            super("tri");
        }

        public double area() {
            return 0.5;
        }

        public int sides() {
            return 3;
        }
    }

    static class Counter {
        private int count;

        class Incrementer {
            void bump() {
                count++;
            }
        }

        int run() {
            Incrementer inc = new Incrementer();
            for (int i = 0; i < 5; i++) {
                inc.bump();
            }
            return count;
        }
    }

    static class Overloads {
        static String f(int x) { return "int"; }
        static String f(long x) { return "long"; }
        static String f(double x) { return "double"; }
        static String f(Object x) { return "Object"; }
        static String f(String x) { return "String"; }
        static String f(char x) { return "char"; }
    }

    static class Parent {
        String field = "parent field";
        static String sfield = "parent static";

        String who() {
            return "Parent";
        }
    }

    static class Child extends Parent {
        String field = "child field";
        static String sfield = "child static";

        String who() {
            return "Child";
        }
    }

    static class Point {
        final int x, y;

        Point(int x, int y) {
            this.x = x;
            this.y = y;
        }

        public boolean equals(Object o) {
            if (!(o instanceof Point)) {
                return false;
            }
            Point p = (Point) o;
            return p.x == x && p.y == y;
        }

        public int hashCode() {
            return x * 31 + y;
        }
    }

    public static void main(String[] args) {
        Shape[] shapes = {new Rect(2, 3), new Square(4), new Tri()};
        for (int i = 0; i < shapes.length; i++) {
            Shape s = shapes[i];
            System.out.println(s + " | " + s.name() + " | " + ((Polygon) s).sides() + " | " + ((Named) s).name());
        }
        Base sq = new Square(2);
        System.out.println(sq.greet() + " " + Base.kind() + " " + Rect.kind() + " " + Shape.SIDES_UNKNOWN);
        System.out.println(new Counter().run());
        Runnable r = new Runnable() {
            int calls;

            public void run() {
                calls++;
                System.out.println("anonymous run " + calls);
            }
        };
        r.run();
        r.run();
        final String captured = "captured";
        Named n = new Named() {
            public String name() {
                return captured + "!";
            }
        };
        System.out.println(n.name());
        char c = 'x';
        byte b = 1;
        System.out.println(Overloads.f(1) + Overloads.f(1L) + Overloads.f(1.0f) + Overloads.f("s") + Overloads.f(new Object()) + Overloads.f(c) + Overloads.f(b));
        Parent p = new Child();
        System.out.println(p.field + " / " + ((Child) p).field + " / " + p.who() + " / " + Parent.sfield + " / " + Child.sfield);
        Object o = sq;
        System.out.println((o instanceof Shape) + " " + (o instanceof Polygon) + " " + (o instanceof Rect) + " " + (o instanceof Tri) + " " + (o instanceof Named) + " " + (null instanceof Object));
        Object arr = new Square[2];
        System.out.println((arr instanceof Object[]) + " " + (arr instanceof Shape[]) + " " + (arr instanceof Rect[]) + " " + (arr instanceof Tri[]) + " " + (arr instanceof int[]));
        Object ints = new int[2][3];
        System.out.println((ints instanceof Object[]) + " " + (ints instanceof int[][]) + " " + (ints instanceof long[][]));
        try {
            Tri t = (Tri) o;
            System.out.println("no cast error " + t);
        } catch (ClassCastException e) {
            System.out.println("ClassCastException");
        }
        Point p1 = new Point(1, 2);
        Point p2 = new Point(1, 2);
        System.out.println(p1.equals(p2) + " " + (p1 == p2) + " " + (p1.hashCode() == p2.hashCode()) + " " + p1.equals("x"));
        System.out.println(sq.getClass().getName() + " " + new int[0].getClass().getName() + " " + new String[0][0].getClass().getName() + " " + Shape.class.getName());
        System.out.println(Rect.class.isInstance(sq) + " " + Shape.class.isAssignableFrom(Square.class) + " " + Square.class.isAssignableFrom(Rect.class) + " " + Shape.class.isInterface() + " " + int[].class.isArray());
        Object str = "text";
        System.out.println(str.toString() + " " + str.equals("text") + " " + (str.hashCode() == "text".hashCode()));
    }
}
