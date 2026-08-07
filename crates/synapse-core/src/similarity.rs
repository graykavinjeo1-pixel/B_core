use std::collections::BTreeSet;

pub fn lexical_similarity<'a>(
    query: &str,
    candidates: impl IntoIterator<Item = &'a String>,
) -> f32 {
    let query_tokens = tokenize(query);
    let mut candidate_tokens = BTreeSet::new();
    for candidate in candidates {
        candidate_tokens.extend(tokenize(candidate));
    }

    if query_tokens.is_empty() || candidate_tokens.is_empty() {
        return 0.0;
    }

    let intersection = query_tokens.intersection(&candidate_tokens).count();
    if intersection > 0 {
        let union = query_tokens.union(&candidate_tokens).count();
        return intersection as f32 / union as f32;
    }

    let query_joined = query_tokens.iter().cloned().collect::<String>();
    let candidate_joined = candidate_tokens.iter().cloned().collect::<String>();

    if !query_joined.is_empty() && candidate_joined.contains(&query_joined) {
        return query_joined.len() as f32 / candidate_joined.len().max(1) as f32;
    }
    if !candidate_joined.is_empty() && query_joined.contains(&candidate_joined) {
        return candidate_joined.len() as f32 / query_joined.len().max(1) as f32;
    }

    0.0
}

fn tokenize(text: &str) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch.to_lowercase().next().unwrap_or(ch));
        } else if !current.is_empty() {
            tokens.insert(std::mem::take(&mut current));
        }
    }

    if !current.is_empty() {
        tokens.insert(current);
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::lexical_similarity;

    #[test]
    fn korean_exact_token_matches() {
        let candidates = vec!["고양이".to_string()];
        assert!(lexical_similarity("고양이", &candidates) > 0.9);
    }
}
