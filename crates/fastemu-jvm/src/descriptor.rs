//! Field and method descriptors (JVMS 4.3).

use crate::value::Value;

/// The kind of value a field, parameter or return type holds, as the
/// operand stack sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// `boolean`, `byte`, `char`, `short` and `int`.
    Int,
    Long,
    Float,
    Double,
    /// Objects and arrays.
    Ref,
    /// Only as a return type.
    Void,
}

impl Kind {
    /// The kind of the field descriptor starting with `tag`.
    pub fn from_tag(tag: u8) -> Option<Kind> {
        Some(match tag {
            b'Z' | b'B' | b'C' | b'S' | b'I' => Kind::Int,
            b'J' => Kind::Long,
            b'F' => Kind::Float,
            b'D' => Kind::Double,
            b'L' | b'[' => Kind::Ref,
            b'V' => Kind::Void,
            _ => return None,
        })
    }

    /// Operand stack slots a value of this kind takes.
    pub fn slots(self) -> usize {
        match self {
            Kind::Long | Kind::Double => 2,
            Kind::Void => 0,
            _ => 1,
        }
    }

    /// The value a field of this kind starts with.
    pub fn default_value(self) -> Value {
        match self {
            Kind::Int => Value::Int(0),
            Kind::Long => Value::Long(0),
            Kind::Float => Value::Float(0.0),
            Kind::Double => Value::Double(0.0),
            Kind::Ref => Value::NULL,
            Kind::Void => Value::Void,
        }
    }
}

/// Returns the length of the field descriptor at the start of `s`, or
/// `None` when `s` does not start with one.
pub fn field_len(s: &[u8]) -> Option<usize> {
    let mut i = 0;
    while s.get(i) == Some(&b'[') {
        i += 1;
    }
    if i > 255 {
        return None;
    }
    match s.get(i)? {
        b'Z' | b'B' | b'C' | b'S' | b'I' | b'J' | b'F' | b'D' => Some(i + 1),
        b'L' => {
            let end = s[i..].iter().position(|&b| b == b';')?;
            (end > 1).then_some(i + end + 1)
        }
        _ => None,
    }
}

/// Whether `desc` is exactly one field descriptor.
pub fn is_field_descriptor(desc: &str) -> bool {
    field_len(desc.as_bytes()) == Some(desc.len())
}

#[cfg(test)]
thread_local! {
    static SHAPES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How many descriptors [`method_shape`] has walked on this thread since
/// the last call, which starts the count again.
#[cfg(test)]
pub(crate) fn take_shapes() -> usize {
    SHAPES.with(|count| count.replace(0))
}

/// What the loader keeps of a method descriptor: the slots its parameters
/// take (`this` not counted) and the kind of its result, or `None` when
/// `desc` is not a method descriptor.
///
/// It walks the text once and keeps nothing, where
/// [`MethodDescriptor::parse`] makes a `String` for each parameter. A
/// descriptor can name 65532 parameters, and a class file can give all of
/// its 65535 methods the same one, so a caller that sees the same text
/// again keeps the answer and asks once.
pub fn method_shape(desc: &str) -> Option<(usize, Kind)> {
    #[cfg(test)]
    SHAPES.with(|count| count.set(count.get() + 1));
    let bytes = desc.as_bytes();
    if bytes.first() != Some(&b'(') {
        return None;
    }
    let mut at = 1;
    let mut slots = 0;
    while *bytes.get(at)? != b')' {
        slots += kind_of(&desc[at..]).slots();
        at += field_len(&bytes[at..])?;
    }
    let ret = &desc[at + 1..];
    if ret != "V" && !is_field_descriptor(ret) {
        return None;
    }
    Some((slots, kind_of(ret)))
}

/// A parsed method descriptor such as `(I[Ljava/lang/String;)V`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodDescriptor {
    /// Each parameter's descriptor.
    pub params: Vec<String>,
    /// The return descriptor, `V` for `void`.
    pub ret: String,
}

