//! Java source text from JVM names, descriptors and constants.

use std::fmt::Write as _;

use fastemu_jvm::descriptor::MethodDescriptor;

use super::runtime::Constant;

/// `a/b/C` as `a.b.C`.
pub fn binary(internal: &str) -> String {
    internal.replace('/', ".")
}

/// The Java type a field descriptor names: `I` is `int`, `[Ljava/lang/String;`
/// is `java.lang.String[]`. Class names are fully qualified.
pub fn type_name(desc: &str) -> Result<String, String> {
    let dims = desc.bytes().take_while(|&b| b == b'[').count();
    let base = &desc[dims..];
    let name = match base {
        "Z" => "boolean".to_owned(),
        "B" => "byte".to_owned(),
        "C" => "char".to_owned(),
        "S" => "short".to_owned(),
        "I" => "int".to_owned(),
        "J" => "long".to_owned(),
        "F" => "float".to_owned(),
        "D" => "double".to_owned(),
        "V" if dims == 0 => "void".to_owned(),
        _ if base.starts_with('L') && base.ends_with(';') && base.len() > 2 => {
            binary(&base[1..base.len() - 1])
        }
        _ => return Err(format!("bad descriptor {desc}")),
    };
    Ok(format!("{name}{}", "[]".repeat(dims)))
}

/// The parameter types and the return type of a method descriptor.
pub fn method_types(desc: &str) -> Result<(Vec<String>, String), String> {
    let parsed = MethodDescriptor::parse(desc).ok_or_else(|| format!("bad descriptor {desc}"))?;
    let params = parsed
        .params
        .iter()
        .map(|p| type_name(p))
        .collect::<Result<_, _>>()?;
    Ok((params, type_name(&parsed.ret)?))
}

/// A value of field type `desc` that is not a compile-time constant, so
/// javac does not copy it into the code that reads the field.
pub fn non_constant(desc: &str) -> String {
    const UNKNOWN: &str = "(java.lang.Object) null == null";
    match desc {
        "Z" => UNKNOWN.to_owned(),
        "B" | "C" | "S" | "I" | "J" | "F" | "D" => {
            format!(
                "({}) ({UNKNOWN} ? 0 : 0)",
                type_name(desc).unwrap_or_default()
            )
        }
        _ => "null".to_owned(),
    }
}

/// An argument of type `desc` for a call that must pick one overload:
/// `null` cast to the class, or a zero of the primitive.
pub fn placeholder(desc: &str) -> String {
    match desc {
        "Z" => "false".to_owned(),
        "B" | "C" | "S" | "I" | "J" | "F" | "D" => {
            format!("({}) 0", type_name(desc).unwrap_or_default())
        }
        _ => format!("({}) null", type_name(desc).unwrap_or_default()),
    }
}

fn float_literal(v: f64, is_float: bool) -> String {
    let suffix = if is_float { "f" } else { "d" };
    if v.is_nan() {
        format!("(0.0{suffix} / 0.0{suffix})")
    } else if v.is_infinite() {
        let sign = if v < 0.0 { "-" } else { "" };
        format!("({sign}1.0{suffix} / 0.0{suffix})")
    } else {
        String::new()
    }
}

/// The Java literal for `constant` in a field of type `desc`.
pub fn literal(desc: &str, constant: &Constant) -> Result<String, String> {
    let text = match (desc, constant) {
        ("Z", Constant::Int(v)) => (*v != 0).to_string(),
        ("B" | "S" | "C", Constant::Int(v)) => {
            format!("({}) {v}", type_name(desc)?)
        }
        ("I", Constant::Int(v)) => v.to_string(),
        ("J", Constant::Long(v)) => format!("{v}L"),
        ("J", Constant::Int(v)) => format!("{v}L"),
        ("F", Constant::Float(v)) => {
            let special = float_literal(f64::from(*v), true);
            if special.is_empty() {
                format!("{v:?}f")
            } else {
                special
            }
        }
        ("D", Constant::Double(v)) => {
            let special = float_literal(*v, false);
            if special.is_empty() {
                format!("{v:?}d")
            } else {
                special
            }
        }
        ("Ljava/lang/String;", Constant::String(units)) => string_literal(units),
        _ => return Err(format!("constant {constant:?} in a field of type {desc}")),
    };
    Ok(text)
}

/// A Java string literal holding exactly `units`.
pub fn string_literal(units: &[u16]) -> String {
    let mut out = String::from("\"");
    for &u in units {
        match u {
            0x22 => out.push_str("\\\""),
            0x5C => out.push_str("\\\\"),
            0x0A => out.push_str("\\n"),
            0x0D => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            // Octal escapes: a \u escape of a line break would end the
            // literal, since javac reads \u escapes before anything else.
            0..0x20 | 0x7F => {
                let _ = write!(out, "\\{u:03o}");
            }
            0x20..0x7F => out.push(char::from(u as u8)),
            _ => {
                let _ = write!(out, "\\u{u:04x}");
            }
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptors_become_types() {
        assert_eq!(type_name("I").unwrap(), "int");
        assert_eq!(type_name("[[B").unwrap(), "byte[][]");
        assert_eq!(
            type_name("[Ljava/lang/String;").unwrap(),
            "java.lang.String[]"
        );
        assert!(type_name("Q").is_err());
        assert!(type_name("[V").is_err());
        let (params, ret) = method_types("(IJ[Ljava/lang/Object;)Ljava/util/Vector;").unwrap();
        assert_eq!(params, ["int", "long", "java.lang.Object[]"]);
        assert_eq!(ret, "java.util.Vector");
    }

    #[test]
    fn constants_keep_their_exact_values() {
        assert_eq!(
            literal("I", &Constant::Int(i32::MIN)).unwrap(),
            "-2147483648"
        );
        assert_eq!(literal("C", &Constant::Int(65)).unwrap(), "(char) 65");
        assert_eq!(literal("Z", &Constant::Int(1)).unwrap(), "true");
        assert_eq!(
            literal("J", &Constant::Long(i64::MIN)).unwrap(),
            "-9223372036854775808L"
        );
        assert_eq!(
            literal("F", &Constant::Float(f32::MAX)).unwrap(),
            "3.4028235e38f"
        );
        assert_eq!(
            literal("F", &Constant::Float(f32::MIN_POSITIVE / 8388608.0)).unwrap(),
            "1e-45f"
        );
        assert_eq!(
            literal("F", &Constant::Float(f32::NAN)).unwrap(),
            "(0.0f / 0.0f)"
        );
        assert_eq!(
            literal("D", &Constant::Double(f64::NEG_INFINITY)).unwrap(),
            "(-1.0d / 0.0d)"
        );
        assert_eq!(literal("D", &Constant::Double(0.1)).unwrap(), "0.1d");
        assert!(literal("I", &Constant::Long(1)).is_err());
    }

    #[test]
    fn strings_escape_what_javac_would_misread() {
        let units: Vec<u16> = "a\"\\\n\u{1}\u{e9}\u{20ac}".encode_utf16().collect();
        assert_eq!(
            string_literal(&units),
            "\"a\\\"\\\\\\n\\001\\u00e9\\u20ac\""
        );
    }

    #[test]
    fn placeholders_pick_one_overload() {
        assert_eq!(placeholder("I"), "(int) 0");
        assert_eq!(placeholder("Ljava/lang/String;"), "(java.lang.String) null");
        assert_eq!(non_constant("Ljava/lang/Object;"), "null");
        assert_eq!(
            non_constant("I"),
            "(int) ((java.lang.Object) null == null ? 0 : 0)"
        );
    }
}
