/// Cleans comments from SQL and respects string quotes to prevent false positive parsing.
pub fn clean_sql(sql: &str) -> String {
    let mut cleaned = String::new();
    let mut chars = sql.chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(c) = chars.next() {
        if in_single_quote {
            if c == '\'' {
                in_single_quote = false;
            }
            cleaned.push(c);
        } else if in_double_quote {
            if c == '"' {
                in_double_quote = false;
            }
            cleaned.push(c);
        } else {
            match c {
                '\'' => {
                    in_single_quote = true;
                    cleaned.push(c);
                }
                '"' => {
                    in_double_quote = true;
                    cleaned.push(c);
                }
                '-' if chars.peek() == Some(&'-') => {
                    // Consume line comment
                    while let Some(nc) = chars.next() {
                        if nc == '\n' || nc == '\r' {
                            cleaned.push(' ');
                            break;
                        }
                    }
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next(); // consume '*'
                    // Consume block comment
                    while let Some(nc) = chars.next() {
                        if nc == '*' && chars.peek() == Some(&'/') {
                            chars.next(); // consume '/'
                            cleaned.push(' ');
                            break;
                        }
                    }
                }
                _ => {
                    cleaned.push(c);
                }
            }
        }
    }
    cleaned
}

fn has_write_keyword(sql: &str) -> bool {
    let bytes = sql.as_bytes();
    let needles: &[&[u8]] = &[b"INSERT", b"UPDATE", b"DELETE"];
    for needle in needles {
        if bytes.windows(needle.len()).any(|window| {
            window.iter().zip(*needle).all(|(&h, &n)| {
                h.to_ascii_uppercase() == n
            })
        }) {
            return true;
        }
    }
    false
}

