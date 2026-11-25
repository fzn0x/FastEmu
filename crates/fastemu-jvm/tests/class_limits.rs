//! Class names, array dimensions and class hierarchies of any size end in
//! a Java error, never in a host stack overflow (which closes the whole
//! app) and never in a search that runs for hours. The class data of a
//! machine is limited too: past `VmLimits::max_class_bytes` a class does
//! not load and the game gets `OutOfMemoryError`.
//!
//! Every case runs on a thread of the virtual machine, which has its own
//! 16 MiB stack, and under a deadline: a regression fails the test where it
//! would otherwise hang it.

mod support;

use std::sync::mpsc;
use std::time::Duration;

use fastemu_jvm::{
    ArrayType, Ctx, JResult, MemorySource, NativeClass, Throw, Value, Vm, VmConfig, VmLimits,
};
use support::{ClassBuilder, PUBLIC, STATIC, idx};

/// How long a case may take. They take milliseconds; a loaded machine gets
/// plenty of room.
const DEADLINE: Duration = Duration::from_secs(20);

/// The classes of a case, by internal name.
type Classes = Vec<(String, Vec<u8>)>;

/// Runs `body` on a virtual machine thread over `classes` and `natives`
/// and returns what it returned. Fails when it takes over [`DEADLINE`]: the
/// work runs on a thread of its own, so a search that never ends cannot
/// hold the test up.
fn run<R: Send + 'static>(
    classes: Classes,
    natives: fn() -> Vec<NativeClass>,
    body: impl FnOnce(&mut Ctx<'_>) -> R + Send + 'static,
) -> R {
    run_with(VmConfig::default(), classes, natives, body)
}

/// [`run`] on a machine of `config`.
fn run_with<R: Send + 'static>(
    config: VmConfig,
    classes: Classes,
    natives: fn() -> Vec<NativeClass>,
    body: impl FnOnce(&mut Ctx<'_>) -> R + Send + 'static,
) -> R {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut source = MemorySource::new();
        for (name, bytes) in classes {
            source.add_class(name, bytes);
        }
        let vm = Vm::new(config, std::sync::Arc::new(source), natives());
        let (result_tx, result_rx) = mpsc::channel();
        vm.spawn("probe", move |ctx| {
            let _ = result_tx.send(body(ctx));
            Ok(())
        })
        .expect("a thread");
        let result = result_rx.recv_timeout(DEADLINE);
        // Stopping takes the VM lock, which a search that never ends holds.
        if result.is_ok() {
            vm.stop();
        }
        let _ = tx.send(result.ok());
    });
    rx.recv_timeout(DEADLINE + Duration::from_secs(5))
        .ok()
        .flatten()
        .expect("the case finished in time")
}

fn no_natives() -> Vec<NativeClass> {
    Vec::new()
}

/// What `result` came to, as a line: `ok`, or the exception's text.
fn outcome<T>(ctx: &mut Ctx<'_>, result: JResult<T>) -> String {
    match result {
        Ok(_) => "ok".to_owned(),
        Err(Throw::Exception(e)) => ctx.describe_exception(e),
        Err(Throw::Stop) => "stopped".to_owned(),
    }
}

/// Calls `Probe.forName(name)`.
fn for_name(ctx: &mut Ctx<'_>, name: &str) -> String {
    let name = ctx.new_string(name).expect("a string");
    let called = ctx.call_static(
        "Probe",
        "forName",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        &[name.into()],
    );
    outcome(ctx, called)
}

/// Calls the static method `Probe.<method>()` that takes nothing and
/// returns a reference.
fn call(ctx: &mut Ctx<'_>, method: &str) -> String {
    let called = ctx.call_static("Probe", method, "()Ljava/lang/Object;", &[]);
    outcome(ctx, called)
}

