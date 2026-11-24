//! `java.util.Vector`, `Stack` and the `Enumeration` of a vector.
//!
//! As in Java, the elements live in the protected `elementData` array
//! and `elementCount`, which a subclass may read and change, so every
//! method reads them afresh. The methods Java declares `synchronized`
//! hold the vector's monitor here too, so a game that locks a vector
//! while it walks it keeps other threads out.

use super::keys;
use crate::class::ClassId;
use crate::classfile::access::{PROTECTED, PUBLIC, SYNCHRONIZED};
use crate::ctx::{Ctx, FieldId, JResult, Throw};
use crate::heap::{Payload, Tracer};
use crate::lang::{string_value, this, throw};
use crate::native::NativeClass;
use crate::value::{ObjRef, Value};

const VECTOR: &str = "java/util/Vector";
const STACK: &str = "java/util/Stack";
const ELEMENTS: &str = "java/util/Vector$Elements";
const DATA: &str = "[Ljava/lang/Object;";

/// The protected fields of a `Vector`.
struct Fields {
    data: FieldId,
    count: FieldId,
    increment: FieldId,
}

fn fields(ctx: &mut Ctx<'_>) -> JResult<Fields> {
    Ok(Fields {
        data: ctx.field_id(VECTOR, "elementData", DATA)?,
        count: ctx.field_id(VECTOR, "elementCount", "I")?,
        increment: ctx.field_id(VECTOR, "capacityIncrement", "I")?,
    })
}

/// A vector's array and count as its fields hold them now.
struct View {
    data: ObjRef,
    count: i32,
    capacity: usize,
}

fn view(ctx: &mut Ctx<'_>, f: &Fields, vector: ObjRef) -> JResult<View> {
    let data = ctx.get_field(vector, f.data);
    let data = ctx.non_null(data)?;
    let count = ctx.get_field(vector, f.count).int();
    let capacity = ctx.array_len(data)?;
    Ok(View {
        data,
        count,
        capacity,
    })
}

/// The error Java's array access throws for `index` in an array of
/// `len`.
fn bad_index(ctx: &mut Ctx<'_>, index: i64, len: usize) -> Throw {
    let message = format!("Index {index} out of bounds for length {len}");
    ctx.new_exception("java/lang/ArrayIndexOutOfBoundsException", Some(&message))
}

/// `elementData[index]`, rooted, with Java's array bounds check.
fn get(ctx: &mut Ctx<'_>, v: &View, index: i32) -> JResult<Option<ObjRef>> {
    match usize::try_from(index).ok().filter(|&i| i < v.capacity) {
        Some(i) => {
            let e = ctx.array::<Option<ObjRef>>(v.data)?[i];
            Ok(e.map(|e| ctx.root(e)))
        }
        None => Err(bad_index(ctx, i64::from(index), v.capacity)),
    }
}

/// `elementData[index] = e`, with Java's array checks: the bounds, and
/// the element type of an array a subclass put there in place of an
/// `Object[]`.
fn set(ctx: &mut Ctx<'_>, v: &View, index: i32, e: Option<ObjRef>) -> JResult<()> {
    let Some(i) = usize::try_from(index).ok().filter(|&i| i < v.capacity) else {
        return Err(bad_index(ctx, i64::from(index), v.capacity));
    };
    if let Some(e) = e
        && let Some(elem) = ctx.array_element_class(ctx.class_of(v.data))
        && !ctx.is_instance(e, elem)
    {
        let name = ctx.class_name(ctx.class_of(e)).replace('/', ".");
        return throw(ctx, "java/lang/ArrayStoreException", &name);
    }
    ctx.array_mut::<Option<ObjRef>>(v.data)?[i] = e;
    Ok(())
}

fn object_class(ctx: &mut Ctx<'_>) -> JResult<ClassId> {
    ctx.find_class("java/lang/Object")
}

/// Replaces the array with one of `capacity` holding the first `keep`
/// elements.
fn reallocate(
    ctx: &mut Ctx<'_>,
    f: &Fields,
    vector: ObjRef,
    v: &View,
    capacity: usize,
    keep: usize,
) -> JResult<View> {
    let object = object_class(ctx)?;
    let data = ctx.new_object_array(object, capacity as i32)?;
    let keep = keep.min(v.capacity).min(capacity);
    let old = ctx.array::<Option<ObjRef>>(v.data)?[..keep].to_vec();
    ctx.array_mut::<Option<ObjRef>>(data)?[..keep].copy_from_slice(&old);
    ctx.set_field(vector, f.data, data.into());
    Ok(View {
        data,
        count: v.count,
        capacity,
    })
}

