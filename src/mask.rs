//! Pure helpers for parsing and masking environment assignments.
//!
//! Every function here is free of I/O so it can be unit-tested in isolation.

use std::borrow::Cow;

/// Returns `true` when `c` may start an environment variable name.
fn is_ident_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

/// Returns `true` when `c` may continue an environment variable name.
fn is_ident_continue(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Parses a single line as an `NAME=VALUE` assignment.
///
/// Accepts an optional `export ` prefix and whitespace around the `=`.
/// The returned value is trimmed of surrounding whitespace and the
/// trailing newline. Returns `None` when the line is not an assignment.
pub fn parse_assignment(line: &str) -> Option<(&str, &str)> {
    let mut rest = line.trim_start();
    if let Some(after) = rest.strip_prefix("export") {
        let after = after.as_bytes();
        if !after.is_empty() && after[0].is_ascii_whitespace() {
            rest = rest["export".len()..].trim_start();
        }
    }

    let bytes = rest.as_bytes();
    let mut i = 0;
    if i >= bytes.len() || !is_ident_start(bytes[i]) {
        return None;
    }
    i += 1;
    while i < bytes.len() && is_ident_continue(bytes[i]) {
        i += 1;
    }
    let name = &rest[..i];

    let after_name = &rest[i..];
    let after_trim = after_name.trim_start();
    if !after_trim.starts_with('=') {
        return None;
    }
    let value = after_trim[1..].trim();
    Some((name, value))
}

/// Masks a single line. Assignment lines become `NAME=###`; every other
/// line is returned unchanged. The trailing newline is always preserved
/// as `\n` so streamed output matches the original tool.
pub fn mask_line(line: &str) -> Cow<'_, str> {
    match parse_assignment(line) {
        Some((name, _)) => Cow::Owned(format!("{name}=###\n")),
        None => Cow::Borrowed(line),
    }
}

/// Extracts every `NAME=VALUE` pair from free-form text such as
/// `"MINHA_VAR=123 VAR2 = 2"`. Values end at the first whitespace.
pub fn extract_pairs(text: &str) -> Vec<(&str, &str)> {
    let bytes = text.as_bytes();
    let mut pairs = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        if !is_ident_start(bytes[i]) {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        while i < bytes.len() && is_ident_continue(bytes[i]) {
            i += 1;
        }
        let name = &text[start..i];

        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'=' {
            i = i.max(start + 1);
            continue;
        }
        j += 1;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        let value_start = j;
        while j < bytes.len() && !bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        let value = &text[value_start..j];
        pairs.push((name, value));
        i = j;
    }

    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_assignment() {
        assert_eq!(parse_assignment("KEY=value"), Some(("KEY", "value")));
    }

    #[test]
    fn parses_export_prefix() {
        assert_eq!(parse_assignment("export KEY=value"), Some(("KEY", "value")));
    }

    #[test]
    fn parses_spaces_around_equals() {
        assert_eq!(parse_assignment("KEY = value"), Some(("KEY", "value")));
    }

    #[test]
    fn parses_leading_whitespace() {
        assert_eq!(parse_assignment("   KEY=value"), Some(("KEY", "value")));
    }

    #[test]
    fn trims_value_and_newline() {
        assert_eq!(parse_assignment("KEY= value \n"), Some(("KEY", "value")));
    }

    #[test]
    fn rejects_non_assignment_lines() {
        assert_eq!(parse_assignment("# comment"), None);
        assert_eq!(parse_assignment("just some text"), None);
        assert_eq!(parse_assignment(""), None);
    }

    #[test]
    fn rejects_identifier_without_equals() {
        assert_eq!(parse_assignment("KEY only"), None);
    }

    #[test]
    fn allows_empty_value() {
        assert_eq!(parse_assignment("KEY="), Some(("KEY", "")));
    }

    #[test]
    fn keeps_underscore_and_digits_in_name() {
        assert_eq!(parse_assignment("A_B2=1"), Some(("A_B2", "1")));
    }

    #[test]
    fn masks_assignment_line() {
        assert_eq!(mask_line("KEY=secret\n"), "KEY=###\n");
    }

    #[test]
    fn masks_without_newline() {
        assert_eq!(mask_line("KEY=secret"), "KEY=###\n");
    }

    #[test]
    fn passes_non_assignment_through() {
        assert_eq!(mask_line("# comment\n"), "# comment\n");
    }

    #[test]
    fn extracts_pairs_from_joined_args() {
        let pairs = extract_pairs("MINHA_VAR=123 VAR2 = 2");
        assert_eq!(pairs, vec![("MINHA_VAR", "123"), ("VAR2", "2")]);
    }

    #[test]
    fn extracts_no_pairs_from_plain_text() {
        assert!(extract_pairs("hello world").is_empty());
    }
}
