//! The errors graphics calls report, one per Java exception MIDP throws.

use std::fmt;

/// A graphics call that failed the way MIDP says it must.
///
/// Each variant stands for one Java exception, so the MIDP wrapper throws
/// [`GfxError::java_class`] with [`GfxError::message`] and needs no table
/// of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GfxError {
    /// `IllegalArgumentException`: a bad anchor, transform, size, region,
    /// colour component or font constant.
    IllegalArgument(&'static str),
    /// `IllegalArgumentException` from `Image.createImage`: the bytes are not
    /// an image FastEmu can decode. `createImage(String)` throws
    /// `IOException` for the same failure; the wrapper picks.
    Decode(String),
    /// `ArrayIndexOutOfBoundsException`: an offset, length or scan length
    /// reaches outside a Java array.
    ArrayIndexOutOfBounds,
    /// `StringIndexOutOfBoundsException`: a substring range outside its
    /// string.
    StringIndexOutOfBounds,
    /// `NullPointerException`. Java references never reach this crate as
    /// null; the wrapper turns a missing one into this through [`non_null`].
    NullPointer,
    /// `IllegalStateException`, such as `copyArea` on the screen.
    IllegalState(&'static str),
    /// `OutOfMemoryError`: an image larger than FastEmu allocates.
    OutOfMemory,
}

impl GfxError {
    /// The binary name of the Java exception class to throw, in the form
    /// `FindClass` takes.
    pub fn java_class(&self) -> &'static str {
        match self {
            Self::IllegalArgument(_) | Self::Decode(_) => "java/lang/IllegalArgumentException",
            Self::ArrayIndexOutOfBounds => "java/lang/ArrayIndexOutOfBoundsException",
            Self::StringIndexOutOfBounds => "java/lang/StringIndexOutOfBoundsException",
            Self::NullPointer => "java/lang/NullPointerException",
            Self::IllegalState(_) => "java/lang/IllegalStateException",
            Self::OutOfMemory => "java/lang/OutOfMemoryError",
        }
    }

    /// The detail message for the exception, if there is one.
    pub fn message(&self) -> Option<&str> {
        match self {
            Self::IllegalArgument(why) | Self::IllegalState(why) => Some(why),
            Self::Decode(why) => Some(why),
            _ => None,
        }
    }
}

impl fmt::Display for GfxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let class = self.java_class().rsplit('/').next().unwrap_or_default();
        match self.message() {
            Some(why) => write!(f, "{class}: {why}"),
            None => f.write_str(class),
        }
    }
}

impl std::error::Error for GfxError {}

/// The result of a graphics call.
pub type Result<T, E = GfxError> = std::result::Result<T, E>;

/// Unwraps a Java reference that may be null, failing with
/// [`GfxError::NullPointer`], so the wrapper reports null arguments through
/// the same error type as everything else.
pub fn non_null<T>(value: Option<T>) -> Result<T> {
    value.ok_or(GfxError::NullPointer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_name_their_java_classes() {
        assert_eq!(
            GfxError::IllegalArgument("x").java_class(),
            "java/lang/IllegalArgumentException"
        );
        assert_eq!(
            GfxError::Decode("x".into()).java_class(),
            "java/lang/IllegalArgumentException"
        );
        assert_eq!(
            GfxError::IllegalState("x").to_string(),
            "IllegalStateException: x"
        );
        assert_eq!(
            GfxError::ArrayIndexOutOfBounds.to_string(),
            "ArrayIndexOutOfBoundsException"
        );
        assert_eq!(GfxError::OutOfMemory.message(), None);
    }

    #[test]
    fn non_null_maps_none_to_a_null_pointer() {
        assert_eq!(non_null(Some(3)), Ok(3));
        assert_eq!(non_null::<u8>(None), Err(GfxError::NullPointer));
    }
}