/// Grows the array to hold at least `min` elements, as Java's does: by
/// `capacityIncrement`, or doubling when that is not positive.
fn grow(ctx: &mut Ctx<'_>, f: &Fields, vector: ObjRef, v: &View, min: usize) -> JResult<View> {
    let increment = ctx.get_field(vector, f.increment).int();
    let step = if increment > 0 {
        increment as usize
    } else {
        v.capacity
    };
    let capacity = v.capacity.saturating_add(step).max(min);
    if capacity > i32::MAX as usize {
        return Err(ctx.out_of_memory());
    }
    reallocate(ctx, f, vector, v, capacity, v.count.max(0) as usize)
}

fn set_count(ctx: &mut Ctx<'_>, f: &Fields, vector: ObjRef, count: i32) {
    ctx.set_field(vector, f.count, Value::Int(count));
}

/// `addElement`.
fn add(ctx: &mut Ctx<'_>, vector: ObjRef, e: Option<ObjRef>) -> JResult<()> {
    let f = fields(ctx)?;
    let mut v = view(ctx, &f, vector)?;
    if v.count >= 0 && v.count as usize == v.capacity {
        v = grow(ctx, &f, vector, &v, v.capacity + 1)?;
    }
    set(ctx, &v, v.count, e)?;
    set_count(ctx, &f, vector, v.count + 1);
    Ok(())
}

/// `elementAt`, with Java's message for an index past the count.
fn element_at(ctx: &mut Ctx<'_>, vector: ObjRef, index: i32) -> JResult<Option<ObjRef>> {
    let f = fields(ctx)?;
    let v = view(ctx, &f, vector)?;
    if index >= v.count {
        let message = format!("{index} >= {}", v.count);
        return throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message);
    }
    get(ctx, &v, index)
}

/// `removeElementAt`.
fn remove_at(ctx: &mut Ctx<'_>, vector: ObjRef, index: i32) -> JResult<()> {
    let f = fields(ctx)?;
    let v = view(ctx, &f, vector)?;
    if index >= v.count {
        let message = format!("{index} >= {}", v.count);
        return throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message);
    }
    if index < 0 {
        let message = format!("Array index out of range: {index}");
        return throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message);
    }
    let last = v.count - 1;
    if last as usize >= v.capacity {
        return Err(bad_index(ctx, i64::from(last), v.capacity));
    }
    let (i, end) = (index as usize, last as usize);
    ctx.array_mut::<Option<ObjRef>>(v.data)?
        .copy_within(i + 1..=end, i);
    set(ctx, &v, last, None)?;
    set_count(ctx, &f, vector, last);
    Ok(())
}

/// `indexOf(o, from)`: the first index at or after `from` whose element
/// `o` equals (or that is null, for a null `o`), or -1.
fn index_of(ctx: &mut Ctx<'_>, vector: ObjRef, o: Option<ObjRef>, from: i32) -> JResult<i32> {
    let f = fields(ctx)?;
    let mut i = from;
    loop {
        // A game's equals may change the vector, so look again each time.
        let v = view(ctx, &f, vector)?;
        if i >= v.count {
            return Ok(-1);
        }
        let e = get(ctx, &v, i)?;
        let found = match o {
            None => e.is_none(),
            Some(o) => keys::equals(ctx, o, e)?,
        };
        if found {
            return Ok(i);
        }
        i += 1;
    }
}

/// `lastIndexOf(o, from)`.
fn last_index_of(ctx: &mut Ctx<'_>, vector: ObjRef, o: Option<ObjRef>, from: i32) -> JResult<i32> {
    let f = fields(ctx)?;
    let count = view(ctx, &f, vector)?.count;
    if from >= count {
        let message = format!("{from} >= {count}");
        return throw(ctx, "java/lang/IndexOutOfBoundsException", &message);
    }
    let mut i = from;
    while i >= 0 {
        let v = view(ctx, &f, vector)?;
        let e = get(ctx, &v, i)?;
        let found = match o {
            None => e.is_none(),
            Some(o) => keys::equals(ctx, o, e)?,
        };
        if found {
            return Ok(i);
        }
        i -= 1;
    }
    Ok(-1)
}

fn init(ctx: &mut Ctx<'_>, vector: ObjRef, capacity: i32, increment: i32) -> JResult<Value> {
    if capacity < 0 {
        let message = format!("Illegal Capacity: {capacity}");
        return throw(ctx, "java/lang/IllegalArgumentException", &message);
    }
    let object = object_class(ctx)?;
    let data = ctx.new_object_array(object, capacity)?;
    let f = fields(ctx)?;
    ctx.set_field(vector, f.data, data.into());
    ctx.set_field(vector, f.increment, Value::Int(increment));
    Ok(Value::Void)
}