impl MethodDescriptor {
    /// Parses `desc`, or `None` when it is not a method descriptor.
    pub fn parse(desc: &str) -> Option<Self> {
        let bytes = desc.as_bytes();
        if bytes.first() != Some(&b'(') {
            return None;
        }
        let mut i = 1;
        let mut params = Vec::new();
        while *bytes.get(i)? != b')' {
            let len = field_len(&bytes[i..])?;
            params.push(desc[i..i + len].to_owned());
            i += len;
        }
        i += 1;
        let ret = &desc[i..];
        if ret != "V" && !is_field_descriptor(ret) {
            return None;
        }
        Some(Self {
            params,
            ret: ret.to_owned(),
        })
    }

    /// The kind of each parameter.
    pub fn param_kinds(&self) -> Vec<Kind> {
        self.params.iter().map(|p| kind_of(p)).collect()
    }

    pub fn ret_kind(&self) -> Kind {
        kind_of(&self.ret)
    }

    /// Slots the parameters take, not counting `this`.
    pub fn param_slots(&self) -> usize {
        self.params.iter().map(|p| kind_of(p).slots()).sum()
    }
}

/// The kind of a descriptor already known to be valid.
pub fn kind_of(desc: &str) -> Kind {
    desc.as_bytes()
        .first()
        .and_then(|&b| Kind::from_tag(b))
        .unwrap_or(Kind::Ref)
}

/// Turns `java.lang.String` into `java/lang/String`.
pub fn internal_name(binary_name: &str) -> String {
    binary_name.replace('.', "/")
}

/// Turns `java/lang/String` into `java.lang.String`.
pub fn binary_name(internal: &str) -> String {
    internal.replace('/', ".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_descriptors_parse() {
        let d = MethodDescriptor::parse("(IJ[Ljava/lang/String;D)V").unwrap();
        assert_eq!(d.params, ["I", "J", "[Ljava/lang/String;", "D"]);
        assert_eq!(d.ret, "V");
        assert_eq!(d.param_slots(), 6);
        assert_eq!(d.ret_kind(), Kind::Void);
        assert_eq!(
            MethodDescriptor::parse("()[[I").unwrap().ret_kind(),
            Kind::Ref
        );
    }

    #[test]
    fn the_shape_of_a_descriptor_is_what_its_parse_says() {
        for good in [
            "()V",
            "(I)I",
            "(IJ[Ljava/lang/String;D)V",
            "([J[[DLjava/lang/Object;F)[[I",
            "(BCSZ)Z",
        ] {
            let parsed = MethodDescriptor::parse(good).unwrap();
            assert_eq!(
                method_shape(good),
                Some((parsed.param_slots(), parsed.ret_kind())),
                "{good}"
            );
        }
        assert_eq!(method_shape("(JD)V"), Some((4, Kind::Void)));
        assert_eq!(method_shape("([J)J"), Some((1, Kind::Long)));
        let wide = format!("({})V", "J".repeat(65000));
        assert_eq!(method_shape(&wide), Some((130_000, Kind::Void)));
    }

    #[test]
    fn a_bad_descriptor_has_no_shape() {
        for bad in [
            "", "I", "(", "(L;)V", "(I", "(I)", "(I)X", "(Q)V", "()VV", "(V)V", "(Lx)V", "(I)L;",
            "()", "(I)II",
        ] {
            assert_eq!(method_shape(bad), None, "{bad}");
            assert!(MethodDescriptor::parse(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn bad_descriptors_fail() {
        for bad in ["", "I", "(", "(L;)V", "(I", "(I)", "(I)X", "(Q)V", "()VV"] {
            assert!(MethodDescriptor::parse(bad).is_none(), "{bad}");
        }
        assert!(is_field_descriptor("[[J"));
        assert!(!is_field_descriptor("[[JI"));
        assert!(!is_field_descriptor("V"));
    }

    #[test]
    fn names_convert() {
        assert_eq!(internal_name("a.b.C"), "a/b/C");
        assert_eq!(binary_name("a/b/C"), "a.b.C");
    }
}
