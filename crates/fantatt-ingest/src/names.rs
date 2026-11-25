//! Names as the portal spells them, normalized to one canonical form.
//!
//! Player names arrive upper-case with run-together apostrophes and doubled
//! spaces; the prototype title-cased them, and the identity reconciliation
//! (G1.6) resolves them against the same form, so the transformation is a
//! pure shared function instead of two drifting copies.

/// A player name, or a doubles pair spelled `SURNAME I - SURNAME I`, in its
/// canonical form: trimmed, inner whitespace collapsed, every word
/// title-cased (`FACENDA SAMUELE` → `Facenda Samuele`, `D'ANGELO MARIO` →
/// `D'Angelo Mario`).
pub fn normalize_name(raw: &str) -> String {
    raw.split_whitespace()
        .map(title_case)
        .collect::<Vec<_>>()
        .join(" ")
}

/// A team name in its canonical form: trimmed and inner whitespace
/// collapsed. The portal spells societies in mixed capitals that carry no
/// case rule, so the record keeps the spelling the source serves.
pub fn normalize_team(raw: &str) -> String {
    collapse_whitespace(raw)
}

// why: a cased character after an uncased one starts a word, which is the
// title-casing rule the prototype's str.title() applied
fn title_case(word: &str) -> String {
    let mut out = String::with_capacity(word.len());
    let mut previous: Option<char> = None;
    for character in word.chars() {
        if previous.is_none_or(|prior| !prior.is_alphanumeric()) {
            out.extend(character.to_uppercase());
        } else {
            out.extend(character.to_lowercase());
        }
        previous = Some(character);
    }
    out
}

/// Collapse every run of whitespace, including no-break spaces the portal
/// prints as entities, into single spaces.
pub fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_player_name_title_cases_every_word() {
        assert_eq!(normalize_name("  FACENDA   SAMUELE "), "Facenda Samuele");
        assert_eq!(normalize_name("DE LUCA MARCO"), "De Luca Marco");
        assert_eq!(normalize_name("MC DONALD"), "Mc Donald");
    }

    #[test]
    fn an_apostrophe_starts_a_new_word() {
        assert_eq!(normalize_name("D'ANGELO MARIO"), "D'Angelo Mario");
        assert_eq!(normalize_name("CITTA'"), "Citta'");
    }

    #[test]
    fn a_doubles_pair_keeps_its_separator() {
        assert_eq!(normalize_name("MOTTA L - RIZZO A"), "Motta L - Rizzo A");
        assert_eq!(normalize_name("ROSSI A/BIANCHI B"), "Rossi A/Bianchi B");
    }

    #[test]
    fn an_already_canonical_name_is_stable() {
        let canonical = normalize_name("NICOLO' ANDREA");
        assert_eq!(normalize_name(&canonical), canonical);
        assert_eq!(canonical, "Nicolo' Andrea");
    }

    #[test]
    fn a_team_name_keeps_the_portal_spelling() {
        assert_eq!(
            normalize_team(" T.T. REGGIO EMILIA GRISSIN BON "),
            "T.T. REGGIO EMILIA GRISSIN BON"
        );
        assert_eq!(
            normalize_team("TT Vercelli  Mazzeri"),
            "TT Vercelli Mazzeri"
        );
    }

    #[test]
    fn an_empty_name_stays_empty() {
        assert_eq!(normalize_name("   "), "");
        assert_eq!(normalize_team(""), "");
    }
}