/// `Probe`, whose methods are:
///
/// ```java
/// static Class forName(String name) { return Class.forName(name); }
/// static Class deep(int n) {
///     StringBuffer sb = new StringBuffer();
///     for (int i = 0; i < n; i++) sb.append('[');
///     sb.append('I');
///     return Class.forName(sb.toString());
/// }
/// static Object constant() { return <the class constant `constant`>; }
/// static int ran() { return C0.ran; }
/// static int missing() { return M.nosuch; }
/// ```
fn probe(constant: &str) -> Vec<u8> {
    let mut c = ClassBuilder::new("Probe", "java/lang/Object", 49, 0);
    let for_name = c.method_ref(
        "java/lang/Class",
        "forName",
        "(Ljava/lang/String;)Ljava/lang/Class;",
    );
    let [fh, fl] = idx(for_name);
    c.method(
        PUBLIC | STATIC,
        "forName",
        "(Ljava/lang/String;)Ljava/lang/Class;",
        1,
        1,
        &[0x2a, 0xb8, fh, fl, 0xb0],
        &[],
    );

    let buffer = c.class("java/lang/StringBuffer");
    let [bh, bl] = idx(buffer);
    let init = c.method_ref("java/lang/StringBuffer", "<init>", "()V");
    let [ih, il] = idx(init);
    let append = c.method_ref(
        "java/lang/StringBuffer",
        "append",
        "(C)Ljava/lang/StringBuffer;",
    );
    let [ah, al] = idx(append);
    let to_string = c.method_ref("java/lang/StringBuffer", "toString", "()Ljava/lang/String;");
    let [th, tl] = idx(to_string);
    #[rustfmt::skip]
    let deep = [
        0xbb, bh, bl,       //  0: new StringBuffer
        0x59,               //  3: dup
        0xb7, ih, il,       //  4: invokespecial <init>
        0x4c,               //  7: astore_1
        0x03,               //  8: iconst_0
        0x3d,               //  9: istore_2
        0x1c,               // 10: iload_2
        0x1a,               // 11: iload_0
        0xa2, 0x00, 0x10,   // 12: if_icmpge 28
        0x2b,               // 15: aload_1
        0x10, b'[',         // 16: bipush '['
        0xb6, ah, al,       // 18: invokevirtual append(C)
        0x57,               // 21: pop
        0x84, 0x02, 0x01,   // 22: iinc 2 1
        0xa7, 0xff, 0xf1,   // 25: goto 10
        0x2b,               // 28: aload_1
        0x10, b'I',         // 29: bipush 'I'
        0xb6, ah, al,       // 31: invokevirtual append(C)
        0x57,               // 34: pop
        0x2b,               // 35: aload_1
        0xb6, th, tl,       // 36: invokevirtual toString
        0xb8, fh, fl,       // 39: invokestatic Class.forName
        0xb0,               // 42: areturn
    ];
    c.method(
        PUBLIC | STATIC,
        "deep",
        "(I)Ljava/lang/Class;",
        2,
        3,
        &deep,
        &[],
    );

    let class = c.class(constant);
    let [ch, cl] = idx(class);
    c.method(
        PUBLIC | STATIC,
        "constant",
        "()Ljava/lang/Object;",
        1,
        0,
        &[0x13, ch, cl, 0xb0],
        &[],
    );

    let ran = c.field_ref("C0", "ran", "I");
    let [rh, rl] = idx(ran);
    c.method(
        PUBLIC | STATIC,
        "ran",
        "()I",
        1,
        0,
        &[0xb2, rh, rl, 0xac],
        &[],
    );

    let nosuch = c.field_ref("M", "nosuch", "I");
    let [nh, nl] = idx(nosuch);
    c.method(
        PUBLIC | STATIC,
        "missing",
        "()I",
        1,
        0,
        &[0xb2, nh, nl, 0xac],
        &[],
    );
    c.build()
}

/// A class `name` that extends `super_name` and has no members.
fn bare(name: &str, super_name: &str) -> Vec<u8> {
    ClassBuilder::new(name, super_name, 49, 0).build()
}

