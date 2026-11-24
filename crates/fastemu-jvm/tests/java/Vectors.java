import java.util.EmptyStackException;
import java.util.Enumeration;
import java.util.NoSuchElementException;
import java.util.Stack;
import java.util.Vector;

// Vector, Stack and the Enumeration of a vector: growth, edge cases and
// their exceptions, and a game's own Vector subclass.
public class Vectors {
    // A game's own Vector that reads the protected fields.
    static class Peek extends Vector {
        Peek() {
            super(2, 3);
        }

        String raw() {
            return elementCount + "/" + elementData.length + "/" + capacityIncrement;
        }

        void retype() {
            elementData = new String[4];
            elementCount = 0;
        }
    }

    static void vectors() {
        Vector v = new Vector();
        System.out.println(v.size() + " " + v.capacity() + " " + v.isEmpty() + " " + v);
        for (int i = 0; i < 12; i++) {
            v.addElement(new Integer(i * 10));
        }
        System.out.println(v.size() + " " + v.capacity() + " " + v);
        v.insertElementAt("first", 0);
        v.insertElementAt(null, 5);
        v.insertElementAt("last", v.size());
        v.setElementAt("five", 6);
        v.removeElementAt(1);
        System.out.println(v + " " + v.capacity());
        System.out.println(v.indexOf(null) + " " + v.indexOf(new Integer(70)) + " " + v.indexOf(new Integer(70), 8)
            + " " + v.lastIndexOf("first") + " " + v.lastIndexOf(new Integer(30), 2) + " " + v.contains("five")
            + " " + v.contains(new Long(30)));
        System.out.println(v.firstElement() + " " + v.lastElement() + " " + v.elementAt(3));
        System.out.println(v.removeElement(new Integer(100)) + " " + v.removeElement("none") + " " + v.size());
        Object[] copy = new Object[v.size() + 2];
        v.copyInto(copy);
        System.out.println(copy[0] + " " + copy[v.size() - 1] + " " + copy[v.size()]);
        v.setSize(3);
        System.out.println(v + " " + v.capacity());
        v.setSize(6);
        System.out.println(v + " " + v.size());
        v.trimToSize();
        System.out.println(v.capacity());
        v.ensureCapacity(20);
        System.out.println(v.capacity());
        v.addElement(v);
        System.out.println(v);
        v.removeAllElements();
        System.out.println(v.size() + " " + v + " " + v.capacity());

        Vector inc = new Vector(1, 4);
        for (int i = 0; i < 10; i++) {
            inc.addElement("x");
            System.out.print(inc.capacity() + " ");
        }
        System.out.println();
        Vector zero = new Vector(0);
        zero.addElement("a");
        zero.addElement("b");
        System.out.println(zero.capacity());

        Peek peek = new Peek();
        peek.addElement("a");
        peek.addElement("b");
        peek.addElement("c");
        System.out.println(peek.raw());
        peek.retype();
        peek.addElement("text");
        try {
            peek.addElement(new Integer(1));
        } catch (ArrayStoreException e) {
            System.out.println("ArrayStoreException " + e.getMessage() + " " + peek.raw());
        }
    }

    static void vectorErrors() {
        Vector v = new Vector();
        v.addElement("a");
        v.addElement("b");
        String[] names = {"elementAt(5)", "elementAt(-1)", "setElementAt(2)", "removeElementAt(2)",
            "removeElementAt(-1)", "insertElementAt(3)", "lastIndexOf(.., 2)", "first of empty",
            "last of empty", "new Vector(-1)", "copyInto short", "copyInto String[]", "setSize(-1)"};
        for (int i = 0; i < names.length; i++) {
            try {
                switch (i) {
                    case 0: v.elementAt(5); break;
                    case 1: v.elementAt(-1); break;
                    case 2: v.setElementAt("x", 2); break;
                    case 3: v.removeElementAt(2); break;
                    case 4: v.removeElementAt(-1); break;
                    case 5: v.insertElementAt("x", 3); break;
                    case 6: v.lastIndexOf("a", 2); break;
                    case 7: new Vector().firstElement(); break;
                    case 8: new Vector().lastElement(); break;
                    case 9: new Vector(-1); break;
                    case 10: v.copyInto(new Object[1]); break;
                    case 11: {
                        Vector w = new Vector();
                        w.addElement(new Integer(1));
                        w.copyInto(new String[1]);
                        break;
                    }
                    default: v.setSize(-1); break;
                }
                System.out.println(names[i] + ": no exception");
            } catch (RuntimeException e) {
                // System.arraycopy's messages vary between JDK releases.
                String message = i == 10 || i == 11 ? "" : " " + e.getMessage();
                System.out.println(names[i] + ": " + e.getClass().getName() + message);
            }
        }
        System.out.println(v);
    }

    static void enumerations() {
        Vector v = new Vector();
        for (int i = 0; i < 6; i++) {
            v.addElement(new Integer(i));
        }
        Enumeration e = v.elements();
        StringBuffer seen = new StringBuffer();
        while (e.hasMoreElements()) {
            Object n = e.nextElement();
            seen.append(n).append(' ');
            if (n.equals(new Integer(1))) {
                v.removeElementAt(0);
            }
            if (n.equals(new Integer(3))) {
                v.addElement("added");
            }
        }
        System.out.println(seen);
        try {
            e.nextElement();
        } catch (NoSuchElementException x) {
            System.out.println("end: " + x.getMessage());
        }
    }

    static void stacks() {
        Stack s = new Stack();
        System.out.println(s.empty() + " " + s.push("a") + " " + s.push("b") + " " + s.push("a") + " " + s);
        System.out.println(s.search("a") + " " + s.search("b") + " " + s.search("z") + " " + s.peek());
        System.out.println(s.pop() + " " + s.pop() + " " + s.size() + " " + s.empty());
        s.pop();
        try {
            s.pop();
        } catch (EmptyStackException e) {
            System.out.println("pop: EmptyStackException " + e.getMessage());
        }
        try {
            s.peek();
        } catch (EmptyStackException e) {
            System.out.println("peek: EmptyStackException");
        }
    }

    public static void main(String[] args) {
        vectors();
        vectorErrors();
        enumerations();
        stacks();
    }
}
