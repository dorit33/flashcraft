//! Interface languages — the same set Rufus ships. English text in the code is
//! the lookup key; translations live in `i18n/<lang>.tsv` (`key<TAB>text`).
//! Adding a language means adding a `Lang` variant, a catalog and a label.
//! Translations are imported from Rufus' `res/loc/rufus.loc` (GPLv3 — the same
//! license as this project) by `tools/loc-import.py`; keys that have no Rufus
//! counterpart are translated by hand.

/// User-selectable language. `Auto` follows the system locale.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Auto,
    Ar,
    Bg,
    Cs,
    Da,
    De,
    El,
    En,
    Es,
    Fa,
    Fi,
    Fr,
    He,
    Hr,
    Hu,
    Id,
    It,
    Ja,
    Ko,
    Lt,
    Lv,
    Ms,
    Nb,
    Nl,
    Pl,
    PtBr,
    PtPt,
    Ro,
    Ru,
    Sk,
    Sl,
    Sr,
    Sv,
    Th,
    Tr,
    Uk,
    Vi,
    ZhCn,
    ZhTw,
}

/// The concrete language used for lookups (a `Lang` with `Auto` resolved).
pub type Resolved = Lang;

impl Lang {
    pub const ALL: &[Lang] = &[
        Lang::Auto,
        Lang::Ar,
        Lang::Bg,
        Lang::Cs,
        Lang::Da,
        Lang::De,
        Lang::El,
        Lang::En,
        Lang::Es,
        Lang::Fa,
        Lang::Fi,
        Lang::Fr,
        Lang::He,
        Lang::Hr,
        Lang::Hu,
        Lang::Id,
        Lang::It,
        Lang::Ja,
        Lang::Ko,
        Lang::Lt,
        Lang::Lv,
        Lang::Ms,
        Lang::Nb,
        Lang::Nl,
        Lang::Pl,
        Lang::PtBr,
        Lang::PtPt,
        Lang::Ro,
        Lang::Ru,
        Lang::Sk,
        Lang::Sl,
        Lang::Sr,
        Lang::Sv,
        Lang::Th,
        Lang::Tr,
        Lang::Uk,
        Lang::Vi,
        Lang::ZhCn,
        Lang::ZhTw,
    ];

    /// Native name shown in the picker (never translated).
    pub fn label(self) -> &'static str {
        match self {
            Lang::Auto => "Auto",
            Lang::Ar => "العربية",
            Lang::Bg => "Български",
            Lang::Cs => "Čeština",
            Lang::Da => "Dansk",
            Lang::De => "Deutsch",
            Lang::El => "Ελληνικά",
            Lang::En => "English",
            Lang::Es => "Español",
            Lang::Fa => "پارسی",
            Lang::Fi => "Suomi",
            Lang::Fr => "Français",
            Lang::He => "עברית",
            Lang::Hr => "Hrvatski",
            Lang::Hu => "Magyar",
            Lang::Id => "Bahasa Indonesia",
            Lang::It => "Italiano",
            Lang::Ja => "日本語",
            Lang::Ko => "한국어",
            Lang::Lt => "Lietuvių",
            Lang::Lv => "Latviešu",
            Lang::Ms => "Bahasa Malaysia",
            Lang::Nb => "Norsk",
            Lang::Nl => "Nederlands",
            Lang::Pl => "Polski",
            Lang::PtBr => "Português do Brasil",
            Lang::PtPt => "Português",
            Lang::Ro => "Română",
            Lang::Ru => "Русский",
            Lang::Sk => "Slovensky",
            Lang::Sl => "Slovenščina",
            Lang::Sr => "Srpski",
            Lang::Sv => "Svenska",
            Lang::Th => "ไทย",
            Lang::Tr => "Türkçe",
            Lang::Uk => "Українська",
            Lang::Vi => "Tiếng Việt",
            Lang::ZhCn => "简体中文",
            Lang::ZhTw => "正體中文",
        }
    }

    /// The language actually used for lookup (`Auto` → detected locale).
    pub fn resolve(self) -> Lang {
        match self {
            Lang::Auto => detect(),
            other => other,
        }
    }

    /// Right-to-left scripts (Arabic, Persian, Hebrew).
    pub fn right_to_left(self) -> bool {
        matches!(self.resolve(), Lang::Ar | Lang::Fa | Lang::He)
    }
}

/// Follow the system locale: `<lang>_<region>` prefixes map to variants.
fn detect() -> Lang {
    let mut val = String::new();
    for var in ["FLASHCRAFT_LANG", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() {
                val = v;
                break;
            }
        }
    }
    let lower = val.to_ascii_lowercase().replace('-', "_");
    match lower.split('_').next().unwrap_or("") {
        "ar" => Lang::Ar,
        "bg" => Lang::Bg,
        "cs" => Lang::Cs,
        "da" => Lang::Da,
        "de" => Lang::De,
        "el" => Lang::El,
        "es" => Lang::Es,
        "fa" => Lang::Fa,
        "fi" => Lang::Fi,
        "fr" => Lang::Fr,
        "he" | "iw" => Lang::He,
        "hr" => Lang::Hr,
        "hu" => Lang::Hu,
        "id" | "in" => Lang::Id,
        "it" => Lang::It,
        "ja" => Lang::Ja,
        "ko" => Lang::Ko,
        "lt" => Lang::Lt,
        "lv" => Lang::Lv,
        "ms" => Lang::Ms,
        "nb" | "nn" | "no" => Lang::Nb,
        "nl" => Lang::Nl,
        "pl" => Lang::Pl,
        "pt" if lower.contains("_br") => Lang::PtBr,
        "pt" => Lang::PtPt,
        "ro" => Lang::Ro,
        "ru" => Lang::Ru,
        "sk" => Lang::Sk,
        "sl" => Lang::Sl,
        "sr" => Lang::Sr,
        "sv" => Lang::Sv,
        "th" => Lang::Th,
        "tr" => Lang::Tr,
        "uk" => Lang::Uk,
        "vi" => Lang::Vi,
        "zh" if lower.contains("_tw") || lower.contains("_hk") => Lang::ZhTw,
        "zh" => Lang::ZhCn,
        _ => Lang::En,
    }
}