/// The text of `e` in a collection's `toString`.
pub(super) fn element_text(ctx: &mut Ctx<'_>, e: Option<ObjRef>) -> JResult<String> {
    let Some(e) = e else {
        return Ok("null".to_owned());
    };
    let s = ctx.call_virtual(e, "toString", "()Ljava/lang/String;", &[])?;
    Ok(ctx.opt_string(s)?.unwrap_or_else(|| "null".to_owned()))
}

/// What a vector's enumeration keeps: the vector and the next index.
#[derive(Default)]
struct Elements {
    vector: Option<ObjRef>,
    next: i32,
}

impl Payload for Elements {
    fn trace(&self, tracer: &mut Tracer<'_>) {
        tracer.mark_opt(self.vector);
    }
}

fn elements_class() -> NativeClass {
    NativeClass::new(ELEMENTS)
        .final_class()
        .implements("java/util/Enumeration")
        .payload::<Elements>()
        .method("hasMoreElements", "()Z", |ctx, a| {
            let state = ctx.payload::<Elements>(this(a))?;
            let (vector, next) = (state.vector.expect("made by elements()"), state.next);
            let f = fields(ctx)?;
            Ok(Value::from(next < ctx.get_field(vector, f.count).int()))
        })
        .method("nextElement", "()Ljava/lang/Object;", |ctx, a| {
            let state = ctx.payload::<Elements>(this(a))?;
            let (vector, next) = (state.vector.expect("made by elements()"), state.next);
            // Java reads the element with the vector locked.
            ctx.monitor_enter(vector)?;
            let f = fields(ctx)?;
            let result = view(ctx, &f, vector).and_then(|v| {
                if next < v.count {
                    get(ctx, &v, next).map(Some)
                } else {
                    Ok(None)
                }
            });
            ctx.monitor_exit(vector)?;
            match result? {
                Some(e) => {
                    ctx.payload_mut::<Elements>(this(a))?.next = next + 1;
                    Ok(e.map_or(Value::NULL, Value::from))
                }
                None => throw(
                    ctx,
                    "java/util/NoSuchElementException",
                    "Vector Enumeration",
                ),
            }
        })
}