pub fn extract_table_name(sql: &str) -> Option<String> {
    // Pre-screen to avoid any allocations/parsing for read-only queries (SELECT, etc.)
    if !has_write_keyword(sql) {
        return None;
    }

    // Fast path: if the query is simple and contains no comments or CTEs, parse directly.
    let trimmed = sql.trim_start();
    let is_simple = if trimmed.len() >= 6 {
        let prefix = &trimmed[..6];
        prefix.eq_ignore_ascii_case("INSERT") || prefix.eq_ignore_ascii_case("UPDATE") || prefix.eq_ignore_ascii_case("DELETE")
    } else {
        false
    };

    if is_simple && !sql.contains("/*") && !sql.contains("--") {
        let mut words = trimmed.split_whitespace();
        if let Some(first) = words.next() {
            if first.eq_ignore_ascii_case("INSERT") {
                while let Some(w) = words.next() {
                    if w.eq_ignore_ascii_case("INTO") {
                        if let Some(target) = words.next() {
                            let raw_name = target.split('(').next().unwrap_or("");
                            let name = raw_name.trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']' || c == '\'');
                            return Some(name.to_lowercase());
                        }
                        break;
                    }
                }
            } else if first.eq_ignore_ascii_case("UPDATE") {
                let target_opt = words.next();
                if let Some(target) = target_opt {
                    let mut final_target = target;
                    if target.eq_ignore_ascii_case("ONLY") {
                        if let Some(next_target) = words.next() {
                            final_target = next_target;
                        } else {
                            return None;
                        }
                    }
                    let name = final_target.trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']' || c == '\'');
                    return Some(name.to_lowercase());
                }
            } else if first.eq_ignore_ascii_case("DELETE") {
                while let Some(w) = words.next() {
                    if w.eq_ignore_ascii_case("FROM") {
                        if let Some(target) = words.next() {
                            let name = target.trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']' || c == '\'');
                            return Some(name.to_lowercase());
                        }
                        break;
                    }
                }
            }
        }
    }

    // Lazy comment cleaning: only call clean_sql if comments actually exist
    let cleaned_owned;
    let has_comments = sql.contains("/*") || sql.contains("--");
    if has_comments {
        cleaned_owned = clean_sql(sql);
    } else {
        cleaned_owned = String::new();
    }
    let mut trimmed = if has_comments {
        cleaned_owned.trim()
    } else {
        sql.trim()
    };
    
    // Scan past Common Table Expressions (CTEs)
    if trimmed.len() >= 4 && trimmed[..4].eq_ignore_ascii_case("WITH") {
        let chars_vec: Vec<char> = trimmed.chars().collect();
        let mut idx = 4; // skip 'WITH'
        
        // Skip RECURSIVE modifier
        let mut rest = trimmed[4..].trim_start();
        if rest.len() >= 9 && rest[..9].eq_ignore_ascii_case("RECURSIVE") {
            rest = rest[9..].trim_start();
            idx = trimmed.len() - rest.len();
        }
        
        // Loop to skip each CTE definition
        loop {
            // Find the top-level "AS" for the current CTE
            let mut paren_count = 0;
            let mut in_single_quote = false;
            let mut in_double_quote = false;
            let mut found_as_idx = None;
            
            while idx < chars_vec.len() {
                let c = chars_vec[idx];
                if in_single_quote {
                    if c == '\'' {
                        in_single_quote = false;
                    }
                } else if in_double_quote {
                    if c == '"' {
                        in_double_quote = false;
                    }
                } else {
                    match c {
                        '\'' => in_single_quote = true,
                        '"' => in_double_quote = true,
                        '(' => paren_count += 1,
                        ')' => {
                            if paren_count > 0 {
                                paren_count -= 1;
                            }
                        }
                        _ if paren_count == 0 => {
                            // Check if this starts "AS"
                            if idx + 2 <= chars_vec.len() {
                                let word: String = chars_vec[idx..idx+2].iter().collect();
                                if word.eq_ignore_ascii_case("AS") {
                                    // Check word boundaries
                                    let prev_ok = idx == 0 || chars_vec[idx-1].is_whitespace() || chars_vec[idx-1] == ')' || chars_vec[idx-1] == ']';
                                    let next_ok = idx + 2 == chars_vec.len() || chars_vec[idx+2].is_whitespace() || chars_vec[idx+2] == '(';
                                    if prev_ok && next_ok {
                                        found_as_idx = Some(idx);
                                        idx += 2;
                                        break;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                idx += 1;
            }
            
            let _ = match found_as_idx {
                Some(i) => i,
                None => break, // invalid CTE syntax, break out
            };
            
            // Skip whitespace to the opening parenthesis '(' of the CTE query
            while idx < chars_vec.len() && chars_vec[idx].is_whitespace() {
                idx += 1;
            }
            if idx >= chars_vec.len() || chars_vec[idx] != '(' {
                break; // invalid CTE syntax
            }
            
            // balance parentheses of the CTE query (starts at paren_count = 1)
            let mut cte_paren_count = 1;
            idx += 1; // skip '('
            in_single_quote = false;
            in_double_quote = false;
            
            while idx < chars_vec.len() {
                let c = chars_vec[idx];
                if in_single_quote {
                    if c == '\'' {
                        in_single_quote = false;
                    }
                } else if in_double_quote {
                    if c == '"' {
                        in_double_quote = false;
                    }
                } else {
                    match c {
                        '\'' => in_single_quote = true,
                        '"' => in_double_quote = true,
                        '(' => cte_paren_count += 1,
                        ')' => {
                            if cte_paren_count > 0 {
                                cte_paren_count -= 1;
                                if cte_paren_count == 0 {
                                    break;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                idx += 1;
            }
            
            if cte_paren_count != 0 {
                break; // unbalanced parentheses, break out
            }
            
            idx += 1; // skip ')'
            
            // Peek at next non-whitespace char
            while idx < chars_vec.len() && chars_vec[idx].is_whitespace() {
                idx += 1;
            }
            
            if idx < chars_vec.len() && chars_vec[idx] == ',' {
                idx += 1; // skip comma and loop to parse next CTE
            } else {
                // Done with CTEs!
                trimmed = &trimmed[idx..];
                break;
            }
        }
    }
 
    let mut words = trimmed.split_whitespace();
    if let Some(first) = words.next() {
        if first.eq_ignore_ascii_case("INSERT") {
            while let Some(w) = words.next() {
                if w.eq_ignore_ascii_case("INTO") {
                    if let Some(target) = words.next() {
                        let raw_name = target.split('(').next().unwrap_or("");
                        let name = raw_name.trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']' || c == '\'');
                        return Some(name.to_lowercase());
                    }
                    break;
                }
            }
        } else if first.eq_ignore_ascii_case("UPDATE") {
            let target_opt = words.next();
            if let Some(target) = target_opt {
                let mut final_target = target;
                if target.eq_ignore_ascii_case("ONLY") {
                    if let Some(next_target) = words.next() {
                        final_target = next_target;
                    } else {
                        return None;
                    }
                }
                let name = final_target.trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']');
                return Some(name.to_lowercase());
            }
        } else if first.eq_ignore_ascii_case("DELETE") {
            while let Some(w) = words.next() {
                if w.eq_ignore_ascii_case("FROM") {
                    if let Some(target) = words.next() {
                        let name = target.trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']');
                        return Some(name.to_lowercase());
                    }
                    break;
                }
            }
        }
    }
    None
}

pub struct SqlStatementSplitter<'a> {
    sql: &'a str,
    char_indices: std::iter::Peekable<std::str::CharIndices<'a>>,
    start_idx: usize,
    in_single_quote: bool,
    in_double_quote: bool,
}

impl<'a> SqlStatementSplitter<'a> {
    pub fn new(sql: &'a str) -> Self {
        Self {
            sql,
            char_indices: sql.char_indices().peekable(),
            start_idx: 0,
            in_single_quote: false,
            in_double_quote: false,
        }
    }
}

impl<'a> Iterator for SqlStatementSplitter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((idx, c)) = self.char_indices.next() {
            if self.in_single_quote {
                if c == '\'' {
                    self.in_single_quote = false;
                }
            } else if self.in_double_quote {
                if c == '"' {
                    self.in_double_quote = false;
                }
            } else {
                match c {
                    '\'' => {
                        self.in_single_quote = true;
                    }
                    '"' => {
                        self.in_double_quote = true;
                    }
                    '-' if self.char_indices.peek().map(|&(_, nc)| nc) == Some('-') => {
                        self.char_indices.next(); // consume second '-'
                        while let Some((_, nc)) = self.char_indices.next() {
                            if nc == '\n' || nc == '\r' {
                                break;
                            }
                        }
                    }
                    '/' if self.char_indices.peek().map(|&(_, nc)| nc) == Some('*') => {
                        self.char_indices.next(); // consume '*'
                        while let Some((_, nc)) = self.char_indices.next() {
                            if nc == '*' && self.char_indices.peek().map(|&(_, nnc)| nnc) == Some('/') {
                                self.char_indices.next(); // consume '/'
                                break;
                            }
                        }
                    }
                    ';' => {
                        let stmt = &self.sql[self.start_idx..idx];
                        let trimmed = stmt.trim();
                        self.start_idx = self.char_indices.peek().map(|&(next_idx, _)| next_idx).unwrap_or(self.sql.len());
                        if !trimmed.is_empty() {
                            return Some(trimmed);
                        }
                    }
                    _ => {}
                }
            }
        }
        
        if self.start_idx < self.sql.len() {
            let stmt = &self.sql[self.start_idx..];
            self.start_idx = self.sql.len();
            let trimmed = stmt.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
        
        None
    }
}

pub fn split_sql_statements(sql: &str) -> SqlStatementSplitter<'_> {
    SqlStatementSplitter::new(sql)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sql_statements() {
        let sql = "INSERT INTO messages (body) VALUES ('hello; world'); UPDATE todos SET text = 'a;b'; DELETE FROM notes";
        let res: Vec<&str> = split_sql_statements(sql).collect();
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], "INSERT INTO messages (body) VALUES ('hello; world')");
        assert_eq!(res[1], "UPDATE todos SET text = 'a;b'");
        assert_eq!(res[2], "DELETE FROM notes");
    }

    #[test]
    fn test_split_sql_with_comments() {
        let sql = "INSERT INTO msg VALUES (1); -- inline comment with semicolon;\nUPDATE notes SET x = 1; /* block comment; */ DELETE FROM todos";
        let res: Vec<&str> = split_sql_statements(sql).collect();
        assert_eq!(res.len(), 3);
        assert!(res[1].contains("inline comment"));
        assert!(res[2].contains("block comment"));
    }

    #[test]
    fn test_extract_table_name_inserts() {
        assert_eq!(extract_table_name("INSERT INTO todos (id, text) VALUES (1, 'hello')"), Some("todos".to_string()));
        assert_eq!(extract_table_name("INSERT INTO [todos] (id) VALUES (1)"), Some("todos".to_string()));
        assert_eq!(extract_table_name("INSERT INTO `todos` VALUES (1)"), Some("todos".to_string()));
        assert_eq!(extract_table_name("  INSERT   INTO   \"todos\" ..."), Some("todos".to_string()));
    }

    #[test]
    fn test_extract_table_name_updates() {
        assert_eq!(extract_table_name("UPDATE users SET name = 'Alice'"), Some("users".to_string()));
        assert_eq!(extract_table_name("UPDATE [users] SET x = 1"), Some("users".to_string()));
        assert_eq!(extract_table_name("UPDATE `users` SET x = 1"), Some("users".to_string()));
    }

    #[test]
    fn test_extract_table_name_deletes() {
        assert_eq!(extract_table_name("DELETE FROM messages WHERE id = 1"), Some("messages".to_string()));
        assert_eq!(extract_table_name("DELETE FROM [messages]"), Some("messages".to_string()));
    }

    #[test]
    fn test_extract_table_name_ctes() {
        let sql_cte_update = "WITH cte AS (SELECT id FROM users WHERE age > 10) UPDATE profiles SET status = 1 WHERE user_id IN (SELECT id FROM cte)";
        assert_eq!(extract_table_name(sql_cte_update), Some("profiles".to_string()));

        let sql_cte_insert = "WITH RECURSIVE temp_ids(n) AS (VALUES(1) UNION ALL SELECT n+1 FROM temp_ids WHERE n<5) INSERT INTO logs (val) SELECT n FROM temp_ids";
        assert_eq!(extract_table_name(sql_cte_insert), Some("logs".to_string()));
    }

    #[test]
    fn test_extract_table_name_comments() {
        let sql_comment_update = "-- inline comment here\nUPDATE users SET active = 1";
        assert_eq!(extract_table_name(sql_comment_update), Some("users".to_string()));

        let sql_block_comment = "/* block comment */ INSERT INTO notes VALUES (1)";
        assert_eq!(extract_table_name(sql_block_comment), Some("notes".to_string()));
    }

    #[test]
    fn test_extract_table_name_invalid_or_select() {
        assert_eq!(extract_table_name("SELECT * FROM todos"), None);
        assert_eq!(extract_table_name("INSERT INTO"), None);
        assert_eq!(extract_table_name("UPDATE"), None);
        assert_eq!(extract_table_name("DELETE FROM"), None);
        assert_eq!(extract_table_name(""), None);
    }
}