/// `Probe` and a chain of `len` classes, `C0` to `C<len - 1>`, each
/// extending the one before. `C0` extends `Object` and sets its static
/// field `ran` when it is initialized.
fn chain(len: usize) -> Classes {
    let mut classes = vec![("Probe".to_owned(), probe("Probe"))];
    let mut root = ClassBuilder::new("C0", "java/lang/Object", 49, 0);
    root.field(STATIC, "ran", "I");
    let ran = root.field_ref("C0", "ran", "I");
    let [h, l] = idx(ran);
    root.method(
        STATIC,
        "<clinit>",
        "()V",
        1,
        0,
        &[0x04, 0xb3, h, l, 0xb1],
        &[],
    );
    classes.push(("C0".to_owned(), root.build()));
    for i in 1..len {
        let name = format!("C{i}");
        let bytes = bare(&name, &format!("C{}", i - 1));
        classes.push((name, bytes));
    }
    classes
}

/// A class constant of `dims` dimensions of `int`.
fn array_name(dims: usize) -> String {
    format!("{}I", "[".repeat(dims))
}

#[test]
fn a_deep_array_name_from_a_game_is_a_java_error() {
    let classes = vec![("Probe".to_owned(), probe(&array_name(300)))];
    let lines = run(classes, no_natives, |ctx| {
        let mut lines = Vec::new();
        // The audit's repro: a StringBuffer loop builds the name.
        for n in [40_000, 300, 256, 255] {
            let called = ctx.call_static("Probe", "deep", "(I)Ljava/lang/Class;", &[Value::Int(n)]);
            lines.push(format!("deep {n}: {}", outcome(ctx, called)));
        }
        lines.push(format!("ldc 300: {}", call(ctx, "constant")));
        lines
    });
    assert_eq!(
        lines[0],
        format!(
            "deep 40000: java.lang.ClassNotFoundException: {}...",
            "[".repeat(100)
        )
    );
    assert_eq!(
        lines[1],
        "deep 300: java.lang.ClassFormatError: array type has too many dimensions"
    );
    assert_eq!(
        lines[2],
        "deep 256: java.lang.ClassFormatError: array type has too many dimensions"
    );
    assert_eq!(lines[3], "deep 255: ok", "255 dimensions are the limit");
    assert_eq!(
        lines[4],
        "ldc 300: java.lang.ClassFormatError: array type has too many dimensions"
    );
}

#[test]
fn a_class_name_over_the_limit_is_a_java_error_with_a_short_message() {
    let long = "a".repeat(300);
    let classes = vec![("Probe".to_owned(), probe(&long))];
    let lines = run(classes, no_natives, move |ctx| {
        let mut lines = vec![format!("ldc: {}", call(ctx, "constant"))];
        lines.push(format!("forName 300: {}", for_name(ctx, &long)));
        lines.push(format!("forName 255: {}", for_name(ctx, &"b".repeat(255))));
        // An array of such a class: the name inside is over the limit.
        let element = format!("[L{};", "c".repeat(300));
        lines.push(format!("forName array: {}", for_name(ctx, &element)));
        lines.push(format!(
            "forName huge: {}",
            for_name(ctx, &"d".repeat(100_000))
        ));
        lines
    });
    for line in &lines {
        assert!(
            line.len() < 200,
            "a message of the name's whole size: {line}"
        );
    }
    assert!(
        lines[0].starts_with("ldc: java.lang.NoClassDefFoundError: aaaa"),
        "{lines:?}"
    );
    assert!(
        lines[1].starts_with("forName 300: java.lang.ClassNotFoundException: aaaa"),
        "{lines:?}"
    );
    assert!(
        lines[2].starts_with("forName 255: java.lang.ClassNotFoundException: bbbb"),
        "{lines:?}"
    );
    assert!(
        lines[3].starts_with("forName array: java.lang.NoClassDefFoundError: ccc"),
        "{lines:?}"
    );
    assert!(
        lines[4].starts_with("forName huge: java.lang.ClassNotFoundException: dddd"),
        "{lines:?}"
    );
}

