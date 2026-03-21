#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct MatchSignals {
    pub stored_binding: bool,
    pub fingerprint_match: bool,
    pub exact_title_match: bool,
    pub fuzzy_title_score: f64,
    pub exact_creator_match: bool,
    pub fuzzy_creator_score: f64,
    pub file_name_similarity: f64,
    pub category_match: bool,
    pub image_similarity: f64,
    pub download_name_similarity: f64,
    pub user_confirmed_source: bool,
}

/// Calculates a match score from the given signals.
#[allow(dead_code)]
pub fn score_match(signals: &MatchSignals) -> f64 {
    let mut score = 0.0;

    if signals.user_confirmed_source {
        score += 50.0;
    }
    if signals.stored_binding {
        score += 40.0;
    }
    if signals.fingerprint_match {
        score += 40.0;
    }
    if signals.exact_title_match {
        score += 20.0;
    }
    score += signals.fuzzy_title_score * 15.0;
    if signals.exact_creator_match {
        score += 20.0;
    }
    score += signals.fuzzy_creator_score * 10.0;
    score += signals.file_name_similarity * 15.0;
    if signals.category_match {
        score += 10.0;
    }
    score += signals.image_similarity * 10.0;
    score += signals.download_name_similarity * 15.0;

    score.min(100.0)
}

/// Returns a confidence level label for a given score.
#[allow(dead_code)]
pub fn confidence_level(score: f64) -> &'static str {
    match score {
        s if s >= 90.0 => "confirmed",
        s if s >= 70.0 => "probable",
        s if s >= 50.0 => "weak",
        _ => "rejected",
    }
}

/// Returns true if a score is high enough for automatic binding.
/// AUTO-BIND DISABLED: Users must always manually confirm sources.
#[allow(dead_code)]
pub fn should_auto_bind(_score: f64) -> bool {
    false
}

/// Computes similarity between two strings using normalized edit distance.
/// Returns a value between 0.0 (completely different) and 1.0 (identical).
#[allow(dead_code)]
pub fn string_similarity(a: &str, b: &str) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let len_a = a.len();
    let len_b = b.len();
    let max_len = len_a.max(len_b);

    let distance = simple_edit_distance(a, b);
    1.0 - (distance as f64 / max_len as f64)
}

#[allow(dead_code)]
fn simple_edit_distance(a: &str, b: &str) -> usize {
    let chars_a: Vec<char> = a.chars().collect();
    let chars_b: Vec<char> = b.chars().collect();
    let len_a = chars_a.len();
    let len_b = chars_b.len();

    if len_a == 0 {
        return len_b;
    }
    if len_b == 0 {
        return len_a;
    }

    let mut matrix = vec![vec![0usize; len_b + 1]; len_a + 1];

    for i in 0..=len_a {
        matrix[i][0] = i;
    }
    for j in 0..=len_b {
        matrix[0][j] = j;
    }

    for i in 1..=len_a {
        for j in 1..=len_b {
            let cost = if chars_a[i - 1] == chars_b[j - 1] {
                0
            } else {
                1
            };
            matrix[i][j] = (matrix[i - 1][j] + 1)
                .min(matrix[i][j - 1] + 1)
                .min(matrix[i - 1][j - 1] + cost);
        }
    }

    matrix[len_a][len_b]
}
