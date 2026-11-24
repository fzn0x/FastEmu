//! The API other crates build on: classes defined in Rust that bytecode
//! extends, threads with Rust bodies, posted work, global roots, waiting,
//! diagnostics and the VM controls.

mod support;

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fastemu_core::LogLevel;
use fastemu_jvm::{Diagnostic, JResult, NativeClass, Payload, Throw, Value, Vm, VmConfig, Wake};
use support::{ClassBuilder, PUBLIC, STATIC, idx};

#[derive(Default)]
struct Clicks {
    count: i32,
}

impl Payload for Clicks {}

/// `demo/Widget`: an abstract native class whose `click` calls the
/// subclass's `value` through virtual dispatch.
fn widget() -> NativeClass {
    NativeClass::new("demo/Widget")
        .abstract_class()
        .payload::<Clicks>()
        .field("label", "Ljava/lang/String;")
        .method("<init>", "()V", |_, _| Ok(Value::Void))
        .abstract_method("value", "()I")
        .method("click", "()I", |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let clicks = ctx.payload_mut::<Clicks>(this)?;
            clicks.count += 1;
            let count = clicks.count;
            let value = ctx.call_virtual(this, "value", "()I", &[])?.int();
            Ok(Value::Int(value * 100 + count))
        })
}

/// `class Game extends demo/Widget { int value() { return 42; } }`
fn game() -> Vec<u8> {
    let mut c = ClassBuilder::new("Game", "demo/Widget", 49, 0);
    c.default_constructor("demo/Widget");
    c.method(PUBLIC, "value", "()I", 1, 1, &[0x10, 42, 0xac], &[]);
    c.build()
}

/// `class Spin { static int count; static void spin() { for (;;) count++; } }`
fn spin() -> Vec<u8> {
    let mut c = ClassBuilder::new("Spin", "java/lang/Object", 49, 0);
    c.field(STATIC, "count", "I");
    let count = c.field_ref("Spin", "count", "I");
    let [h, l] = idx(count);
    #[rustfmt::skip]
    let code = [
        0xb2, h, l, 0x04, 0x60, // getstatic count, iconst_1, iadd
        0xb3, h, l,             // putstatic count
        0xa7, 0xff, 0xf8,       // goto 0
    ];
    c.method(STATIC, "spin", "()V", 2, 0, &code, &[]);
    c.build()
}

/// A class that uses a missing class, a missing method and a missing
/// field, one per static method.
fn missing() -> Vec<u8> {
    let mut c = ClassBuilder::new("Missing", "java/lang/Object", 49, 0);
    let canvas = c.class("com/nokia/mid/ui/FullCanvas");
    let draw = c.method_ref("demo/Widget", "drawFoo", "(II)V");
    let field = c.field_ref("demo/Widget", "nope", "I");
    let [ch, cl] = idx(canvas);
    let [mh, ml] = idx(draw);
    let [fh, fl] = idx(field);
    c.method(
        STATIC,
        "newCanvas",
        "()V",
        2,
        0,
        &[0xbb, ch, cl, 0x57, 0xb1],
        &[],
    );
    c.method(
        STATIC,
        "draw",
        "()V",
        2,
        0,
        &[0x03, 0x03, 0xb8, mh, ml, 0xb1],
        &[],
    );
    c.method(STATIC, "field", "()I", 1, 0, &[0xb2, fh, fl, 0xac], &[]);
    c.build()
}

/// `class Npes { static void npes(int n) { for (i = 0; i < n; i++) try {
/// null.hashCode(); } catch (NullPointerException e) {} } }`
fn npes() -> Vec<u8> {
    let mut c = ClassBuilder::new("Npes", "java/lang/Object", 49, 0);
    let hash = c.method_ref("java/lang/Object", "hashCode", "()I");
    let npe = c.class("java/lang/NullPointerException");
    let [h, l] = idx(hash);
    #[rustfmt::skip]
    let code = [
        0x03, 0x3c,                 // iconst_0 istore_1
        0x1b, 0x1a,                 // 2: iload_1 iload_0
        0xa2, 0x00, 0x12,           // if_icmpge 22
        0x01, 0xb6, h, l, 0x57,     // 7: aconst_null invokevirtual hashCode pop
        0xa7, 0x00, 0x04,           // 12: goto 16
        0x4d,                       // 15: astore_2 (the handler)
        0x84, 0x01, 0x01,           // 16: iinc 1 1
        0xa7, 0xff, 0xef,           // goto 2
        0xb1,                       // 22: return
    ];
    c.method(STATIC, "npes", "(I)V", 2, 3, &code, &[(7, 12, 15, npe)]);
    c.build()
}