#[test]
fn a_class_name_of_wide_characters_over_the_limit_is_not_found() {
    // The limit is in bytes, and a character takes two of them here: 127
    // characters fit, 128 do not, and 200 are under the limit of units but
    // over that of bytes.
    let lines = run(
        vec![("Probe".to_owned(), probe("Probe"))],
        no_natives,
        |ctx| {
            [127, 128, 200]
                .map(|n| for_name(ctx, &"\u{e9}".repeat(n)))
                .to_vec()
        },
    );
    for line in &lines {
        assert!(
            line.starts_with("java.lang.ClassNotFoundException: \u{e9}\u{e9}\u{e9}"),
            "{lines:?}"
        );
    }
}

#[test]
fn a_new_array_of_arrays_of_the_most_dimensions_is_a_java_error() {
    let lines = run(Vec::new(), no_natives, |ctx| {
        let deepest = ctx.find_class(&array_name(255)).expect("255 dimensions");
        let mut lines = Vec::new();
        let made = ctx.new_object_array(deepest, 1);
        lines.push(outcome(ctx, made));
        let int = ctx.find_class("[I").expect("int[]");
        let made = ctx.new_object_array(int, 1);
        lines.push(outcome(ctx, made));
        lines
    });
    assert_eq!(
        lines,
        [
            "java.lang.ClassFormatError: array type has too many dimensions",
            "ok"
        ]
    );
}

#[test]
fn a_chain_loaded_from_its_leaf_down_is_refused_past_the_limit() {
    let lines = run(chain(300), no_natives, |ctx| {
        let mut lines = vec![for_name(ctx, "C299")];
        // Nothing of the failed load stayed behind.
        lines.push(for_name(ctx, "C100"));
        lines.push(for_name(ctx, "C254"));
        lines.push(for_name(ctx, "C255"));
        lines
    });
    assert!(
        lines[0].starts_with("java.lang.ClassFormatError: C")
            && lines[0].ends_with(": the class hierarchy is over 256 classes deep"),
        "{lines:?}"
    );
    assert_eq!(lines[1], "ok");
    assert_eq!(
        lines[2], "ok",
        "a hierarchy of 256 classes with Object loads"
    );
    assert!(
        lines[3].starts_with("java.lang.ClassFormatError"),
        "{lines:?}"
    );
}

#[test]
fn a_chain_loaded_from_its_root_up_is_refused_at_the_257th_class() {
    let (failed, ran, after) = run(chain(300), no_natives, |ctx| {
        let mut failed = Vec::new();
        for i in 0..300 {
            if let Err(Throw::Exception(e)) = ctx.find_class(&format!("C{i}")) {
                failed.push((i, ctx.describe_exception(e)));
            }
        }
        // Initializing the deepest class that loaded runs the initializers
        // of all 255 classes above it.
        let init = for_name(ctx, "C254");
        let ran = ctx
            .call_static("Probe", "ran", "()I", &[])
            .ok()
            .map(Value::int);
        (failed, (init, ran), for_name(ctx, "C255"))
    });
    assert_eq!(failed.first().map(|f| f.0), Some(255), "{failed:?}");
    assert_eq!(failed.len(), 45, "every class past the limit fails");
    assert!(
        failed[0]
            .1
            .starts_with("java.lang.ClassFormatError: C255: "),
        "{failed:?}"
    );
    assert_eq!(ran, ("ok".to_owned(), Some(1)));
    assert!(after.starts_with("java.lang.ClassFormatError"), "{after}");
}

