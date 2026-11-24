//! The vendor APIs of Siemens, Samsung, Motorola and LG, which maker a
//! device profile imitates, and the lights, vibration and sound clips they
//! share.
//!
//! Games detect the phone they run on with `Class.forName`
//! (`docs/compat/emulator-notes.md`, rule 13): a game that finds
//! `com.siemens.mp.game.Light` may decide it runs on a Siemens and use
//! Siemens key codes. So a vendor's classes exist only on profiles of that
//! vendor; [`Vendor::of`] reads the vendor from the profile's id.
//! `docs/architecture/vendors.md` lists what each vendor gets.

mod clip;
pub(crate) mod device;
pub mod lg;
pub mod motorola;
pub mod samsung;
pub mod siemens;

use fastemu_core::DeviceProfile;
use fastemu_jvm::NativeClass;

/// The phone maker a device profile imitates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Vendor {
    /// No maker in particular: the `generic-*` profiles and any id this
    /// module does not know. These behave like a Nokia, the most common
    /// target of J2ME games.
    Generic,
    Nokia,
    SonyEricsson,
    Siemens,
    Samsung,
    Motorola,
    Lg,
}

impl Vendor {
    /// The vendor of `profile`, from the first word of its id
    /// (`siemens-132x176` is a Siemens). Unknown words give
    /// [`Vendor::Generic`].
    pub fn of(profile: &DeviceProfile) -> Self {
        let word = profile
            .id
            .split(['-', '_', ' '])
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        match word.as_str() {
            "nokia" => Self::Nokia,
            "sonyericsson" | "se" => Self::SonyEricsson,
            "siemens" => Self::Siemens,
            "samsung" => Self::Samsung,
            "motorola" | "moto" => Self::Motorola,
            "lg" => Self::Lg,
            _ => Self::Generic,
        }
    }

    /// Whether the vendor's phones had Nokia's UI API
    /// (`com.nokia.mid.ui` and `com.nokia.mid.sound`): Nokia's own, Sony
    /// Ericsson's and the generic profiles, which all use Nokia's key
    /// codes. A game that finds `FullCanvas` assumes those codes, so the
    /// makers with key codes of their own (Siemens, Motorola) do not get
    /// it, nor do Samsung and LG, for which no source in
    /// `docs/compat/notes` shows the API.
    pub fn has_nokia_ui(self) -> bool {
        matches!(self, Self::Generic | Self::Nokia | Self::SonyEricsson)
    }
}

/// The classes of `vendor`'s own APIs. Nokia's are in [`crate::nokia`].
pub fn classes(vendor: Vendor) -> Vec<NativeClass> {
    match vendor {
        Vendor::Siemens => siemens::classes(),
        Vendor::Samsung => samsung::classes(),
        Vendor::Motorola => motorola::classes(),
        Vendor::Lg => lg::classes(),
        // Sony Ericsson shipped standard JSRs and Nokia's UI API; its own
        // Capuchin (Flash) API is rare in games.
        Vendor::Generic | Vendor::Nokia | Vendor::SonyEricsson => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use fastemu_core::{KeyCodes, builtin_profiles, default_profile};

    use super::*;

    fn with_id(id: &str) -> DeviceProfile {
        DeviceProfile {
            id: id.to_owned(),
            ..default_profile()
        }
    }

    #[test]
    fn the_vendor_comes_from_the_first_word_of_the_id() {
        assert_eq!(Vendor::of(&with_id("siemens-132x176")), Vendor::Siemens);
        assert_eq!(Vendor::of(&with_id("Samsung_D500")), Vendor::Samsung);
        assert_eq!(Vendor::of(&with_id("lg-u8180")), Vendor::Lg);
        assert_eq!(Vendor::of(&with_id("moto-v600")), Vendor::Motorola);
        assert_eq!(Vendor::of(&with_id("generic-240x320")), Vendor::Generic);
        assert_eq!(Vendor::of(&with_id("my phone")), Vendor::Generic);
        assert_eq!(Vendor::of(&with_id("")), Vendor::Generic);
    }

    #[test]
    fn every_builtin_profile_has_the_vendor_its_name_says() {
        for profile in builtin_profiles() {
            let name = profile.name.to_ascii_lowercase();
            let expected = [
                ("generic", Vendor::Generic),
                ("nokia", Vendor::Nokia),
                ("sony ericsson", Vendor::SonyEricsson),
                ("siemens", Vendor::Siemens),
                ("samsung", Vendor::Samsung),
                ("motorola", Vendor::Motorola),
                ("lg", Vendor::Lg),
            ]
            .into_iter()
            .find(|(prefix, _)| name.starts_with(prefix))
            .map(|(_, vendor)| vendor);
            assert_eq!(Some(Vendor::of(&profile)), expected, "{}", profile.id);
        }
    }

    #[test]
    fn nokia_ui_goes_with_nokia_key_codes() {
        for profile in builtin_profiles() {
            assert_eq!(
                Vendor::of(&profile).has_nokia_ui(),
                profile.key_codes == KeyCodes::NOKIA,
                "{}",
                profile.id
            );
        }
    }
}
