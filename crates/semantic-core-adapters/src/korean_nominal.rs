//! Adapter-local nominal sound evidence. Spelling/inflection is not meaning.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct KoreanNominalFormIR {
    pub nominal: String,
    pub observed_form: String,
}

impl KoreanNominalFormIR {
    pub fn observe(nominal: &str, form: &str) -> Option<Self> {
        let witness = Self {
            nominal: nominal.into(),
            observed_form: form.into(),
        };
        witness.validate().then_some(witness)
    }

    pub fn validate(&self) -> bool {
        !self.nominal.trim().is_empty()
            && self.nominal.len() <= 512
            && self.observed_coda().is_some()
            && spelled_coda(&self.nominal).is_none_or(|c| Some(c) == self.observed_coda())
    }

    pub fn matches(&self, nominal: &str) -> bool {
        self.nominal.to_lowercase() == nominal.to_lowercase()
    }

    fn observed_coda(&self) -> Option<bool> {
        match self.observed_form.strip_prefix(&self.nominal)? {
            "이" | "은" | "을" => Some(true),
            "가" | "는" | "를" => Some(false),
            _ => None,
        }
    }
}

pub fn spelled_coda(surface: &str) -> Option<bool> {
    let last = surface.chars().last()?;
    ('가'..='힣')
        .contains(&last)
        .then(|| (u32::from(last) - u32::from('가')) % 28 != 0)
}

/// Returns the coda class used by Korean postpositions for a rendered surface.
///
/// This is deliberately separate from [`spelled_coda`]: displayed values can
/// end in closing punctuation or an Arabic digit.  The digit mapping follows
/// the conventional Korean readings 영/일/이/삼/사/오/육/칠/팔/구.
pub(crate) fn surface_coda(surface: &str) -> Option<bool> {
    let last = surface.chars().rev().find(|character| {
        !character.is_whitespace()
            && !matches!(
                character,
                '“' | '”'
                    | '‘'
                    | '’'
                    | '\''
                    | '"'
                    | ')'
                    | ']'
                    | '}'
                    | '〉'
                    | '》'
                    | '」'
                    | '』'
                    | '.'
                    | ','
                    | '?'
                    | '!'
                    | ':'
                    | ';'
            )
    })?;
    match last {
        '0' | '1' | '3' | '6' | '7' | '8' => Some(true),
        '2' | '4' | '5' | '9' => Some(false),
        '가'..='힣' => Some((u32::from(last) - u32::from('가')) % 28 != 0),
        _ => None,
    }
}

/// Whether the rendered surface has final /l/, for the `로/으로` exception.
pub(crate) fn surface_final_rieul(surface: &str) -> Option<bool> {
    let last = surface.chars().rev().find(|character| {
        !character.is_whitespace()
            && !matches!(
                character,
                '“' | '”'
                    | '‘'
                    | '’'
                    | '\''
                    | '"'
                    | ')'
                    | ']'
                    | '}'
                    | '〉'
                    | '》'
                    | '」'
                    | '』'
                    | '.'
                    | ','
                    | '?'
                    | '!'
                    | ':'
                    | ';'
            )
    })?;
    match last {
        '1' | '7' | '8' => Some(true),
        '0' | '2' | '3' | '4' | '5' | '6' | '9' => Some(false),
        '가'..='힣' => Some((u32::from(last) - u32::from('가')) % 28 == 8),
        _ => None,
    }
}

pub(crate) fn select_particle<'a>(
    surface: &str,
    consonant: &'a str,
    vowel: &'a str,
) -> Option<&'a str> {
    surface_coda(surface).map(|coda| if coda { consonant } else { vowel })
}

pub(crate) fn select_directional_particle(surface: &str) -> Option<&'static str> {
    Some(
        if surface_coda(surface)? && !surface_final_rieul(surface)? {
            "으로"
        } else {
            "로"
        },
    )
}

/// Attach a Korean particle when the surface pronunciation is known.  Opaque
/// Latin labels and identifiers are instead quoted before a Korean head noun,
/// so the head supplies deterministic sound evidence without guessing how the
/// opaque label is pronounced.
pub(crate) fn mark_or_label(
    surface: &str,
    consonant: &str,
    vowel: &str,
    korean_head: &str,
) -> String {
    if let Some(particle) = select_particle(surface, consonant, vowel) {
        return format!("{surface}{particle}");
    }
    let particle = select_particle(korean_head, consonant, vowel)
        .expect("a Korean fallback head must expose Hangul coda evidence");
    let label = surface.trim().trim_matches(['‘', '’', '“', '”', '\'', '"']);
    format!("‘{label}’ {korean_head}{particle}")
}