#[test]
fn a_chain_of_200_loads_and_initializes() {
    let lines = run(chain(200), no_natives, |ctx| {
        let init = for_name(ctx, "C199");
        let ran = ctx
            .call_static("Probe", "ran", "()I", &[])
            .ok()
            .map(Value::int);
        (init, ran)
    });
    assert_eq!(lines, ("ok".to_owned(), Some(1)));
}

/// Layers of interfaces: `A<k>` and `B<k>` both extend `A<k-1>` and
/// `B<k-1>`, and the class `M` implements the last `A`. The first layer's
/// `B0` declares the static field `found`.
fn diamond() -> Vec<NativeClass> {
    const LAYERS: usize = 40;
    let mut natives = vec![
        NativeClass::interface("A0"),
        NativeClass::interface("B0").static_field("found", "I"),
    ];
    for k in 1..LAYERS {
        for side in ["A", "B"] {
            natives.push(
                NativeClass::interface(&format!("{side}{k}"))
                    .implements(&format!("A{}", k - 1))
                    .implements(&format!("B{}", k - 1)),
            );
        }
    }
    natives.push(NativeClass::new("M").implements(&format!("A{}", LAYERS - 1)));
    natives
}

#[test]
fn a_field_lookup_over_a_diamond_of_40_interface_layers_ends_at_once() {
    let classes = vec![("Probe".to_owned(), probe("Probe"))];
    let (missing, found) = run(classes, diamond, |ctx| {
        let missing = ctx.call_static("Probe", "missing", "()I", &[]);
        let missing = outcome(ctx, missing);
        let found = ctx.static_field_id("M", "found", "I").is_ok();
        (missing, found)
    });
    assert!(
        missing.starts_with("java.lang.NoSuchFieldError"),
        "{missing}"
    );
    assert!(found, "a field of the bottom interface resolves");
}

/// Interfaces `I0` to `I299`, each extending the one before.
fn interface_chain() -> Vec<NativeClass> {
    (0..300)
        .map(|i| match i {
            0 => NativeClass::interface("I0"),
            _ => NativeClass::interface(&format!("I{i}")).implements(&format!("I{}", i - 1)),
        })
        .collect()
}

#[test]
fn an_interface_chain_is_refused_past_the_limit_in_either_order() {
    // The leaf first: the loads nest and stop at 256.
    let leaf_first = run(Vec::new(), interface_chain, |ctx| {
        let leaf = ctx.find_class("I299");
        outcome(ctx, leaf)
    });
    assert!(
        leaf_first.starts_with("java.lang.ClassFormatError: I")
            && leaf_first.ends_with(": the class hierarchy is over 256 classes deep"),
        "{leaf_first}"
    );

    // The root first: no load nests, so each interface is checked by its
    // height. `I254` is the last with 256 classes on its path.
    let failed = run(Vec::new(), interface_chain, |ctx| {
        (0..300)
            .filter(|i| ctx.find_class(&format!("I{i}")).is_err())
            .collect::<Vec<_>>()
    });
    assert_eq!(failed, (255..300).collect::<Vec<_>>());
}

// The class memory budget.

/// A machine that holds `max_class_bytes` of class data.
fn limited(max_class_bytes: usize) -> VmConfig {
    VmConfig {
        limits: VmLimits {
            max_class_bytes,
            ..VmLimits::default()
        },
        ..VmConfig::default()
    }
}

