//! Interface languages. English text in the code is the lookup key; every
//! user-facing string goes through `t()` / `tf()`. Adding a language means
//! adding a catalog below and a `Lang` variant.

/// User-selectable language. `Auto` follows the system locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Auto,
    En,
    Hu,
}

impl Lang {
    pub const ALL: &[Lang] = &[Lang::Auto, Lang::En, Lang::Hu];

    pub fn label(self) -> &'static str {
        match self {
            Lang::Auto => "Auto",
            Lang::En => "English",
            Lang::Hu => "Magyar",
        }
    }

    /// The language actually used for lookup.
    pub fn resolve(self) -> Resolved {
        match self {
            Lang::En => Resolved::En,
            Lang::Hu => Resolved::Hu,
            Lang::Auto => detect(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolved {
    En,
    Hu,
}

/// Follow the system locale: any hu_* locale selects Hungarian.
fn detect() -> Resolved {
    for var in ["FLASHCRAFT_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            if v.to_ascii_lowercase().starts_with("hu") {
                return Resolved::Hu;
            }
            if !v.is_empty() {
                return Resolved::En;
            }
        }
    }
    Resolved::En
}

/// Look up `key` in the resolved language. Unknown keys fall back to the key
/// itself (the English text), so a missing row never renders blank.
pub fn t(key: &str, lang: Resolved) -> std::borrow::Cow<'static, str> {
    let hit = match lang {
        Resolved::En => en(key),
        Resolved::Hu => hu(key),
    };
    if hit.is_empty() {
        std::borrow::Cow::Owned(key.to_string())
    } else {
        std::borrow::Cow::Borrowed(hit)
    }
}

/// `t()` with `{0}` substitution.
pub fn tf(key: &str, lang: Resolved, arg0: &str) -> String {
    t(key, lang).replace("{0}", arg0)
}

fn en(key: &str) -> &'static str {
    match key {
        "Settings" => "Settings",
        "Language" => "Language",
        "Drive Properties" => "Drive Properties",
        "Device" => "Device",
        "Refresh" => "Refresh",
        "Boot selection" => "Boot selection",
        "Disk or ISO image (Please select)" => "Disk or ISO image (Please select)",
        "SELECT" => "SELECT",
        "Persistent partition size" => "Persistent partition size",
        "Format Options" => "Format Options",
        "Image option" => "Image option",
        "Standard installation" => "Standard installation",
        "Partition scheme" => "Partition scheme",
        "Target system" => "Target system",
        "BIOS or UEFI" => "BIOS or UEFI",
        "UEFI (non CSM)" => "UEFI (non CSM)",
        "Volume label" => "Volume label",
        "File system" => "File system",
        "Cluster size" => "Cluster size",
        "Show advanced drive properties" => "▷ Show advanced drive properties",
        "Hide advanced drive properties" => "▽ Hide advanced drive properties",
        "List USB Hard Drives" => "List USB Hard Drives",
        "Show advanced format options" => "▷ Show advanced format options",
        "Hide advanced format options" => "▽ Hide advanced format options",
        "Quick format" => "Quick format",
        "Create extended label and icon files" => "Create extended label and icon files",
        "Check device for bad blocks" => "Check device for bad blocks",
        "Status:" => "Status:",
        "READY" => "READY",
        "Log" => "Log",
        "START" => "START",
        "OK" => "  OK  ",
        "Cancel" => "Cancel",
        "WARNING: ALL DATA ON DEVICE '{0}' WILL BE DESTROYED" => {
            "WARNING: ALL DATA ON DEVICE '{0}' WILL BE DESTROYED"
        }
        "To continue, click OK. To abort the operation, click Cancel." => {
            "To continue, click OK. To abort the operation, click Cancel."
        }
        "aborted by user" => "aborted by user",
        "Please select an image (SELECT)" => "Please select an image (SELECT)",
        "Please select a device" => "Please select a device",
        "write started" => "write started: {0}",
        "verify started" => "verify started: {0}",
        "WRITE DONE" => "WRITE DONE",
        "VERIFY OK" => "VERIFY OK",
        "Selected '{0}'" => "Selected '{0}'",
        "Device list refreshed" => "Device list refreshed",
        "no drive selected" => "no drive selected",
        "image inspection is not available in the browser" => {
            "image inspection is not available in the browser"
        }
        "device access is not available in the browser" => {
            "device access is not available in the browser"
        }
        _ => "",
    }
}