fn vm_with(config: VmConfig) -> Vm {
    let source = support::source(&[
        ("Game", game()),
        ("Spin", spin()),
        ("Missing", missing()),
        ("Npes", npes()),
    ]);
    Vm::new(config, Arc::new(source), vec![widget()])
}

fn vm() -> Vm {
    vm_with(VmConfig::default())
}

/// Polls `check` until it holds or two seconds pass.
fn eventually(mut check: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        if check() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

#[test]
fn natives_call_overrides_of_game_subclasses() {
    let vm = vm();
    let results = vm.attach("test", |ctx| -> JResult<(i32, i32, i32)> {
        let game = ctx.construct("Game", "()V", &[])?;
        let first = ctx.call_virtual(game, "click", "()I", &[])?.int();
        let second = ctx.call_virtual(game, "click", "()I", &[])?.int();
        let label = ctx.field_id("demo/Widget", "label", "Ljava/lang/String;")?;
        let text = ctx.new_string("start")?;
        ctx.set_field(game, label, text.into());
        let read = ctx.get_field(game, label).obj().expect("set above");
        assert_eq!(ctx.string(read)?, "start");
        Ok((first, second, ctx.payload::<Clicks>(game)?.count))
    });
    assert_eq!(results, Ok((4201, 4202, 2)));
}

#[test]
fn abstract_natives_refuse_instantiation() {
    let vm = vm();
    let message = vm.attach("test", |ctx| {
        match ctx.construct("demo/Widget", "()V", &[]) {
            Err(Throw::Exception(e)) => ctx.describe_exception(e),
            other => format!("{other:?}"),
        }
    });
    assert_eq!(message, "java.lang.InstantiationError: demo.Widget");
}

#[test]
fn diagnostics_name_what_is_missing() {
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let sink = Arc::clone(&seen);
    let (log, lines) = support::capture();
    let vm = vm_with(VmConfig {
        diagnostics: Some(Arc::new(move |d: &Diagnostic| {
            sink.lock().unwrap().push(d.to_string());
        })),
        log,
        ..VmConfig::default()
    });
    let errors = vm.attach("test", |ctx| {
        let calls = [
            ("newCanvas", "()V"),
            ("draw", "()V"),
            ("draw", "()V"),
            ("field", "()I"),
        ];
        let mut errors = Vec::new();
        for (name, desc) in calls {
            errors.push(match ctx.call_static("Missing", name, desc, &[]) {
                Err(Throw::Exception(e)) => ctx.describe_exception(e),
                other => format!("{other:?}"),
            });
        }
        errors
    });
    assert_eq!(
        errors,
        [
            "java.lang.NoClassDefFoundError: com.nokia.mid.ui.FullCanvas",
            "java.lang.NoSuchMethodError: demo.Widget.drawFoo(II)V",
            "java.lang.NoSuchMethodError: demo.Widget.drawFoo(II)V",
            "java.lang.NoSuchFieldError: nope",
        ]
    );
    assert_eq!(
        *seen.lock().unwrap(),
        [
            "missing class com/nokia/mid/ui/FullCanvas",
            "missing method demo/Widget.drawFoo(II)V",
            "missing field demo/Widget.nope:I",
        ],
        "each diagnostic is reported once"
    );
    assert!(
        support::lines_at(&lines, LogLevel::Warn)
            .contains(&"missing class com/nokia/mid/ui/FullCanvas".to_owned())
    );
}

#[test]
fn posted_tasks_run_on_an_event_thread() {
    let vm = vm();
    vm.attach("setup", |ctx| ctx.set_extension(Vec::<i32>::new()));
    vm.spawn("event", |ctx| {
        loop {
            ctx.wait_posted(None)?;
            ctx.run_posted()?;
        }
    })
    .unwrap();
    for i in 0..5 {
        vm.post(move |ctx| {
            let s = ctx.new_string(&format!("task {i}"))?;
            let n = ctx.call_virtual(s, "length", "()I", &[])?.int();
            ctx.extension_mut::<Vec<i32>>().unwrap().push(i * 10 + n);
            Ok(())
        });
    }
    let done = eventually(|| {
        vm.attach("poll", |ctx| {
            ctx.extension::<Vec<i32>>().unwrap().len() == 5
        })
    });
    assert!(done, "the event thread ran the tasks");
    let ran = vm.attach("read", |ctx| ctx.extension::<Vec<i32>>().unwrap().clone());
    assert_eq!(ran, [6, 16, 26, 36, 46]);
    vm.stop();
    assert!(vm.wait_finished(Some(Duration::from_secs(2))));
}

#[test]
fn exceptions_the_interpreter_raises_are_not_kept_alive() {
    let vm = vm();
    let used = vm.attach("main", |ctx| {
        ctx.call_static("Npes", "npes", "(I)V", &[Value::Int(200_000)])
            .expect("every exception is caught");
        ctx.gc();
        ctx.heap_usage().1
    });
    assert!(
        used < 512 * 1024,
        "{used} bytes still in use after the loop"
    );
}

#[test]
fn global_roots_keep_objects_alive_until_dropped() {
    let vm = vm();
    let global = vm.attach("make", |ctx| {
        let kept = ctx.new_string("kept").unwrap();
        for _ in 0..1000 {
            ctx.new_string("garbage").unwrap();
        }
        ctx.new_global(kept)
    });
    let before = vm.heap_stats().live_objects;
    let text = vm.attach("read", |ctx| {
        ctx.gc();
        let s = global.get(ctx);
        ctx.string(s).unwrap()
    });
    assert_eq!(text, "kept");
    let after_gc = vm.heap_stats().live_objects;
    assert!(
        after_gc + 900 < before,
        "garbage was freed: {before} then {after_gc}"
    );
    drop(global);
    vm.attach("collect", |ctx| {
        ctx.gc();
    });
    assert!(
        vm.heap_stats().live_objects < after_gc,
        "the dropped root's object was freed"
    );
}

#[test]
fn rust_threads_wait_and_wake_on_interrupt() {
    let vm = vm();
    let (thread, start) = vm.attach("main", |ctx| {
        ctx.set_extension(Vec::<Wake>::new());
        let thread = ctx
            .spawn_thread("waiter", |ctx| {
                // Wakeups are not precise, so wait until the deadline says so.
                let deadline = Instant::now() + Duration::from_millis(20);
                while ctx.wait_until(Some(deadline))? != Wake::TimedOut {}
                ctx.extension_mut::<Vec<Wake>>()
                    .unwrap()
                    .push(Wake::TimedOut);
                loop {
                    let wake = ctx.wait_until(None)?;
                    if wake == Wake::Interrupted {
                        ctx.extension_mut::<Vec<Wake>>().unwrap().push(wake);
                        return Ok(());
                    }
                }
            })
            .unwrap();
        (ctx.new_global(thread), Instant::now())
    });
    assert!(eventually(|| vm.attach("poll", |ctx| ctx
        .extension::<Vec<Wake>>()
        .unwrap()
        .len()
        == 1)));
    assert!(start.elapsed() >= Duration::from_millis(20));
    vm.attach("interrupt", |ctx| {
        let t = thread.get(ctx);
        ctx.call_virtual(t, "interrupt", "()V", &[]).unwrap();
    });
    assert!(vm.wait_finished(Some(Duration::from_secs(2))));
    let wakes = vm.attach("read", |ctx| ctx.extension::<Vec<Wake>>().unwrap().clone());
    assert_eq!(wakes, [Wake::TimedOut, Wake::Interrupted]);
}

fn spin_count(vm: &Vm) -> i32 {
    vm.attach("read", |ctx| {
        let field = ctx.static_field_id("Spin", "count", "I").unwrap();
        ctx.get_static(field).unwrap().int()
    })
}

#[test]
fn pause_resume_and_stop_control_running_java() {
    let vm = vm();
    vm.spawn("spinner", |ctx| {
        ctx.call_static("Spin", "spin", "()V", &[])?;
        Ok(())
    })
    .unwrap();
    assert!(eventually(|| spin_count(&vm) > 1000));
    vm.pause();
    std::thread::sleep(Duration::from_millis(20));
    let paused_at = spin_count(&vm);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(spin_count(&vm), paused_at, "a paused thread does not run");
    vm.resume();
    assert!(eventually(|| spin_count(&vm) > paused_at));
    vm.stop();
    assert!(
        vm.wait_finished(Some(Duration::from_secs(2))),
        "a stopped loop ends"
    );
    assert_eq!(vm.thread_count(), 0);
}

#[test]
fn locks_release_while_a_native_does_slow_work() {
    let vm = vm();
    vm.spawn("spinner", |ctx| {
        ctx.call_static("Spin", "spin", "()V", &[])?;
        Ok(())
    })
    .unwrap();
    assert!(eventually(|| spin_count(&vm) > 0));
    let progressed = vm.attach("io", |ctx| {
        let field = ctx.static_field_id("Spin", "count", "I").unwrap();
        let before = ctx.get_static(field).unwrap().int();
        ctx.without_lock(|| std::thread::sleep(Duration::from_millis(30)));
        ctx.get_static(field).unwrap().int() > before
    });
    assert!(progressed, "the spinner ran while the lock was released");
    vm.stop();
    assert!(vm.wait_finished(Some(Duration::from_secs(2))));
}

#[test]
fn uncaught_exceptions_are_reported_with_the_thread_name() {
    let (log, lines) = support::capture();
    let vm = vm_with(VmConfig {
        log,
        ..VmConfig::default()
    });
    vm.spawn("worker", |ctx| {
        Err(ctx.new_exception("java/lang/IllegalStateException", Some("bad")))
    })
    .unwrap();
    assert!(vm.wait_finished(Some(Duration::from_secs(2))));
    let errors = support::lines_at(&lines, LogLevel::Err);
    assert_eq!(
        errors.first().map(String::as_str),
        Some("Exception in thread \"worker\" java.lang.IllegalStateException: bad")
    );
}

#[test]
fn system_exit_stops_the_vm_or_throws() {
    let vm = vm();
    let result = vm.attach("main", |ctx| {
        ctx.call_static("java/lang/System", "exit", "(I)V", &[Value::Int(3)])
    });
    assert_eq!(result, Err(Throw::Stop));
    assert_eq!(vm.exit_code(), Some(3));
    assert!(vm.is_stopping());

    let strict = vm_with(VmConfig {
        allow_exit: false,
        ..VmConfig::default()
    });
    let message = strict.attach("main", |ctx| {
        match ctx.call_static("java/lang/System", "exit", "(I)V", &[Value::Int(0)]) {
            Err(Throw::Exception(e)) => ctx.describe_exception(e),
            other => format!("{other:?}"),
        }
    });
    assert!(
        message.starts_with("java.lang.SecurityException"),
        "{message}"
    );
    assert_eq!(strict.exit_code(), None);
}

#[test]
fn properties_resources_and_class_objects() {
    let mut source = support::source(&[("Game", game())]);
    source.add_resource("/data/level1.txt", b"abc".to_vec());
    let vm = Vm::new(
        VmConfig {
            properties: vec![("microedition.platform".into(), "TestPhone".into())],
            ..VmConfig::default()
        },
        Arc::new(source),
        vec![widget()],
    );
    let (platform, bytes, name) = vm
        .attach("main", |ctx| -> JResult<(String, Vec<i32>, String)> {
            let key = ctx.new_string("microedition.platform")?;
            let value = ctx.call_static(
                "java/lang/System",
                "getProperty",
                "(Ljava/lang/String;)Ljava/lang/String;",
                &[key.into()],
            )?;
            let platform = ctx.string(value.obj().unwrap())?;
            let game = ctx.find_class("Game")?;
            let mirror = ctx.class_object(game);
            let path = ctx.new_string("/data/level1.txt")?;
            let stream = ctx.call_virtual(
                mirror,
                "getResourceAsStream",
                "(Ljava/lang/String;)Ljava/io/InputStream;",
                &[path.into()],
            )?;
            let stream = stream.obj().expect("the resource exists");
            let mut bytes = Vec::new();
            loop {
                let b = ctx.call_virtual(stream, "read", "()I", &[])?.int();
                bytes.push(b);
                if b < 0 {
                    break;
                }
            }
            let name = ctx.call_virtual(mirror, "getName", "()Ljava/lang/String;", &[])?;
            Ok((platform, bytes, ctx.string(name.obj().unwrap())?))
        })
        .unwrap();
    assert_eq!(platform, "TestPhone");
    assert_eq!(bytes, [97, 98, 99, -1]);
    assert_eq!(name, "Game");
}
