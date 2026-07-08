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

pub fn extract_table_name(sql: &str) -> Option<String> {
    let cleaned = clean_sql(sql);
    let mut trimmed = cleaned.trim();
    
    // Scan past Common Table Expressions (CTEs)
    if trimmed.to_uppercase().starts_with("WITH") {
        let chars_vec: Vec<char> = trimmed.chars().collect();
        let mut idx = 4; // skip 'WITH'
        
        // Skip RECURSIVE modifier
        let mut rest = trimmed[4..].trim_start();
        if rest.to_uppercase().starts_with("RECURSIVE") {
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
                                if word.to_uppercase() == "AS" {
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
 
    let sql_upper = trimmed.to_uppercase();
    let words: Vec<&str> = sql_upper.split_whitespace().collect();
    let raw_words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.is_empty() {
        return None;
    }
     
    match words[0] {
        "INSERT" => {
            let idx = words.iter().position(|&w| w == "INTO")?;
            if idx + 1 < raw_words.len() {
                let raw_name = raw_words[idx + 1].split('(').next().unwrap_or("");
                let name = raw_name.trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']' || c == '\'');
                return Some(name.to_lowercase());
            }
        }
        "UPDATE" => {
            let mut name_idx = 1;
            if name_idx < words.len() && words[name_idx] == "ONLY" {
                name_idx += 1;
            }
            if name_idx < raw_words.len() {
                let name = raw_words[name_idx].trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']');
                return Some(name.to_lowercase());
            }
        }
        "DELETE" => {
            let idx = words.iter().position(|&w| w == "FROM")?;
            if idx + 1 < raw_words.len() {
                let name = raw_words[idx + 1].trim_matches(|c| c == '`' || c == '"' || c == '[' || c == ']');
                return Some(name.to_lowercase());
            }
        }
        _ => {}
    }
    None
}

pub fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut chars = sql.chars().peekable();
    
    while let Some(c) = chars.next() {
        if in_single_quote {
            if c == '\'' {
                in_single_quote = false;
            }
            current.push(c);
        } else if in_double_quote {
            if c == '"' {
                in_double_quote = false;
            }
            current.push(c);
        } else {
            match c {
                '\'' => {
                    in_single_quote = true;
                    current.push(c);
                }
                '"' => {
                    in_double_quote = true;
                    current.push(c);
                }
                '-' if chars.peek() == Some(&'-') => {
                    current.push(c);
                    current.push(chars.next().unwrap()); // push second '-'
                    while let Some(nc) = chars.next() {
                        current.push(nc);
                        if nc == '\n' || nc == '\r' {
                            break;
                        }
                    }
                }
                '/' if chars.peek() == Some(&'*') => {
                    current.push(c);
                    current.push(chars.next().unwrap()); // push '*'
                    while let Some(nc) = chars.next() {
                        current.push(nc);
                        if nc == '*' && chars.peek() == Some(&'/') {
                            current.push(chars.next().unwrap()); // push '/'
                            break;
                        }
                    }
                }
                ';' => {
                    let trimmed = current.trim();
                    if !trimmed.is_empty() {
                        statements.push(trimmed.to_string());
                    }
                    current.clear();
                }
                _ => {
                    current.push(c);
                }
            }
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        statements.push(trimmed.to_string());
    }
    statements
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_sql_statements() {
        let sql = "INSERT INTO messages (body) VALUES ('hello; world'); UPDATE todos SET text = 'a;b'; DELETE FROM notes";
        let res = split_sql_statements(sql);
        assert_eq!(res.len(), 3);
        assert_eq!(res[0], "INSERT INTO messages (body) VALUES ('hello; world')");
        assert_eq!(res[1], "UPDATE todos SET text = 'a;b'");
        assert_eq!(res[2], "DELETE FROM notes");
    }

    #[test]
    fn test_split_sql_with_comments() {
        let sql = "INSERT INTO msg VALUES (1); -- inline comment with semicolon;\nUPDATE notes SET x = 1; /* block comment; */ DELETE FROM todos";
        let res = split_sql_statements(sql);
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
