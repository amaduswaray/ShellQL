use once_cell::sync::Lazy;
use regex::Regex;

/// Regex that finds unquoted identifiers immediately after SQL keywords that
/// reference tables.
static TABLE_REF_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(FROM|JOIN|INTO|UPDATE|TABLE)\s+([a-zA-Z_][a-zA-Z0-9_]*)\b").unwrap()
});

/// For every unquoted identifier after `FROM` / `JOIN` / `INTO` / `UPDATE` /
/// `TABLE`, if the identifier matches a known table name case-insensitively,
/// replace it with the properly-quoted exact-case name.
///
/// This lets users write `SELECT * FROM users` even when the Postgres catalog
/// stores the table as `Users` (mixed-case identifiers are case-sensitive in
/// Postgres only when quoted).
pub fn normalize_sql_table_names(sql: &str, tables: &[String]) -> String {
    TABLE_REF_RE
        .replace_all(sql, |caps: &regex::Captures| {
            let keyword = caps.get(1).unwrap().as_str();
            let identifier = caps.get(2).unwrap().as_str();
            if let Some(table) = tables
                .iter()
                .find(|t| t.to_lowercase() == identifier.to_lowercase())
            {
                format!("{} \"{}\"", keyword, table)
            } else {
                caps.get(0).unwrap().as_str().to_string()
            }
        })
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_from() {
        let tables = vec!["Users".to_string(), "Orders".to_string()];
        assert_eq!(
            normalize_sql_table_names("SELECT * FROM users", &tables),
            "SELECT * FROM \"Users\""
        );
    }

    #[test]
    fn test_normalize_join() {
        let tables = vec!["Users".to_string(), "Orders".to_string()];
        assert_eq!(
            normalize_sql_table_names("SELECT * FROM users JOIN orders", &tables),
            "SELECT * FROM \"Users\" JOIN \"Orders\""
        );
    }

    #[test]
    fn test_no_normalize_unknown_table() {
        let tables = vec!["Users".to_string()];
        assert_eq!(
            normalize_sql_table_names("SELECT * FROM products", &tables),
            "SELECT * FROM products"
        );
    }

    #[test]
    fn test_no_normalize_already_quoted() {
        let tables = vec!["Users".to_string()];
        assert_eq!(
            normalize_sql_table_names(r#"SELECT * FROM "Users""#, &tables),
            r#"SELECT * FROM "Users""#
        );
    }

    #[test]
    fn test_normalize_update() {
        let tables = vec!["Users".to_string()];
        assert_eq!(
            normalize_sql_table_names("UPDATE users SET name = 'x'", &tables),
            "UPDATE \"Users\" SET name = 'x'"
        );
    }
}