/// The TSV catalog bundled per language; `None` means "use English".
fn catalog(lang: Lang) -> Option<&'static str> {
    match lang {
        Lang::Ar => Some(include_str!("i18n/ar.tsv")),
        Lang::Bg => Some(include_str!("i18n/bg.tsv")),
        Lang::Cs => Some(include_str!("i18n/cs.tsv")),
        Lang::Da => Some(include_str!("i18n/da.tsv")),
        Lang::De => Some(include_str!("i18n/de.tsv")),
        Lang::El => Some(include_str!("i18n/el.tsv")),
        Lang::Es => Some(include_str!("i18n/es.tsv")),
        Lang::Fa => Some(include_str!("i18n/fa.tsv")),
        Lang::Fi => Some(include_str!("i18n/fi.tsv")),
        Lang::Fr => Some(include_str!("i18n/fr.tsv")),
        Lang::He => Some(include_str!("i18n/he.tsv")),
        Lang::Hr => Some(include_str!("i18n/hr.tsv")),
        Lang::Hu => Some(include_str!("i18n/hu.tsv")),
        Lang::Id => Some(include_str!("i18n/id.tsv")),
        Lang::It => Some(include_str!("i18n/it.tsv")),
        Lang::Ja => Some(include_str!("i18n/ja.tsv")),
        Lang::Ko => Some(include_str!("i18n/ko.tsv")),
        Lang::Lt => Some(include_str!("i18n/lt.tsv")),
        Lang::Lv => Some(include_str!("i18n/lv.tsv")),
        Lang::Ms => Some(include_str!("i18n/ms.tsv")),
        Lang::Nb => Some(include_str!("i18n/nb.tsv")),
        Lang::Nl => Some(include_str!("i18n/nl.tsv")),
        Lang::Pl => Some(include_str!("i18n/pl.tsv")),
        Lang::PtBr => Some(include_str!("i18n/pt-BR.tsv")),
        Lang::PtPt => Some(include_str!("i18n/pt-PT.tsv")),
        Lang::Ro => Some(include_str!("i18n/ro.tsv")),
        Lang::Ru => Some(include_str!("i18n/ru.tsv")),
        Lang::Sk => Some(include_str!("i18n/sk.tsv")),
        Lang::Sl => Some(include_str!("i18n/sl.tsv")),
        Lang::Sr => Some(include_str!("i18n/sr.tsv")),
        Lang::Sv => Some(include_str!("i18n/sv.tsv")),
        Lang::Th => Some(include_str!("i18n/th.tsv")),
        Lang::Tr => Some(include_str!("i18n/tr.tsv")),
        Lang::Uk => Some(include_str!("i18n/uk.tsv")),
        Lang::Vi => Some(include_str!("i18n/vi.tsv")),
        Lang::ZhCn => Some(include_str!("i18n/zh-CN.tsv")),
        Lang::ZhTw => Some(include_str!("i18n/zh-TW.tsv")),
        Lang::Auto | Lang::En => None,
    }
}

/// Look up `key` in the resolved language. Unknown keys fall back to the key
/// itself (the English text), so a missing row never renders blank.
pub fn t(key: &str, lang: Lang) -> std::borrow::Cow<'static, str> {
    if let Some(cat) = catalog(lang) {
        for line in cat.lines() {
            if let Some((k, v)) = line.split_once('\t') {
                if k == key && !v.is_empty() {
                    return std::borrow::Cow::Borrowed(v);
                }
            }
        }
    }
    std::borrow::Cow::Owned(key.to_string())
}

/// `t()` with `{0}` substitution.
pub fn tf(key: &str, lang: Lang, arg0: &str) -> String {
    t(key, lang).replace("{0}", arg0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every catalog must parse and translate the core keys (no blank rows).
    #[test]
    fn catalogs_cover_core_keys() {
        let keys = ["Device", "START", "READY", "Volume label", "Cancel"];
        for lang in Lang::ALL {
            let resolved = lang.resolve();
            for key in keys {
                let v = t(key, resolved);
                assert!(!v.is_empty(), "{:?} misses {key}", lang);
            }
        }
    }

    #[test]
    fn fallback_and_format() {
        assert_eq!(t("untranslated key", Lang::Hu), "untranslated key");
        assert_eq!(
            tf(
                "WARNING: ALL DATA ON DEVICE '{0}' WILL BE DESTROYED",
                Lang::Hu,
                "sdb"
            ),
            "FIGYELMEZTETÉS: A(Z) 'sdb' ESZKÖZÖN AZ ÖSSZES ADAT LE LESZ TÖRÖLVE!"
        );
    }

    #[test]
    fn rtl_flags() {
        assert!(Lang::Ar.right_to_left());
        assert!(Lang::He.right_to_left());
        assert!(!Lang::Hu.right_to_left());
    }
}
