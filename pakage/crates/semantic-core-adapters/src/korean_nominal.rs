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
}