/// `Loader`, whose method loads a class and catches the error a game gets
/// when it cannot:
///
/// ```java
/// static int tryLoad(String name) {
///     try { Class.forName(name); return 1; }
///     catch (OutOfMemoryError e) { return 0; }
/// }
/// ```
fn loader() -> Vec<u8> {
    let mut c = ClassBuilder::new("Loader", "java/lang/Object", 49, 0);
    let for_name = c.method_ref(
        "java/lang/Class",
        "forName",
        "(Ljava/lang/String;)Ljava/lang/Class;",
    );
    let [fh, fl] = idx(for_name);
    let oom = c.class("java/lang/OutOfMemoryError");
    #[rustfmt::skip]
    let code = [
        0x2a,               // 0: aload_0
        0xb8, fh, fl,       // 1: invokestatic Class.forName
        0x57,               // 4: pop
        0x04,               // 5: iconst_1
        0xac,               // 6: ireturn
        0x57,               // 7: pop (the error)
        0x03,               // 8: iconst_0
        0xac,               // 9: ireturn
    ];
    c.method(
        PUBLIC | STATIC,
        "tryLoad",
        "(Ljava/lang/String;)I",
        1,
        1,
        &code,
        &[(0, 7, 7, oom)],
    );
    c.build()
}

/// Whether the game's own code loaded the class `name`: 1 when it did, 0
/// when it caught `OutOfMemoryError`.
fn try_load(ctx: &mut Ctx<'_>, name: &str) -> i32 {
    let name = ctx.new_string(name).expect("a string");
    let called = ctx.call_static("Loader", "tryLoad", "(Ljava/lang/String;)I", &[name.into()]);
    match called {
        Ok(v) => v.int(),
        Err(Throw::Exception(e)) => panic!("{}", ctx.describe_exception(e)),
        Err(Throw::Stop) => panic!("stopped"),
    }
}

/// A class `name` that holds `entries` constants and one method. The
/// audit's `bigpool`: a pool of 65535 entries is 327 KB of file, 600 bytes
/// in a JAR, and 6.3 MiB in memory.
fn big_pool(name: &str, entries: usize) -> Vec<u8> {
    let mut c = ClassBuilder::new(name, "java/lang/Object", 49, 0);
    for i in 0..entries {
        c.integer(i as i32);
    }
    c.method(PUBLIC | STATIC, "run", "()V", 0, 0, &[0xb1], &[]);
    c.build()
}

/// `Loader` and the classes `<prefix>0` to `<prefix><count - 1>`, each a
/// pool of `entries` constants.
fn pools(prefix: &str, count: usize, entries: usize) -> Classes {
    let mut classes = vec![("Loader".to_owned(), loader())];
    for i in 0..count {
        let name = format!("{prefix}{i}");
        classes.push((name.clone(), big_pool(&name, entries)));
    }
    classes
}

