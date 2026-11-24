import java.util.Calendar;
import java.util.Date;
import java.util.TimeZone;

// Calendar, Date and TimeZone in fixed zones: every CLDC field, lenient
// sets that roll over, which of two hour or date fields wins, custom
// GMT offsets, and the calendar arithmetic games do for clocks and daily
// bonuses. Weeks start on Sunday, as in the US locale the JDK test uses.
public class CalendarFields {
    static final TimeZone GMT = TimeZone.getTimeZone("GMT");

    static String fields(Calendar c) {
        return c.get(Calendar.YEAR) + "-" + (c.get(Calendar.MONTH) + 1) + "-" + c.get(Calendar.DAY_OF_MONTH)
            + " " + c.get(Calendar.HOUR_OF_DAY) + ":" + c.get(Calendar.MINUTE) + ":" + c.get(Calendar.SECOND)
            + "." + c.get(Calendar.MILLISECOND) + " dow " + c.get(Calendar.DAY_OF_WEEK) + " " + c.get(Calendar.HOUR)
            + (c.get(Calendar.AM_PM) == Calendar.AM ? "am" : "pm") + " date " + c.get(Calendar.DATE);
    }

    static Calendar at(long millis, TimeZone zone) {
        Calendar c = Calendar.getInstance(zone);
        c.setTime(new Date(millis));
        return c;
    }

    public static void main(String[] args) {
        long[] instants = {0L, -1L, 951782400000L, 1795000000123L, 1767225599999L, -2208988800000L, 4102444800000L};
        for (int i = 0; i < instants.length; i++) {
            System.out.println(instants[i] + " " + fields(at(instants[i], GMT)));
        }

        Calendar c = at(1795000000123L, GMT);
        c.set(Calendar.MONTH, 13);
        System.out.println("month 13: " + fields(c) + " " + c.getTime().getTime());
        c.set(Calendar.DAY_OF_MONTH, 0);
        System.out.println("day 0: " + fields(c));
        c.set(Calendar.DAY_OF_MONTH, 40);
        c.set(Calendar.HOUR_OF_DAY, 30);
        c.set(Calendar.MINUTE, -5);
        System.out.println("day 40 hour 30 minute -5: " + fields(c));
        c.set(Calendar.YEAR, 2024);
        c.set(Calendar.MONTH, Calendar.FEBRUARY);
        c.set(Calendar.DAY_OF_MONTH, 29);
        c.set(Calendar.HOUR, 3);
        c.set(Calendar.AM_PM, Calendar.PM);
        System.out.println("leap day 3pm: " + fields(c));
        c.set(Calendar.HOUR_OF_DAY, 1);
        System.out.println("hour of day set last: " + fields(c));
        c.set(Calendar.DAY_OF_WEEK, Calendar.SATURDAY);
        System.out.println("saturday of that week: " + fields(c));
        c.set(Calendar.DAY_OF_WEEK, Calendar.MONDAY);
        c.set(Calendar.DAY_OF_MONTH, 10);
        System.out.println("day of month set last: " + fields(c));
        c.set(Calendar.SECOND, 59);
        c.set(Calendar.MILLISECOND, 1001);
        System.out.println("ms 1001: " + fields(c) + " " + c.getTime().getTime());

        // A daily bonus: midnight today, and the days between two dates.
        Calendar now = at(1795000000123L, GMT);
        now.set(Calendar.HOUR_OF_DAY, 0);
        now.set(Calendar.MINUTE, 0);
        now.set(Calendar.SECOND, 0);
        now.set(Calendar.MILLISECOND, 0);
        long midnight = now.getTime().getTime();
        Calendar later = at(midnight, GMT);
        later.set(Calendar.DAY_OF_MONTH, later.get(Calendar.DAY_OF_MONTH) + 100);
        System.out.println(midnight + " " + (later.getTime().getTime() - midnight) / 86400000L + " " + fields(later));

        int dowSum = 0;
        Calendar walk = at(1735689600000L, GMT);
        for (int day = 0; day < 800; day++) {
            dowSum = dowSum * 7 + walk.get(Calendar.DAY_OF_WEEK);
            dowSum ^= walk.get(Calendar.DAY_OF_MONTH) << 3;
            walk.set(Calendar.DAY_OF_MONTH, walk.get(Calendar.DAY_OF_MONTH) + 1);
        }
        System.out.println("walk " + dowSum + " " + fields(walk));

        String[] ids = {"GMT", "UTC", "GMT+07:00", "GMT+8", "GMT-0530", "GMT+1:5", "GMT+24", "Nowhere/Unknown", "gmt"};
        for (int i = 0; i < ids.length; i++) {
            TimeZone z = TimeZone.getTimeZone(ids[i]);
            System.out.println(ids[i] + " -> " + z.getID() + " " + z.getRawOffset() + " " + z.useDaylightTime() + " "
                + z.getOffset(1, 2026, Calendar.NOVEMBER, 24, Calendar.TUESDAY, 0) + " " + fields(at(1795000000123L, z)));
        }
        try {
            GMT.getOffset(1, 2026, 12, 1, 1, 0);
        } catch (IllegalArgumentException e) {
            System.out.println("month 12: IllegalArgumentException");
        }
        try {
            GMT.getOffset(1, 2026, 1, 30, 1, 0);
        } catch (IllegalArgumentException e) {
            System.out.println("30 February: IllegalArgumentException");
        }

        Calendar moved = at(1795000000123L, GMT);
        moved.setTimeZone(TimeZone.getTimeZone("GMT-0530"));
        System.out.println("moved: " + fields(moved) + " " + moved.getTimeZone().getID() + " " + moved.getTime().getTime());
        moved.set(Calendar.HOUR_OF_DAY, 12);
        moved.setTimeZone(GMT);
        System.out.println("set then moved: " + fields(moved) + " " + moved.getTime().getTime());

        Calendar a = at(1000L, GMT);
        Calendar b = at(2000L, GMT);
        Calendar a2 = at(1000L, TimeZone.getTimeZone("GMT"));
        System.out.println(a.before(b) + " " + a.after(b) + " " + b.after(a) + " " + a.before("x") + " "
            + a.equals(a2) + " " + a.equals(b) + " " + a.equals(at(1000L, TimeZone.getTimeZone("GMT+01:00"))));

        Date d = new Date(123456789L);
        Date same = new Date(123456789L);
        System.out.println(d.getTime() + " " + d.equals(same) + " " + (d.hashCode() == same.hashCode()) + " "
            + d.hashCode() + " " + new Date(-123456789012L).hashCode() + " " + d.equals("x"));
        d.setTime(5L);
        System.out.println(d.getTime() + " " + d.equals(same));
        long before = System.currentTimeMillis();
        long made = new Date().getTime();
        long cal = Calendar.getInstance().getTime().getTime();
        System.out.println((made >= before) + " " + (cal >= made));

        Calendar local = Calendar.getInstance();
        local.setTime(new Date(1795000000123L));
        Calendar gmt = at(1795000000123L, GMT);
        int shift = (local.get(Calendar.HOUR_OF_DAY) - gmt.get(Calendar.HOUR_OF_DAY) + 24) % 24;
        int raw = TimeZone.getDefault().getRawOffset() / 3600000;
        System.out.println("default zone agrees: " + (shift == (raw + 24) % 24));
    }
}
