/// The most characters of a suite's name, vendor, version and MIDlet names
/// that [`SuiteInfo`] holds for display. The longest of them in the test
/// folder has 40. The attributes in [`SuiteInfo::properties`] stay whole.
/// The suite reader cuts them to this, and the app's library cuts them
/// again before it keeps them.
pub const MAX_DISPLAY_CHARS: usize = 100;

/// What a MIDlet suite (a `.jar`, optionally with its `.jad`) says about
/// itself, read without starting it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SuiteInfo {
    /// `MIDlet-Name`.
    pub name: String,
    /// `MIDlet-Vendor`.
    pub vendor: String,
    /// `MIDlet-Version`.
    pub version: String,
    /// `MIDlet-Description`.
    pub description: Option<String>,
    /// The suite's icon as stored in the JAR (PNG, most often), from
    /// `MIDlet-Icon` or else the first MIDlet's icon.
    pub icon: Option<Vec<u8>>,
    /// The MIDlets the suite lists as `MIDlet-1`, `MIDlet-2`, and so on.
    pub midlets: Vec<MidletInfo>,
    /// Every attribute of the descriptor and the manifest, the descriptor's
    /// first, as the application properties `MIDlet.getAppProperty` reads.
    pub properties: Vec<(String, String)>,
    /// The size of the JAR in bytes.
    pub jar_bytes: u64,
}

impl SuiteInfo {
    /// The value of attribute `name`. MIDP says descriptor attribute names
    /// are case-sensitive, but this lookup ignores case: suites in the wild
    /// spell names loosely, and a phone that found nothing returned `null`,
    /// which no game depends on.
    pub fn property(&self, name: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }
}

/// One `MIDlet-<n>` entry of a suite.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MidletInfo {
    pub name: String,
    /// The icon's path inside the JAR.
    pub icon_path: Option<String>,
    /// The fully qualified class, such as `com.example.Game`.
    pub class_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn property_ignores_case() {
        let info = SuiteInfo {
            properties: vec![("MIDlet-Name".into(), "Snake".into())],
            ..SuiteInfo::default()
        };
        assert_eq!(info.property("midlet-name"), Some("Snake"));
        assert_eq!(info.property("MIDlet-Vendor"), None);
    }
}
