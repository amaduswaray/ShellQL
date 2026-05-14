//! Regex-based SQL syntax highlighting for the query editor.

use once_cell::sync::Lazy;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use regex::Regex;

// ═══════════════════════════════════════════════════════════════════════════════
// Token types
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    Comment,
    String,
    Number,
    Keyword,
    Function,
    Operator,
    Default,
}

impl TokenType {
    fn style(self) -> Style {
        match self {
            // Muted blue-gray for comments
            TokenType::Comment => Style::default().fg(Color::Rgb(86, 95, 137)),
            // Soft green for strings
            TokenType::String => Style::default().fg(Color::Rgb(158, 206, 106)),
            // Soft orange for numbers
            TokenType::Number => Style::default().fg(Color::Rgb(255, 158, 100)),
            // Soft purple for keywords — no bold, just color
            TokenType::Keyword => Style::default().fg(Color::Rgb(187, 154, 247)),
            // Soft blue for functions
            TokenType::Function => Style::default().fg(Color::Rgb(122, 162, 247)),
            // Light cyan for operators
            TokenType::Operator => Style::default().fg(Color::Rgb(137, 221, 255)),
            // Soft white for everything else
            TokenType::Default => Style::default().fg(Color::Rgb(192, 202, 245)),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Regexes — compiled once, evaluated in priority order
// ═══════════════════════════════════════════════════════════════════════════════

static COMMENT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^--.*$").unwrap());
static STRING_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^'[^']*'").unwrap());
static NUMBER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+(\.\d+)?").unwrap());

static KEYWORD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^((?:SELECT|FROM|WHERE|INSERT|INTO|VALUES|UPDATE|SET|DELETE|JOIN|LEFT|RIGHT|INNER|OUTER|ON|CREATE|TABLE|DROP|ALTER|AND|OR|NOT|NULL|IS|AS|ORDER|BY|GROUP|HAVING|LIMIT|OFFSET|UNION|ALL|DISTINCT|CASE|WHEN|THEN|ELSE|END|IF|EXISTS|INDEX|VIEW|TRIGGER|PRIMARY|KEY|FOREIGN|REFERENCES|UNIQUE|CHECK|DEFAULT|AUTO_INCREMENT|SERIAL|BIGINT|INT|INTEGER|SMALLINT|TINYINT|VARCHAR|CHAR|TEXT|BOOLEAN|DATE|TIME|TIMESTAMP|DATETIME|FLOAT|DOUBLE|DECIMAL|NUMERIC|REAL|BLOB|JSON|ARRAY|BEGIN|COMMIT|ROLLBACK|TRANSACTION|GRANT|REVOKE|SHOW|DESCRIBE|EXPLAIN|ANALYZE|VACUUM|PRAGMA|WITH|RECURSIVE|WINDOW|OVER|PARTITION|RANGE|ROWS|PRECEDING|FOLLOWING|CURRENT|ROW|BETWEEN|IN|LIKE|ILIKE|SIMILAR|TO|ESCAPE|GLOB|REGEXP|MATCH|COLLATE|CAST|CONVERT|INTERVAL|EXTRACT|SUBSTRING|TRIM|POSITION|LENGTH|CHAR_LENGTH|OCTET_LENGTH|BIT_LENGTH|CURRENT_DATE|CURRENT_TIME|CURRENT_TIMESTAMP|LOCALTIME|LOCALTIMESTAMP|NOW|SESSION_USER|USER|CURRENT_USER|SYSTEM_USER))(?:[^a-zA-Z0-9_]|$)"
    ).unwrap()
});

static FUNCTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)^((?:COUNT|SUM|AVG|MAX|MIN|COALESCE|NULLIF|GREATEST|LEAST|ROUND|TRUNC|ABS|CEIL|FLOOR|MOD|POWER|SQRT|EXP|LN|LOG|SIN|COS|TAN|ASIN|ACOS|ATAN|ATAN2|RANDOM|UPPER|LOWER|INITCAP|LENGTH|CHAR_LENGTH|REPLACE|SUBSTRING|SUBSTR|STRPOS|POSITION|TRIM|LTRIM|RTRIM|LPAD|RPAD|REPEAT|REVERSE|TRANSLATE|OVERLAY|CONCAT|CONCAT_WS|FORMAT|TO_CHAR|TO_DATE|TO_TIMESTAMP|TO_NUMBER|EXTRACT|EPOCH|DATE_PART|DATE_TRUNC|AGE|OVERLAPS|INET|HOST|TEXT|INET_MERGE|INET_SAME_FAMILY|INET_CONTAINS|PG_SLEEP|GENERATE_SERIES|STRING_AGG|ARRAY_AGG|JSON_BUILD_OBJECT|JSONB_BUILD_OBJECT|JSON_AGG|JSONB_AGG|JSON_ARRAY|JSONB_ARRAY|JSON_OBJECT|JSONB_OBJECT|JSON_EXTRACT_PATH|JSONB_EXTRACT_PATH|JSON_EACH|JSONB_EACH|JSON_ARRAY_ELEMENTS|JSONB_ARRAY_ELEMENTS|JSON_TYPEOF|JSONB_TYPEOF|JSON_STRIP_NULLS|JSONB_STRIP_NULLS|ROW_TO_JSON|TO_JSON|TO_JSONB|ARRAY_TO_JSON|JSON_BUILD_ARRAY|JSONB_BUILD_ARRAY|SETSEED|WIDTH_BUCKET|CUME_DIST|DENSE_RANK|FIRST_VALUE|LAG|LAST_VALUE|LEAD|NTH_VALUE|NTILE|PERCENT_RANK|RANK|RATIO_TO_REPORT|ROW_NUMBER|CROSSTAB|UNNEST|GENERATE_SUBSCRIPTS|ARRAY_APPEND|ARRAY_CAT|ARRAY_DIMS|ARRAY_FILL|ARRAY_LENGTH|ARRAY_LOWER|ARRAY_POSITION|ARRAY_POSITIONS|ARRAY_PREPEND|ARRAY_REMOVE|ARRAY_REPLACE|ARRAY_TO_STRING|ARRAY_UPPER|CARDINALITY|ARRAYNDIMS|ARRAY_AGG|MODE|PERCENTILE_CONT|PERCENTILE_DISC|REGR_AVGX|REGR_AVGY|REGR_COUNT|REGR_INTERCEPT|REGR_R2|REGR_SLOPE|REGR_SXX|REGR_SXY|REGR_SYY|STDDEV|STDDEV_POP|STDDEV_SAMP|VAR_POP|VAR_SAMP|VARIANCE|BOOL_AND|BOOL_OR|EVERY|XMLAGG|XMLCOMMENT|XMLCONCAT|XMLELEMENT|XMLEXISTS|XMLFOREST|XMLPARSE|XMLPI|XMLROOT|XMLSERIALIZE|XMLTABLE))(?:[^a-zA-Z0-9_]|$)"
    ).unwrap()
});

static OPERATOR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[+\-*/=<>!]+|::|->|->>|#>>|#>").unwrap());

// ═══════════════════════════════════════════════════════════════════════════════
// Line tokenizer
// ═══════════════════════════════════════════════════════════════════════════════

