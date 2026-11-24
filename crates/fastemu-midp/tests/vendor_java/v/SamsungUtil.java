package v;

import com.samsung.util.AudioClip;
import com.samsung.util.LCDLight;
import com.samsung.util.SM;
import com.samsung.util.SMS;
import com.samsung.util.Vibration;

/**
 * Samsung's com.samsung.util: audio clips and their argument checks,
 * the backlight and vibration, and text messages that are never sent.
 */
public class SamsungUtil {
    interface Call {
        void run() throws Exception;
    }

    static void expect(String what, Call call) {
        try {
            call.run();
            System.out.println(what + ": no exception");
        } catch (Exception e) {
            System.out.println(what + ": " + e.getClass().getName());
        }
    }

    public static void main(String[] args) throws Exception {
        System.out.println("supported: " + AudioClip.isSupported() + " " + LCDLight.isSupported() + " "
                + Vibration.isSupported() + " " + SMS.isSupported());

        byte[] midi = {'M', 'T', 'h', 'd', 0, 0, 0, 6, 0, 0, 0, 1, 0, 96, 'M', 'T', 'r', 'k', 0, 0, 0, 4, 0,
            (byte) 0xFF, 0x2F, 0};
        final AudioClip clip = new AudioClip(AudioClip.TYPE_MIDI, midi, 0, midi.length);
        clip.play(1, 5);
        clip.pause();
        clip.resume();
        clip.stop();
        System.out.println("clip played");
        expect("volume 6", new Call() {
            public void run() {
                clip.play(1, 6);
            }
        });
        expect("loop 256", new Call() {
            public void run() {
                clip.play(256, 1);
            }
        });
        final AudioClip mmf = new AudioClip(AudioClip.TYPE_MMF, new byte[] {'M', 'M', 'M', 'D', 0, 0}, 0, 6);
        mmf.play(-1, 80);
        System.out.println("mmf played");
        expect("bad range", new Call() {
            public void run() {
                new AudioClip(AudioClip.TYPE_MIDI, new byte[4], 2, 3);
            }
        });
        expect("null data", new Call() {
            public void run() {
                new AudioClip(AudioClip.TYPE_MIDI, null, 0, 0);
            }
        });
        expect("missing file", new Call() {
            public void run() throws Exception {
                new AudioClip(AudioClip.TYPE_MIDI, "/nothing.mid");
            }
        });

        LCDLight.on(1000);
        LCDLight.off();
        Vibration.start(300, 3);
        Vibration.stop();
        expect("strength 6", new Call() {
            public void run() {
                Vibration.start(100, 6);
            }
        });

        final SM sm = new SM("+12345", "555", "hello");
        System.out.println("message: " + sm.getDestAddress() + " " + sm.getCallbackAddress() + " " + sm.getData());
        expect("letters in the number", new Call() {
            public void run() {
                sm.setDestAddress("call me");
            }
        });
        expect("81 characters", new Call() {
            public void run() {
                StringBuffer text = new StringBuffer();
                for (int i = 0; i < 81; i++) {
                    text.append('x');
                }
                sm.setData(text.toString());
            }
        });
        expect("send", new Call() {
            public void run() {
                SMS.send(sm);
            }
        });
        System.out.println("done");
    }
}
