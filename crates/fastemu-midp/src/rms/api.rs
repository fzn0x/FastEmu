//! The interfaces and exception classes of `javax.microedition.rms`, as
//! `docs/compat/api/midp-rms.md` lists them.

use fastemu_jvm::{NativeClass, Value};

use super::{
    INVALID_RECORD_ID_EXCEPTION, RECORD_COMPARATOR, RECORD_ENUMERATION, RECORD_FILTER,
    RECORD_LISTENER, RECORD_STORE_EXCEPTION, RECORD_STORE_FULL_EXCEPTION,
    RECORD_STORE_NOT_FOUND_EXCEPTION, RECORD_STORE_NOT_OPEN_EXCEPTION,
};

/// The descriptor of the three `RecordListener` methods.
pub(crate) const LISTENER_DESC: &str = "(Ljavax/microedition/rms/RecordStore;I)V";

/// The interfaces and exceptions.
pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        NativeClass::interface(RECORD_COMPARATOR)
            .constant("EQUIVALENT", "I", Value::Int(0))
            .constant("FOLLOWS", "I", Value::Int(1))
            .constant("PRECEDES", "I", Value::Int(-1))
            .abstract_method("compare", "([B[B)I"),
        NativeClass::interface(RECORD_FILTER).abstract_method("matches", "([B)Z"),
        NativeClass::interface(RECORD_LISTENER)
            .abstract_method("recordAdded", LISTENER_DESC)
            .abstract_method("recordChanged", LISTENER_DESC)
            .abstract_method("recordDeleted", LISTENER_DESC),
        NativeClass::interface(RECORD_ENUMERATION)
            .abstract_method("numRecords", "()I")
            .abstract_method("nextRecord", "()[B")
            .abstract_method("nextRecordId", "()I")
            .abstract_method("previousRecord", "()[B")
            .abstract_method("previousRecordId", "()I")
            .abstract_method("hasNextElement", "()Z")
            .abstract_method("hasPreviousElement", "()Z")
            .abstract_method("reset", "()V")
            .abstract_method("rebuild", "()V")
            .abstract_method("keepUpdated", "(Z)V")
            .abstract_method("isKeptUpdated", "()Z")
            .abstract_method("destroy", "()V"),
        exception(RECORD_STORE_EXCEPTION, "java/lang/Exception"),
        exception(INVALID_RECORD_ID_EXCEPTION, RECORD_STORE_EXCEPTION),
        exception(RECORD_STORE_FULL_EXCEPTION, RECORD_STORE_EXCEPTION),
        exception(RECORD_STORE_NOT_FOUND_EXCEPTION, RECORD_STORE_EXCEPTION),
        exception(RECORD_STORE_NOT_OPEN_EXCEPTION, RECORD_STORE_EXCEPTION),
    ]
}

/// An exception class with the two constructors every RMS exception has,
/// each passing on to the superclass's.
fn exception(name: &str, superclass: &'static str) -> NativeClass {
    NativeClass::new(name)
        .extends(superclass)
        .method("<init>", "()V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            ctx.call_special(superclass, this, "<init>", "()V", &[])?;
            Ok(Value::Void)
        })
        .method("<init>", "(Ljava/lang/String;)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            ctx.call_special(superclass, this, "<init>", "(Ljava/lang/String;)V", &[a[1]])?;
            Ok(Value::Void)
        })
}
