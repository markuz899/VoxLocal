//! Pulizia senza LLM del testo dettato: toglie le esitazioni ("ehm", "uhm") e le parole brevi
//! ripetute per sbaglio ("il il"). Vale anche in Grezzo; le modalità con LLM partono da qui.
//!
//! ponytail: regole fisse e prudenti. Le false partenze e le autocorrezioni ("anzi") restano
//! all'LLM o al futuro modello locale.

/// Esitazioni, già con le lettere ripetute ridotte a una ("ehmmm" → "ehm").
/// Non ci sono "eh", "ah", "oh": in italiano sono anche interiezioni vere ("eh sì").
const FILLERS: &[&str] = &[
    "ehm", "em", "ahm", "uhm", "um", "uh", "erm", "hm", "mh", "mhm",
];

fn is_filler(word: &str) -> bool {
    let s = squeeze(word);
    // "mmm" sì, "m" da sola no: può essere "3 m"
    FILLERS.contains(&s.as_str()) || (s == "m" && word.chars().count() > 1)
}

/// Parole che non si ripetono mai apposta. "piano piano" o "molto molto" invece sì, e restano.
const SHORT: &[&str] = &[
    "il", "lo", "la", "i", "gli", "le", "un", "uno", "una", "di", "a", "da", "in", "con", "su",
    "per", "tra", "fra", "e", "o", "che", "non", "mi", "ti", "si", "ci", "vi", "the", "an", "to",
    "of", "and", "or", "in", "on", "is", "it", "i",
];

fn squeeze(word: &str) -> String {
    let mut out = String::new();
    for c in word.chars().flat_map(char::to_lowercase) {
        if !out.ends_with(c) {
            out.push(c);
        }
    }
    out
}

fn core(token: &str) -> &str {
    token.trim_matches(|c: char| !c.is_alphanumeric())
}

fn capitalize(token: &str) -> String {
    let mut chars = token.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

pub fn clean(text: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut capitalize_next = false;
    for token in text.split_whitespace() {
        let word = core(token);
        let lower = word.to_lowercase();

        if !word.is_empty() && is_filler(word) {
            let after = &token[token.find(word).unwrap_or(0) + word.len()..];
            let sentence_start = out.last().is_none_or(|p| p.ends_with(['.', '?', '!']));
            if sentence_start && word.starts_with(char::is_uppercase) {
                capitalize_next = true;
            }
            if let Some(prev) = out.last_mut() {
                let ellipsis = after.contains("..") || after.contains('…');
                let terminal = after.contains(['?', '!']) || (after.contains('.') && !ellipsis);
                if terminal && !prev.ends_with(['.', '?', '!']) {
                    // "va bene ehm." → "va bene."
                    *prev = prev.trim_end_matches([',', ';', ':']).to_string();
                    prev.push_str(after.trim_start_matches([',', ' ']));
                } else if after.starts_with(',') || ellipsis {
                    // "volevo, ehm, dirti" → "volevo dirti"
                    if prev.ends_with(',') && !prev.ends_with(",,") {
                        prev.pop();
                    }
                }
            }
            continue;
        }

        if let Some(prev) = out.last() {
            let prev_word = core(prev);
            if SHORT.contains(&lower.as_str())
                && prev_word.to_lowercase() == lower
                && prev.ends_with(prev_word)
            {
                continue;
            }
        }

        out.push(if capitalize_next {
            capitalize_next = false;
            capitalize(token)
        } else {
            token.to_string()
        });
    }
    let joined = out.join(" ");
    // tutto il dettato era esitazione: meglio niente che "ehm"
    joined.trim_start_matches([',', ' ']).to_string()
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn removes_hesitations() {
        assert_eq!(
            clean("Ehm, allora ci vediamo domani."),
            "Allora ci vediamo domani."
        );
        assert_eq!(
            clean("volevo, ehm, dirti una cosa"),
            "volevo dirti una cosa"
        );
        assert_eq!(clean("Va bene ehmmm."), "Va bene.");
        assert_eq!(clean("So, um, I think uh we can"), "So I think we can");
        assert_eq!(
            clean("Uhm... the meeting is at ten"),
            "The meeting is at ten"
        );
        assert_eq!(clean("Fatto. Ehm, e poi?"), "Fatto. E poi?");
        assert_eq!(clean("ehm"), "");
        assert_eq!(clean("volevo, ehm... dirti"), "volevo dirti");
        assert_eq!(clean("mmm, non so"), "non so");
        assert_eq!(clean("sono 3 m di cavo"), "sono 3 m di cavo");
    }

    #[test]
    fn removes_stutters_but_not_reduplication() {
        assert_eq!(clean("mandami il il file"), "mandami il file");
        assert_eq!(clean("I I think so"), "I think so");
        assert_eq!(
            clean("piano piano ci arriviamo"),
            "piano piano ci arriviamo"
        );
        assert_eq!(clean("è molto molto bello"), "è molto molto bello");
    }

    #[test]
    fn leaves_real_words_alone() {
        let s = "Eh sì, ah ecco: l'emme è una lettera, e Umberto ha detto hmmm no, cioè sì.";
        assert_eq!(
            clean(s),
            "Eh sì, ah ecco: l'emme è una lettera, e Umberto ha detto no, cioè sì."
        );
        assert_eq!(clean("e e-mail"), "e e-mail");
    }
}