/// A public method that holds the vector's monitor, as Java's
/// `synchronized` methods do.
fn sync(
    class: NativeClass,
    name: &str,
    desc: &str,
    f: impl Fn(&mut Ctx<'_>, &[Value]) -> JResult<Value> + Send + Sync + 'static,
) -> NativeClass {
    class.method_with(name, desc, PUBLIC | SYNCHRONIZED, f)
}

fn value(e: Option<ObjRef>) -> Value {
    e.map_or(Value::NULL, Value::from)
}

fn vector_class() -> NativeClass {
    let class = NativeClass::new(VECTOR)
        .field_with("elementData", DATA, PROTECTED)
        .field_with("elementCount", "I", PROTECTED)
        .field_with("capacityIncrement", "I", PROTECTED)
        .method("<init>", "()V", |ctx, a| init(ctx, this(a), 10, 0))
        .method("<init>", "(I)V", |ctx, a| init(ctx, this(a), a[1].int(), 0))
        .method("<init>", "(II)V", |ctx, a| {
            init(ctx, this(a), a[1].int(), a[2].int())
        })
        .method("elements", "()Ljava/util/Enumeration;", |ctx, a| {
            let class = ctx.find_class(ELEMENTS)?;
            let e = ctx.new_object(class)?;
            ctx.payload_mut::<Elements>(e)?.vector = Some(this(a));
            Ok(e.into())
        })
        .method("contains", "(Ljava/lang/Object;)Z", |ctx, a| {
            ctx.monitor_enter(this(a))?;
            let found = index_of(ctx, this(a), a[1].obj(), 0);
            ctx.monitor_exit(this(a))?;
            Ok(Value::from(found? >= 0))
        })
        .method("indexOf", "(Ljava/lang/Object;)I", |ctx, a| {
            ctx.monitor_enter(this(a))?;
            let found = index_of(ctx, this(a), a[1].obj(), 0);
            ctx.monitor_exit(this(a))?;
            Ok(Value::Int(found?))
        });
    let class = sync(class, "copyInto", "([Ljava/lang/Object;)V", |ctx, a| {
        let f = fields(ctx)?;
        let v = view(ctx, &f, this(a))?;
        let args = [
            v.data.into(),
            Value::Int(0),
            a[1],
            Value::Int(0),
            Value::Int(v.count),
        ];
        let desc = "(Ljava/lang/Object;ILjava/lang/Object;II)V";
        ctx.call_static("java/lang/System", "arraycopy", desc, &args)?;
        Ok(Value::Void)
    });
    let class = sync(class, "trimToSize", "()V", |ctx, a| {
        let f = fields(ctx)?;
        let v = view(ctx, &f, this(a))?;
        if v.count >= 0 && (v.count as usize) < v.capacity {
            reallocate(ctx, &f, this(a), &v, v.count as usize, v.count as usize)?;
        }
        Ok(Value::Void)
    });
    let class = sync(class, "ensureCapacity", "(I)V", |ctx, a| {
        let min = a[1].int();
        let f = fields(ctx)?;
        let v = view(ctx, &f, this(a))?;
        if min > 0 && min as usize > v.capacity {
            grow(ctx, &f, this(a), &v, min as usize)?;
        }
        Ok(Value::Void)
    });
    let class = sync(class, "setSize", "(I)V", |ctx, a| {
        let size = a[1].int();
        let f = fields(ctx)?;
        let mut v = view(ctx, &f, this(a))?;
        if size > 0 && size as usize > v.capacity {
            v = grow(ctx, &f, this(a), &v, size as usize)?;
        }
        for i in size..v.count {
            set(ctx, &v, i, None)?;
        }
        set_count(ctx, &f, this(a), size);
        Ok(Value::Void)
    });
    let class = sync(class, "capacity", "()I", |ctx, a| {
        let f = fields(ctx)?;
        Ok(Value::Int(view(ctx, &f, this(a))?.capacity as i32))
    });
    let class = sync(class, "size", "()I", |ctx, a| {
        let f = fields(ctx)?;
        Ok(ctx.get_field(this(a), f.count))
    });
    let class = sync(class, "isEmpty", "()Z", |ctx, a| {
        let f = fields(ctx)?;
        Ok(Value::from(ctx.get_field(this(a), f.count).int() == 0))
    });
    let class = sync(class, "indexOf", "(Ljava/lang/Object;I)I", |ctx, a| {
        Ok(Value::Int(index_of(ctx, this(a), a[1].obj(), a[2].int())?))
    });
    let class = sync(class, "lastIndexOf", "(Ljava/lang/Object;)I", |ctx, a| {
        let f = fields(ctx)?;
        let count = ctx.get_field(this(a), f.count).int();
        Ok(Value::Int(last_index_of(
            ctx,
            this(a),
            a[1].obj(),
            count - 1,
        )?))
    });
    let class = sync(class, "lastIndexOf", "(Ljava/lang/Object;I)I", |ctx, a| {
        Ok(Value::Int(last_index_of(
            ctx,
            this(a),
            a[1].obj(),
            a[2].int(),
        )?))
    });
    let class = sync(class, "elementAt", "(I)Ljava/lang/Object;", |ctx, a| {
        Ok(value(element_at(ctx, this(a), a[1].int())?))
    });
    let class = sync(class, "firstElement", "()Ljava/lang/Object;", |ctx, a| {
        let f = fields(ctx)?;
        let v = view(ctx, &f, this(a))?;
        if v.count == 0 {
            return Err(ctx.new_exception("java/util/NoSuchElementException", None));
        }
        Ok(value(get(ctx, &v, 0)?))
    });
    let class = sync(class, "lastElement", "()Ljava/lang/Object;", |ctx, a| {
        let f = fields(ctx)?;
        let v = view(ctx, &f, this(a))?;
        if v.count == 0 {
            return Err(ctx.new_exception("java/util/NoSuchElementException", None));
        }
        Ok(value(get(ctx, &v, v.count - 1)?))
    });
    let class = sync(class, "setElementAt", "(Ljava/lang/Object;I)V", |ctx, a| {
        let f = fields(ctx)?;
        let v = view(ctx, &f, this(a))?;
        let index = a[2].int();
        if index >= v.count {
            let message = format!("{index} >= {}", v.count);
            return throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message);
        }
        set(ctx, &v, index, a[1].obj())?;
        Ok(Value::Void)
    });
    let class = sync(class, "removeElementAt", "(I)V", |ctx, a| {
        remove_at(ctx, this(a), a[1].int())?;
        Ok(Value::Void)
    });
    let class = sync(
        class,
        "insertElementAt",
        "(Ljava/lang/Object;I)V",
        |ctx, a| {
            let index = a[2].int();
            let f = fields(ctx)?;
            let mut v = view(ctx, &f, this(a))?;
            if index > v.count {
                let message = format!("{index} > {}", v.count);
                return throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message);
            }
            if v.count >= 0 && v.count as usize == v.capacity {
                v = grow(ctx, &f, this(a), &v, v.capacity + 1)?;
            }
            if index < 0 || v.count as usize >= v.capacity {
                let message = format!(
                    "arraycopy: source index {index} out of bounds for object array[{}]",
                    v.capacity
                );
                return throw(ctx, "java/lang/ArrayIndexOutOfBoundsException", &message);
            }
            let (i, end) = (index as usize, v.count as usize);
            ctx.array_mut::<Option<ObjRef>>(v.data)?
                .copy_within(i..end, i + 1);
            set(ctx, &v, index, a[1].obj())?;
            set_count(ctx, &f, this(a), v.count + 1);
            Ok(Value::Void)
        },
    );
    let class = sync(class, "addElement", "(Ljava/lang/Object;)V", |ctx, a| {
        add(ctx, this(a), a[1].obj())?;
        Ok(Value::Void)
    });
    let class = sync(class, "removeElement", "(Ljava/lang/Object;)Z", |ctx, a| {
        let i = index_of(ctx, this(a), a[1].obj(), 0)?;
        if i < 0 {
            return Ok(Value::from(false));
        }
        remove_at(ctx, this(a), i)?;
        Ok(Value::from(true))
    });
    let class = sync(class, "removeAllElements", "()V", |ctx, a| {
        let f = fields(ctx)?;
        let v = view(ctx, &f, this(a))?;
        for i in 0..v.count {
            set(ctx, &v, i, None)?;
        }
        set_count(ctx, &f, this(a), 0);
        Ok(Value::Void)
    });
    sync(class, "toString", "()Ljava/lang/String;", |ctx, a| {
        let vector = this(a);
        let f = fields(ctx)?;
        let mut text = String::from("[");
        let mut i = 0;
        loop {
            let v = view(ctx, &f, vector)?;
            if i >= v.count {
                break;
            }
            if i > 0 {
                text.push_str(", ");
            }
            let e = get(ctx, &v, i)?;
            if e == Some(vector) {
                text.push_str("(this Collection)");
            } else {
                text.push_str(&element_text(ctx, e)?);
            }
            i += 1;
        }
        text.push(']');
        string_value(ctx, &text)
    })
}