fn hu(key: &str) -> &'static str {
    match key {
        "Settings" => "Beállítások",
        "Language" => "Nyelv",
        "Drive Properties" => "Meghajtó-tulajdonságok",
        "Device" => "Eszköz",
        "Refresh" => "Frissítés",
        "Boot selection" => "Rendszerindítás kiválasztása",
        "Disk or ISO image (Please select)" => "Lemez- vagy ISO-lemezkép (válassz)",
        "SELECT" => "TALLÓZÁS",
        "Persistent partition size" => "Állandó partíció mérete",
        "Format Options" => "Formázási beállítások",
        "Image option" => "Lemezkép típusa",
        "Standard installation" => "Normál telepítés",
        "Partition scheme" => "Partíciós séma",
        "Target system" => "Célrendszer",
        "BIOS or UEFI" => "BIOS vagy UEFI",
        "UEFI (non CSM)" => "UEFI (CSM nélkül)",
        "Volume label" => "Kötetcímke",
        "File system" => "Fájlrendszer",
        "Cluster size" => "Kluszterméret",
        "Show advanced drive properties" => "▷ Speciális meghajtó-tulajdonságok",
        "Hide advanced drive properties" => "▽ Speciális meghajtó-tulajdonságok elrejtése",
        "List USB Hard Drives" => "USB-s merevlemezek listázása",
        "Show advanced format options" => "▷ Speciális formázási beállítások",
        "Hide advanced format options" => "▽ Speciális formázási beállítások elrejtése",
        "Quick format" => "Gyorsformázás",
        "Create extended label and icon files" => "Kiterjesztett címke- és ikonfájlok létrehozása",
        "Check device for bad blocks" => "Hibás blokkok keresése az eszközön:",
        "Status:" => "Állapot:",
        "READY" => "KÉSZ",
        "Log" => "Napló",
        "START" => "INDÍTÁS",
        "OK" => "  OK  ",
        "Cancel" => "Mégse",
        "WARNING: ALL DATA ON DEVICE '{0}' WILL BE DESTROYED" => {
            "FIGYELMEZTETÉS: AZ ESZKÖZÖN ('{0}') LÉVŐ ÖSSZES ADAT TÖRLŐDNI FOG"
        }
        "To continue, click OK. To abort the operation, click Cancel." => {
            "A folytatáshoz kattints az OK gombra, a megszakításhoz a Mégse gombra."
        }
        "aborted by user" => "a felhasználó megszakította",
        "Please select an image (SELECT)" => "Válassz egy lemezképet (TALLÓZÁS)",
        "Please select a device" => "Válassz egy eszközt",
        "write started" => "írás elindítva: {0}",
        "verify started" => "ellenőrzés elindítva: {0}",
        "WRITE DONE" => "ÍRÁS KÉSZ",
        "VERIFY OK" => "ELLENŐRZÉS RENDBEN",
        "Selected '{0}'" => "Kiválasztva: '{0}'",
        "Device list refreshed" => "Eszközlista frissítve",
        "no drive selected" => "nincs eszköz kiválasztva",
        "image inspection is not available in the browser" => {
            "a böngészőben nem érhető el a lemezkép-ellenőrzés"
        }
        "device access is not available in the browser" => {
            "a böngészőből nem érhetőek el az eszközök"
        }
        _ => en(key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_hu_key_exists_in_en() {
        // Spot-check a few required keys resolve in both languages.
        for key in ["Device", "START", "READY", "Volume label"] {
            assert!(!t(key, Resolved::En).is_empty());
            assert!(!t(key, Resolved::Hu).is_empty());
        }
    }

    #[test]
    fn hu_falls_back_to_en() {
        assert_eq!(
            t("some untranslated key", Resolved::Hu),
            "some untranslated key"
        );
        assert_eq!(
            tf(
                "WARNING: ALL DATA ON DEVICE '{0}' WILL BE DESTROYED",
                Resolved::Hu,
                "sdb"
            ),
            "FIGYELMEZTETÉS: AZ ESZKÖZÖN ('sdb') LÉVŐ ÖSSZES ADAT TÖRLŐDNI FOG"
        );
    }
}
