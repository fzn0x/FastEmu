//! `javax.microedition.media.protocol`: `DataSource`, `SourceStream` and
//! `ContentDescriptor`, for the few games that feed a player from their
//! own source, and `Manager.createPlayer(DataSource)`.
//!
//! The player reads the source's first stream to its end when it is made
//! and plays the bytes as it would a stream's; the source stays connected
//! and started, as a game expects while its player lives.

use fastemu_jvm::classfile::access;
use fastemu_jvm::{Ctx, JResult, NativeClass, ObjRef, Value};

use super::api::{CONTROLLABLE, MEDIA_EXCEPTION};
use super::{manager, stream};

const DATA_SOURCE: &str = "javax/microedition/media/protocol/DataSource";
const SOURCE_STREAM: &str = "javax/microedition/media/protocol/SourceStream";
const CONTENT_DESCRIPTOR: &str = "javax/microedition/media/protocol/ContentDescriptor";

/// A class whose one private `String` field its constructor sets and its
/// getter returns: `DataSource`'s locator, `ContentDescriptor`'s type.
fn holder(name: &'static str, field: &'static str, getter: &str) -> NativeClass {
    NativeClass::new(name)
        .field_with(field, "Ljava/lang/String;", access::PRIVATE)
        .method("<init>", "(Ljava/lang/String;)V", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let id = ctx.field_id(name, field, "Ljava/lang/String;")?;
            ctx.set_field(this, id, a[1]);
            Ok(Value::Void)
        })
        .method(getter, "()Ljava/lang/String;", move |ctx, a| {
            let this = ctx.non_null(a[0])?;
            let id = ctx.field_id(name, field, "Ljava/lang/String;")?;
            Ok(ctx.get_field(this, id))
        })
}

fn data_source() -> NativeClass {
    [
        ("getContentType", "()Ljava/lang/String;"),
        ("connect", "()V"),
        ("disconnect", "()V"),
        ("start", "()V"),
        ("stop", "()V"),
        (
            "getStreams",
            "()[Ljavax/microedition/media/protocol/SourceStream;",
        ),
        ("getControls", "()[Ljavax/microedition/media/Control;"),
        (
            "getControl",
            "(Ljava/lang/String;)Ljavax/microedition/media/Control;",
        ),
    ]
    .into_iter()
    .fold(
        holder(DATA_SOURCE, "locator", "getLocator")
            .abstract_class()
            .implements(CONTROLLABLE),
        |class, (name, desc)| class.abstract_method(name, desc),
    )
}

fn source_stream() -> NativeClass {
    [
        (
            "getContentDescriptor",
            "()Ljavax/microedition/media/protocol/ContentDescriptor;",
        ),
        ("getContentLength", "()J"),
        ("read", "([BII)I"),
        ("getTransferSize", "()I"),
        ("seek", "(J)J"),
        ("tell", "()J"),
        ("getSeekType", "()I"),
    ]
    .into_iter()
    .fold(
        NativeClass::interface(SOURCE_STREAM)
            .implements(CONTROLLABLE)
            .constant("NOT_SEEKABLE", "I", Value::Int(0))
            .constant("SEEKABLE_TO_START", "I", Value::Int(1))
            .constant("RANDOM_ACCESSIBLE", "I", Value::Int(2)),
        |class, (name, desc)| class.abstract_method(name, desc),
    )
}

/// `Manager.createPlayer(DataSource)`: connects and starts the source,
/// reads its first stream and makes a player of the bytes. Exceptions the
/// source throws (an `IOException`) pass through.
pub(crate) fn create_player(ctx: &mut Ctx<'_>, a: &[Value]) -> JResult<Value> {
    let Some(source) = a[0].obj() else {
        return ctx.throw(
            "java/lang/IllegalArgumentException",
            "the data source is null",
        );
    };
    ctx.call_virtual(source, "connect", "()V", &[])?;
    ctx.call_virtual(source, "start", "()V", &[])?;
    let declared = ctx.call_virtual(source, "getContentType", "()Ljava/lang/String;", &[])?;
    let declared = ctx.opt_string(declared)?;
    let streams = ctx
        .call_virtual(
            source,
            "getStreams",
            "()[Ljavax/microedition/media/protocol/SourceStream;",
            &[],
        )?
        .obj();
    let first = match streams {
        Some(streams) if ctx.array_len(streams)? > 0 => ctx.array::<Option<ObjRef>>(streams)?[0],
        _ => None,
    };
    let Some(first) = first else {
        return ctx.throw(MEDIA_EXCEPTION, "the data source has no stream");
    };
    let first = ctx.root(first);
    let bytes = stream::read_source_stream(ctx, first)?;
    Ok(manager::player_for(ctx, bytes, declared)?.into())
}

/// The classes of this file.
pub(crate) fn classes() -> Vec<NativeClass> {
    vec![
        data_source(),
        source_stream(),
        holder(CONTENT_DESCRIPTOR, "contentType", "getContentType"),
    ]
}