fn stack_class() -> NativeClass {
    let class = NativeClass::new(STACK)
        .extends(VECTOR)
        .method("<init>", "()V", |ctx, a| {
            ctx.call_special(VECTOR, this(a), "<init>", "()V", &[])?;
            Ok(Value::Void)
        })
        .method(
            "push",
            "(Ljava/lang/Object;)Ljava/lang/Object;",
            |ctx, a| {
                ctx.monitor_enter(this(a))?;
                let added = add(ctx, this(a), a[1].obj());
                ctx.monitor_exit(this(a))?;
                added?;
                Ok(a[1])
            },
        )
        .method("empty", "()Z", |ctx, a| {
            let f = fields(ctx)?;
            Ok(Value::from(ctx.get_field(this(a), f.count).int() == 0))
        });
    let class = sync(class, "pop", "()Ljava/lang/Object;", |ctx, a| {
        let f = fields(ctx)?;
        let len = ctx.get_field(this(a), f.count).int();
        if len == 0 {
            return Err(ctx.new_exception("java/util/EmptyStackException", None));
        }
        let top = element_at(ctx, this(a), len - 1)?;
        remove_at(ctx, this(a), len - 1)?;
        Ok(value(top))
    });
    let class = sync(class, "peek", "()Ljava/lang/Object;", |ctx, a| {
        let f = fields(ctx)?;
        let len = ctx.get_field(this(a), f.count).int();
        if len == 0 {
            return Err(ctx.new_exception("java/util/EmptyStackException", None));
        }
        Ok(value(element_at(ctx, this(a), len - 1)?))
    });
    sync(class, "search", "(Ljava/lang/Object;)I", |ctx, a| {
        let f = fields(ctx)?;
        let len = ctx.get_field(this(a), f.count).int();
        let i = last_index_of(ctx, this(a), a[1].obj(), len - 1)?;
        Ok(Value::Int(if i >= 0 { len - i } else { -1 }))
    })
}

pub(crate) fn classes() -> Vec<NativeClass> {
    vec![vector_class(), stack_class(), elements_class()]
}
