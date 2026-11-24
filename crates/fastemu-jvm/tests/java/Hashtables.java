import java.util.Enumeration;
import java.util.Hashtable;

// Hashtable: keys with their own hashCode and equals, boxed numbers and
// strings as keys, removal while enumerating, and the order keys come
// out in, which games that save or draw in key order rely on.
public class Hashtables {
    // A key whose hash collides on purpose and whose equals ignores case.
    static class Name {
        final String text;

        Name(String text) {
            this.text = text;
        }

        public int hashCode() {
            return text.length() * -7919;
        }

        public boolean equals(Object o) {
            return o instanceof Name && ((Name) o).text.equalsIgnoreCase(text);
        }

        public String toString() {
            return "Name(" + text + ")";
        }
    }

    // A key that counts the calls the table makes on it.
    static class Counted {
        static int hashes;
        static int equalities;
        final int id;

        Counted(int id) {
            this.id = id;
        }

        public int hashCode() {
            hashes++;
            return id % 5;
        }

        public boolean equals(Object o) {
            equalities++;
            return o instanceof Counted && ((Counted) o).id == id;
        }

        public String toString() {
            return "C" + id;
        }
    }

    static String all(Enumeration e) {
        StringBuffer sb = new StringBuffer();
        while (e.hasMoreElements()) {
            sb.append(e.nextElement()).append(' ');
        }
        return sb.toString();
    }

    static void tables() {
        Hashtable h = new Hashtable();
        System.out.println(h + " " + h.size() + " " + h.isEmpty());
        String[] words = {"apple", "banana", "cherry", "date", "elder", "fig", "grape", "kiwi", "lemon",
            "mango", "nut", "olive", "pear", "quince", "rasp", "sloe", "tomato", "ugli", "vanilla"};
        for (int i = 0; i < words.length; i++) {
            h.put(words[i], new Integer(i));
        }
        System.out.println(h);
        System.out.println(all(h.keys()));
        System.out.println(all(h.elements()));
        System.out.println(h.get("fig") + " " + h.get("none") + " " + h.containsKey("kiwi") + " " + h.contains(new Integer(3))
            + " " + h.contains("3"));
        System.out.println(h.put("fig", "FIG") + " " + h.remove("kiwi") + " " + h.remove("kiwi") + " " + h.size());

        Hashtable numbers = new Hashtable(3);
        numbers.put(new Integer(-5), "int -5");
        numbers.put(new Long(-5), "long -5");
        numbers.put(new Short((short) 7), "short 7");
        numbers.put(new Character('x'), "char x");
        numbers.put(new Boolean(true), "true");
        numbers.put(new Float(-0.0f), "float -0");
        numbers.put(new Float(0.0f), "float 0");
        numbers.put(new Double(Double.NaN), "nan");
        numbers.put(new Integer(Integer.MIN_VALUE), "min");
        System.out.println(numbers);
        System.out.println(numbers.get(new Integer(-5)) + " " + numbers.get(new Long(-5)) + " "
            + numbers.get(new Double(0.0 / 0.0)) + " " + numbers.get(new Float(0.0f)) + " " + numbers.get(new Byte((byte) 7)));

        Hashtable names = new Hashtable();
        names.put(new Name("Ann"), "first");
        names.put(new Name("Bob"), "second");
        names.put(new Name("ANN"), "replaced");
        names.put(new Name("Eve"), "third");
        System.out.println(names.size() + " " + names.get(new Name("aNN")) + " " + names.get(new Name("bob")) + " " + names);

        Counted.hashes = 0;
        Counted.equalities = 0;
        Hashtable counted = new Hashtable();
        for (int i = 0; i < 20; i++) {
            counted.put(new Counted(i), "v" + i);
        }
        counted.get(new Counted(13));
        counted.remove(new Counted(4));
        System.out.println(counted.size() + " " + Counted.hashes + " " + Counted.equalities + " " + counted);

        Hashtable big = new Hashtable();
        for (int i = 0; i < 1000; i++) {
            big.put("k" + (i * 7919 % 1000), new Integer(i));
        }
        long order = 0;
        Enumeration keys = big.keys();
        while (keys.hasMoreElements()) {
            order = order * 31 + keys.nextElement().hashCode();
        }
        System.out.println(big.size() + " " + order);

        Enumeration e = h.keys();
        StringBuffer seen = new StringBuffer();
        while (e.hasMoreElements()) {
            Object k = e.nextElement();
            seen.append(k).append(' ');
            h.remove(k);
            if (k.equals("date")) {
                h.remove("apple");
                h.put("zebra", "z");
            }
        }
        System.out.println(seen);
        System.out.println(h.size() + " " + h);

        Hashtable self = new Hashtable();
        self.put("me", self);
        System.out.println(self);
        self.clear();
        System.out.println(self.size() + " " + self + " " + self.keys().hasMoreElements());
    }

    static void tableErrors() {
        Hashtable h = new Hashtable();
        String[] names = {"put null key", "put null value", "get null", "contains null", "containsKey null",
            "remove null", "new Hashtable(-1)", "next past end"};
        for (int i = 0; i < names.length; i++) {
            try {
                switch (i) {
                    case 0: h.put(null, "v"); break;
                    case 1: h.put("k", null); break;
                    case 2: h.get(null); break;
                    case 3: h.contains(null); break;
                    case 4: h.containsKey(null); break;
                    case 5: h.remove(null); break;
                    case 6: new Hashtable(-1); break;
                    default: h.elements().nextElement(); break;
                }
                System.out.println(names[i] + ": no exception");
            } catch (RuntimeException e) {
                System.out.println(names[i] + ": " + e.getClass().getName());
            }
        }
        Hashtable tiny = new Hashtable(0);
        tiny.put("a", "1");
        tiny.put("b", "2");
        tiny.put("c", "3");
        System.out.println(tiny);
    }

    // put calls the protected rehash() when the table fills up, through
    // a game's override.
    static class Growing extends Hashtable {
        int rehashes;

        Growing() {
            super(3);
        }

        protected void rehash() {
            rehashes++;
            super.rehash();
        }
    }

    // One that never grows: put still adds every entry, in longer chains.
    static class Fixed extends Hashtable {
        int rehashes;

        Fixed() {
            super(2);
        }

        protected void rehash() {
            rehashes++;
        }
    }

    static void rehashes() {
        Growing g = new Growing();
        Fixed f = new Fixed();
        for (int i = 0; i < 20; i++) {
            g.put(new Integer(i), "g" + i);
            f.put(new Integer(i), "f" + i);
        }
        System.out.println("growing " + g.rehashes + " " + g.size() + " " + g);
        System.out.println("fixed " + f.rehashes + " " + f.size() + " " + f.get(new Integer(13)) + " " + f);
    }

    public static void main(String[] args) {
        tables();
        tableErrors();
        rehashes();
    }
}
