// tableswitch, lookupswitch and switch on strings.
public class Switches {
    static String dense(int i) {
        switch (i) {
            case -2: return "minus two";
            case -1: return "minus one";
            case 0: return "zero";
            case 1: return "one";
            case 2:
            case 3: return "two or three";
            case 5: return "five";
            default: return "other " + i;
        }
    }

    static int sparse(int i) {
        switch (i) {
            case Integer.MIN_VALUE: return 1;
            case -1000000: return 2;
            case 7: return 3;
            case 1000: return 4;
            case 123456789: return 5;
            case Integer.MAX_VALUE: return 6;
            default: return 0;
        }
    }

    static String words(String s) {
        switch (s) {
            case "apple": return "fruit";
            case "carrot": return "vegetable";
            case "Aa":
            case "BB": return "same hash " + s;
            case "": return "empty";
            default: return "unknown";
        }
    }

    static int chars(char c) {
        int r = 0;
        switch (c) {
            case 'a':
                r += 1;
            case 'b':
                r += 10;
                break;
            case 'Ж':
                r = 100;
                break;
            default:
                r = -1;
        }
        return r;
    }

    public static void main(String[] args) {
        for (int i = -4; i <= 7; i++) {
            System.out.println(i + " " + dense(i));
        }
        int[] keys = {Integer.MIN_VALUE, -1000000, -999999, 7, 1000, 123456789, Integer.MAX_VALUE, 0};
        for (int i = 0; i < keys.length; i++) {
            System.out.println(keys[i] + " -> " + sparse(keys[i]));
        }
        String[] ws = {"apple", "carrot", "Aa", "BB", "", "pear", "APPLE"};
        for (int i = 0; i < ws.length; i++) {
            System.out.println("'" + ws[i] + "' " + words(ws[i]));
        }
        System.out.println(chars('a') + " " + chars('b') + " " + chars('Ж') + " " + chars('z'));
        int total = 0;
        for (int i = 0; i < 100; i++) {
            switch (i % 7) {
                case 0: total += i; break;
                case 3: total -= 1; break;
                case 6: continue;
                default: total ^= i;
            }
            total++;
        }
        System.out.println(total);
    }
}