/// Tokenize a single line of SQL into colored spans.
fn tokenize_line(line: &str) -> Vec<Span<'_>> {
    let mut spans = Vec::new();
    let mut pos = 0usize;

    while pos < line.len() {
        let slice = &line[pos..];

        // Try each regex in priority order.
        let (matched, ty) = if let Some(m) = COMMENT_RE.find(slice) {
            if m.start() == 0 {
                (m.end(), TokenType::Comment)
            } else {
                (0, TokenType::Default)
            }
        } else if let Some(m) = STRING_RE.find(slice) {
            if m.start() == 0 {
                (m.end(), TokenType::String)
            } else {
                (0, TokenType::Default)
            }
        } else if let Some(m) = NUMBER_RE.find(slice) {
            if m.start() == 0 {
                (m.end(), TokenType::Number)
            } else {
                (0, TokenType::Default)
            }
        } else if let Some(caps) = KEYWORD_RE.captures(slice) {
            if caps.get(0).unwrap().start() == 0 {
                let m = caps.get(1).unwrap();
                (m.end(), TokenType::Keyword)
            } else {
                (0, TokenType::Default)
            }
        } else if let Some(caps) = FUNCTION_RE.captures(slice) {
            if caps.get(0).unwrap().start() == 0 {
                let m = caps.get(1).unwrap();
                (m.end(), TokenType::Function)
            } else {
                (0, TokenType::Default)
            }
        } else if let Some(m) = OPERATOR_RE.find(slice) {
            if m.start() == 0 {
                (m.end(), TokenType::Operator)
            } else {
                (0, TokenType::Default)
            }
        } else {
            (0, TokenType::Default)
        };

        if ty == TokenType::Default && matched == 0 {
            // No regex matched at the current position — consume one char.
            let ch = slice.chars().next().unwrap();
            spans.push(Span::styled(ch.to_string(), TokenType::Default.style()));
            pos += ch.len_utf8();
        } else {
            spans.push(Span::styled(&line[pos..pos + matched], ty.style()));
            pos += matched;
        }
    }

    spans
}

// ═══════════════════════════════════════════════════════════════════════════════
// Public API
// ═══════════════════════════════════════════════════════════════════════════════

/// Convert a slice of SQL text lines into ratatui `Line`s with syntax highlighting.
pub fn highlight_sql_lines(lines: &[String]) -> Vec<Line<'_>> {
    lines
        .iter()
        .map(|line| Line::from(tokenize_line(line)))
        .collect()
}

/// Compute the visual (display) x position of a cursor at `(row, col)` within
/// the given lines.  Returns the number of display columns from the start of
/// the line up to (but not including) the character at `col`.
pub fn cursor_visual_x(line: &str, col: usize) -> usize {
    use unicode_width::UnicodeWidthChar;
    line.chars()
        .take(col)
        .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_matches_keyword() {
        let caps = KEYWORD_RE.captures("SELECT * FROM users");
        assert!(caps.is_some(), "KEYWORD_RE should match SELECT");
        assert_eq!(caps.unwrap().get(1).unwrap().as_str(), "SELECT");
    }

    #[test]
    fn test_regex_matches_lowercase_keyword() {
        let caps = KEYWORD_RE.captures("select * from users");
        assert!(caps.is_some(), "KEYWORD_RE should match lowercase select");
        assert_eq!(caps.unwrap().get(1).unwrap().as_str(), "select");
    }

    #[test]
    fn test_tokenize_line_produces_keyword_spans() {
        let spans = tokenize_line("SELECT * FROM users");
        let keyword_color = Color::Rgb(187, 154, 247);
        let op_color = Color::Rgb(137, 221, 255);
        // SELECT should be a keyword span (soft purple, no bold)
        assert!(spans.iter().any(|s| s.content == "SELECT" && s.style.fg == Some(keyword_color)),
            "SELECT should be highlighted as keyword");
        // FROM should be a keyword span
        assert!(spans.iter().any(|s| s.content == "FROM" && s.style.fg == Some(keyword_color)),
            "FROM should be highlighted as keyword");
        // * should be an operator span (light cyan)
        assert!(spans.iter().any(|s| s.content == "*" && s.style.fg == Some(op_color)),
            "* should be highlighted as operator");
    }

    #[test]
    fn test_string_highlighting() {
        let lines = vec!["SELECT * FROM users WHERE name = 'john'".to_string()];
        let highlighted = highlight_sql_lines(&lines);
        let spans = &highlighted[0].spans;
        let string_color = Color::Rgb(158, 206, 106);
        assert!(spans.iter().any(|s| s.content == "'john'" && s.style.fg == Some(string_color)));
    }
}