#[test]
fn classes_with_big_constant_pools_stop_loading_at_the_limit() {
    // Each class takes about 420 KB by the model, so two fit in 1 MiB.
    let loaded = run_with(limited(1 << 20), pools("P", 8, 4000), no_natives, |ctx| {
        (0..8)
            .map(|i| try_load(ctx, &format!("P{i}")))
            .collect::<Vec<_>>()
    });
    assert_eq!(loaded, [1, 1, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn a_class_that_does_not_fit_leaves_room_for_a_smaller_one() {
    let mut classes = pools("Small", 1, 100);
    classes.push(("Big".to_owned(), big_pool("Big", 12000)));
    let loaded = run_with(limited(1 << 20), classes, no_natives, |ctx| {
        // The refused class is not half loaded: asking again is refused
        // again, and nothing of it is left to take the room.
        [
            try_load(ctx, "Big"),
            try_load(ctx, "Big"),
            try_load(ctx, "Small0"),
        ]
    });
    assert_eq!(loaded, [0, 0, 1]);
}

#[test]
fn a_game_sized_set_of_classes_loads_under_the_default_limit() {
    let loaded = run(pools("Game", 200, 300), no_natives, |ctx| {
        (0..200)
            .map(|i| try_load(ctx, &format!("Game{i}")))
            .sum::<i32>()
    });
    assert_eq!(loaded, 200);
}

/// `Loader`, `Base` with `methods` methods and `subclasses` classes that
/// extend it and declare nothing: the audit's `wide`. Each subclass is a
/// few dozen bytes of file and has the whole method table of `Base` of its
/// own.
fn wide(methods: usize, subclasses: usize) -> Classes {
    let mut classes = vec![("Loader".to_owned(), loader())];
    let mut base = ClassBuilder::new("Base", "java/lang/Object", 49, 0);
    for i in 0..methods {
        base.method(PUBLIC, &format!("m{i}"), "()V", 0, 1, &[0xb1], &[]);
    }
    classes.push(("Base".to_owned(), base.build()));
    for i in 0..subclasses {
        classes.push((format!("S{i}"), bare(&format!("S{i}"), "Base")));
    }
    classes
}

#[test]
fn subclasses_of_a_wide_class_stop_loading_at_the_limit() {
    let loaded = run_with(limited(4 << 20), wide(3000, 200), no_natives, |ctx| {
        (0..200)
            .map(|i| try_load(ctx, &format!("S{i}")))
            .collect::<Vec<_>>()
    });
    // `Base` takes about 1.9 MiB, and each subclass about 100 KB of method
    // table: about 20 of them fit.
    let fit = loaded.iter().take_while(|&&l| l == 1).count();
    assert!((10..=40).contains(&fit), "{fit} subclasses loaded");
    assert!(loaded[fit..].iter().all(|&l| l == 0), "{loaded:?}");
}

/// The interface `name`, which extends `parent` when there is one.
fn interface(name: &str, parent: Option<&str>) -> Vec<u8> {
    let mut c = ClassBuilder::new(name, "java/lang/Object", 49, 0);
    c.access(0x0601);
    let parent = parent.map(|p| c.class(p).to_be_bytes());
    let mut bytes = c.build();
    if let Some([hi, lo]) = parent {
        // With no fields, methods or attributes, the class file ends in
        // four counts of 2 bytes: the interfaces, then the three others.
        let at = bytes.len() - 8;
        bytes.splice(at..at + 2, [0, 1, hi, lo]);
    }
    bytes
}

#[test]
fn a_chain_of_interfaces_loaded_from_its_root_stops_at_the_limit() {
    // The list of interfaces of each one is longer than the one before's,
    // so a chain takes memory by the square of its length.
    let mut classes = vec![("Loader".to_owned(), loader())];
    for i in 0..250 {
        let parent = (i > 0).then(|| format!("I{}", i - 1));
        let name = format!("I{i}");
        classes.push((name.clone(), interface(&name, parent.as_deref())));
    }
    let loaded = run_with(limited(256 << 10), classes, no_natives, |ctx| {
        (0..250)
            .map(|i| try_load(ctx, &format!("I{i}")))
            .collect::<Vec<_>>()
    });
    let fit = loaded.iter().take_while(|&&l| l == 1).count();
    assert!((50..250).contains(&fit), "{fit} interfaces loaded");
    assert!(loaded[fit..].iter().all(|&l| l == 0), "{loaded:?}");
}

#[test]
fn arrays_of_every_class_stop_at_the_limit_and_primitive_arrays_do_not() {
    // A game can ask for an array of 255 dimensions of every class there
    // is, which makes 255 classes, each with a name and a method table.
    let mut classes = vec![("Loader".to_owned(), loader())];
    for i in 0..40 {
        classes.push((format!("E{i}"), bare(&format!("E{i}"), "java/lang/Object")));
    }
    let (made, primitives) = run_with(limited(1 << 20), classes, no_natives, |ctx| {
        let made = (0..40)
            .filter(|i| {
                let name = format!("{}LE{i};", "[".repeat(255));
                ctx.find_class(&name).is_ok()
            })
            .count();
        // Whatever is left of the limit, `new double[4]` needs its class.
        let primitives = [ArrayType::Double, ArrayType::Long, ArrayType::Short]
            .map(|t| ctx.new_array(t, 4).is_ok());
        (made, primitives)
    });
    assert!((1..10).contains(&made), "{made} of 40 arrays were made");
    assert_eq!(primitives, [true; 3]);
}
