#[derive(Debug, Clone, PartialEq)]
pub enum DiffType {
    Added,
    Removed,
    Unchanged,
}

pub struct DiffSegment {
    pub r#type: DiffType,
    pub text: String,
}

pub fn get_diff_segments(old_str: &str, new_str: &str) -> Vec<DiffSegment> {
    let old_words: Vec<String> = old_str.split_whitespace().map(|s| s.to_string()).collect();
    let new_words: Vec<String> = new_str.split_whitespace().map(|s| s.to_string()).collect();

    let mut matrix = vec![vec![0; new_words.len() + 1]; old_words.len() + 1];

    for i in 1..=old_words.len() {
        for j in 1..=new_words.len() {
            if old_words[i - 1] == new_words[j - 1] {
                matrix[i][j] = matrix[i - 1][j - 1] + 1;
            } else {
                matrix[i][j] = std::cmp::max(matrix[i - 1][j], matrix[i][j - 1]);
            }
        }
    }

    let mut segments = Vec::new();
    let mut i = old_words.len();
    let mut j = new_words.len();

    while i > 0 || j > 0 {
        if i > 0 && j > 0 && old_words[i - 1] == new_words[j - 1] {
            segments.push(DiffSegment {
                r#type: DiffType::Unchanged,
                text: format!("{} ", old_words[i - 1]),
            });
            i -= 1;
            j -= 1;
        } else if j > 0 && (i == 0 || matrix[i][j - 1] >= matrix[i - 1][j]) {
            segments.push(DiffSegment {
                r#type: DiffType::Added,
                text: format!("{} ", new_words[j - 1]),
            });
            j -= 1;
        } else if i > 0 && (j == 0 || matrix[i][j - 1] < matrix[i - 1][j]) {
            segments.push(DiffSegment {
                r#type: DiffType::Removed,
                text: format!("{} ", old_words[i - 1]),
            });
            i -= 1;
        }
    }

    segments.reverse();
    segments
}
