//! Testi del backend (tray, notifiche) nella lingua dell'interfaccia: inglese di default, italiano.
//! ponytail: due lingue con una funzione; un file per lingua quando ne arriva una terza

pub fn tr<'a>(lang: &str, en: &'a str, it: &'a str) -> &'a str {
    if lang == "it" {
        it
    } else {
        en
    }
}

/// Nome inglese di una lingua per i prompt dell'LLM ("de" -> "German").
pub fn language_name(code: &str) -> &str {
    match code {
        "it" => "Italian",
        "en" => "English",
        "es" => "Spanish",
        "fr" => "French",
        "de" => "German",
        "pt" => "Portuguese",
        "nl" => "Dutch",
        "pl" => "Polish",
        "ro" => "Romanian",
        "el" => "Greek",
        "sv" => "Swedish",
        "da" => "Danish",
        "no" => "Norwegian",
        "fi" => "Finnish",
        "cs" => "Czech",
        "hu" => "Hungarian",
        "uk" => "Ukrainian",
        "ru" => "Russian",
        "tr" => "Turkish",
        "ar" => "Arabic",
        "hi" => "Hindi",
        "zh" => "Chinese",
        "ja" => "Japanese",
        "ko" => "Korean",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_italian_only_when_asked() {
        assert_eq!(tr("it", "Ready", "Pronto"), "Pronto");
        assert_eq!(tr("en", "Ready", "Pronto"), "Ready");
        assert_eq!(tr("fr", "Ready", "Pronto"), "Ready");
        assert_eq!(language_name("de"), "German");
        assert_eq!(language_name("xx"), "xx");
    }
}