pub(crate) fn mark_direction_or_label(surface: &str, korean_head: &str) -> String {
    if let Some(particle) = select_directional_particle(surface) {
        return format!("{surface}{particle}");
    }
    let particle = select_directional_particle(korean_head)
        .expect("a Korean fallback head must expose Hangul coda evidence");
    let label = surface.trim().trim_matches(['‘', '’', '“', '”', '\'', '"']);
    format!("‘{label}’ {korean_head}{particle}")
}

pub fn final_coda(surface: &str, forms: &[KoreanNominalFormIR]) -> Option<bool> {
    if let Some(coda) = spelled_coda(surface) {
        return Some(coda);
    }
    let mut selected = None;
    for form in forms.iter().filter(|f| f.matches(surface)) {
        if !form.validate() {
            return None;
        }
        let coda = form.observed_coda()?;
        if selected.is_some_and(|prior| prior != coda) {
            return None;
        }
        selected = Some(coda);
    }
    selected
}

/// One witness per label/coda class is enough; contradictory classes survive.
pub fn merge_forms(target: &mut Vec<KoreanNominalFormIR>, incoming: &[KoreanNominalFormIR]) {
    for form in incoming.iter().filter(|f| f.validate()) {
        if !target
            .iter()
            .any(|f| f.matches(&form.nominal) && f.observed_coda() == form.observed_coda())
        {
            target.push(form.clone());
        }
    }
    target.sort();
    target.truncate(64);
}

pub fn validate_forms(forms: &[KoreanNominalFormIR], nominal: &str) -> bool {
    forms.len() <= 2
        && forms.iter().all(|f| f.validate() && f.matches(nominal))
        && forms
            .windows(2)
            .all(|p| p[0] < p[1] && p[0].observed_coda() != p[1].observed_coda())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_evidence_is_not_guessed_from_latin_spelling() {
        assert_eq!(final_coda("나무", &[]), Some(false));
        assert_eq!(final_coda("사람", &[]), Some(true));
        for name in ["HanSol", "Mira", "CCTV", "17", "Qx"] {
            assert_eq!(final_coda(name, &[]), None);
        }
        for (form, expected) in [
            ("Qx이", true),
            ("Qx은", true),
            ("Qx가", false),
            ("Qx를", false),
        ] {
            let witness = KoreanNominalFormIR::observe("Qx", form).unwrap();
            assert_eq!(final_coda("QX", &[witness]), Some(expected));
        }
        assert!(KoreanNominalFormIR::observe("사람", "사람가").is_none());
        assert!(KoreanNominalFormIR::observe("Qx", "Qx이라는").is_none());
    }

    #[test]
    fn contradictory_forms_remain_unknown_instead_of_last_writer_winning() {
        let mut forms = Vec::new();
        for observed in ["Qx이", "Qx은", "Qx가", "Qx는"] {
            merge_forms(
                &mut forms,
                &[KoreanNominalFormIR::observe("Qx", observed).unwrap()],
            );
        }
        assert_eq!(forms.len(), 2);
        assert!(validate_forms(&forms, "Qx"));
        assert_eq!(final_coda("Qx", &forms), None);
    }

    #[test]
    fn rendered_particle_sound_handles_all_hangul_codas_digits_and_wrappers() {
        for jong in 0..28 {
            let syllable = char::from_u32(u32::from('가') + jong).unwrap().to_string();
            assert_eq!(surface_coda(&syllable), Some(jong != 0), "{syllable}");
            assert_eq!(
                surface_final_rieul(&syllable),
                Some(jong == 8),
                "{syllable}"
            );
        }
        for (digit, coda, rieul, directional) in [
            ("0", true, false, "으로"),
            ("1", true, true, "로"),
            ("2", false, false, "로"),
            ("3", true, false, "으로"),
            ("4", false, false, "로"),
            ("5", false, false, "로"),
            ("6", true, false, "으로"),
            ("7", true, true, "로"),
            ("8", true, true, "로"),
            ("9", false, false, "로"),
        ] {
            assert_eq!(surface_coda(digit), Some(coda), "{digit}");
            assert_eq!(surface_final_rieul(digit), Some(rieul), "{digit}");
            assert_eq!(select_directional_particle(digit), Some(directional));
        }
        assert_eq!(surface_coda("‘사람’"), Some(true));
        assert_eq!(surface_coda("“나무”"), Some(false));
        assert_eq!(select_particle("사람", "은", "는"), Some("은"));
        assert_eq!(select_particle("나무", "은", "는"), Some("는"));
        assert_eq!(mark_or_label("node", "은", "는", "항목"), "‘node’ 항목은");
        assert_eq!(mark_or_label("‘node’", "은", "는", "항목"), "‘node’ 항목은");
        assert_eq!(mark_or_label("나무", "은", "는", "항목"), "나무는");
        assert_eq!(mark_direction_or_label("Qx", "항목"), "‘Qx’ 항목으로");
    }
}
