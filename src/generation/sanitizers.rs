//! Language-agnostic sanitizer functions for OpenAPI code generation
//!
//! This module provides utilities to sanitize various strings used in code generation
//! to ensure they are valid for their intended use across all target languages.

use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashSet;

lazy_static! {
    /// A set of all Rust 2018 keywords.
    ///
    /// This list includes keywords that cannot be used as identifiers without
    /// a raw identifier prefix (`r#`).
    static ref RUST_KEYWORDS: HashSet<&'static str> = {
        let keywords = vec![
            // Keywords that are currently in use
            "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false",
            "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
            "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
            "true", "type", "union", "unsafe", "use", "where", "while",
            // Keywords reserved for future use
            "async", "await", "dyn",
            // Weak keywords (contextual) - generally safe as identifiers but good to escape for consistency
            // "union" is already in the main list
            // "static" is already in the main list
            // "dyn" is already in the main list
            // Keywords that are not yet in use but are reserved
            "abstract", "become", "box", "do", "final", "macro", "override", "priv",
            "typeof", "unsized", "virtual", "yield", "try",
        ];
        keywords.into_iter().collect()
    };
}

/// Sanitizes Markdown for use in code documentation across all languages
///
/// This function:
/// - Replaces smart quotes with regular quotes
/// - Replaces em-dashes with regular dashes
/// - Escapes backslashes and quotes for string literals
/// - Escapes braces and brackets to prevent documentation parsing issues
/// - Collapses whitespace and removes empty lines
///
/// # Examples
/// ```
/// use agenterra::generation::sanitizers::sanitize_markdown;
///
/// let input = "This is a "smart quote" example—with em-dash";
/// let output = sanitize_markdown(input);
/// assert!(output.contains("\\\"smart quote\\\""));
/// assert!(output.contains("-with em-dash"));
/// ```
pub fn sanitize_markdown(input: &str) -> String {
    // Regex for problematic Unicode (e.g., smart quotes, em-dash)
    let unicode_re = Regex::new(r"[\u{2018}\u{2019}\u{201C}\u{201D}\u{2014}]").unwrap();
    // Regex to collapse any whitespace sequence into a single space
    let ws_re = Regex::new(r"\s+").unwrap();

    input
        .lines()
        .map(|line| {
            let mut line = line.replace('\t', " ");
            // Remove problematic Unicode
            line = unicode_re
                .replace_all(&line, |caps: &regex::Captures| match &caps[0] {
                    "\u{2018}" | "\u{2019}" => "'",
                    "\u{201C}" | "\u{201D}" => "\"",
                    "\u{2014}" => "-",
                    _ => "",
                })
                .to_string();
            // Trim edges and collapse inner whitespace
            let mut trimmed = ws_re.replace_all(line.trim(), " ").to_string();
            // Remove spaces around hyphens
            trimmed = trimmed
                .replace(" - ", "-")
                .replace("- ", "-")
                .replace(" -", "-");
            // Escape backslashes and quotes
            let mut safe = trimmed.replace('\\', "\\\\").replace('"', "\\\"");
            // Escape braces and brackets
            safe = safe
                .replace("{", "&#123;")
                .replace("}", "&#125;")
                .replace("[", "&#91;")
                .replace("]", "&#93;");
            safe
        })
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Sanitizes a string to be a valid Rust identifier, escaping keywords if necessary.
///
/// If the input string is a Rust keyword, behavior depends on context:
/// - In code context: the keyword is prefixed with `r#` to create a raw identifier
/// - In string context: the keyword is returned as-is for use in strings, JSON, etc.
///
/// # Arguments
/// * `name` - The string to sanitize.
/// * `is_string_context` - If true, preserves keywords as-is for string usage.
///                         If false, escapes keywords for Rust code usage.
///
/// # Returns
/// A `String` that is suitable for the specified context.
///
/// # Examples
/// ```
/// use agenterra::generation::sanitizers::sanitize_rust_identifier;
///
/// // In code context, keywords are escaped
/// assert_eq!(sanitize_rust_identifier("type", false), "r#type");
/// assert_eq!(sanitize_rust_identifier("name", false), "name");
/// assert_eq!(sanitize_rust_identifier("for", false), "r#for");
///
/// // In string context, keywords are preserved
/// assert_eq!(sanitize_rust_identifier("type", true), "type");
/// assert_eq!(sanitize_rust_identifier("name", true), "name");
/// assert_eq!(sanitize_rust_identifier("for", true), "for");
/// ```
pub fn sanitize_rust_identifier(name: &str, is_string_context: bool) -> String {
    if is_string_context {
        // In string contexts (JSON schemas, query params, etc.), preserve keywords as-is
        name.to_string()
    } else {
        // In code contexts, escape Rust keywords with raw identifier prefix
        if RUST_KEYWORDS.contains(name) {
            format!("r#{}", name)
        } else {
            name.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_markdown() {
        // Test smart quotes
        let input = "This is a \"smart quote\" example";
        let output = sanitize_markdown(input);
        assert_eq!(output, "This is a \\\"smart quote\\\" example");

        // Test em-dash
        let input = "This—is an em-dash";
        let output = sanitize_markdown(input);
        assert_eq!(output, "This-is an em-dash");

        // Test multiple lines and whitespace
        let input = "Line one\n\nLine two\n   \nLine three";
        let output = sanitize_markdown(input);
        assert_eq!(output, "Line one Line two Line three");

        // Test escaping
        let input = "This has {braces} and [brackets]";
        let output = sanitize_markdown(input);
        assert_eq!(output, "This has &#123;braces&#125; and &#91;brackets&#93;");

        // Test backslashes
        let input = "Path\\to\\file";
        let output = sanitize_markdown(input);
        assert_eq!(output, "Path\\\\to\\\\file");
    }

    #[test]
    fn test_sanitize_rust_identifier() {
        // Test code context with Rust keywords (should be escaped)
        assert_eq!(sanitize_rust_identifier("type", false), "r#type");
        assert_eq!(sanitize_rust_identifier("for", false), "r#for");
        assert_eq!(sanitize_rust_identifier("fn", false), "r#fn");
        assert_eq!(sanitize_rust_identifier("async", false), "r#async");
        assert_eq!(sanitize_rust_identifier("union", false), "r#union");

        // Test code context with non-keywords (should be unchanged)
        assert_eq!(sanitize_rust_identifier("name", false), "name");
        assert_eq!(sanitize_rust_identifier("id", false), "id");
        assert_eq!(sanitize_rust_identifier("description", false), "description");
        assert_eq!(sanitize_rust_identifier("my_variable", false), "my_variable");

        // Test string context with keywords (should be preserved)
        assert_eq!(sanitize_rust_identifier("type", true), "type");
        assert_eq!(sanitize_rust_identifier("for", true), "for");
        assert_eq!(sanitize_rust_identifier("fn", true), "fn");
        assert_eq!(sanitize_rust_identifier("async", true), "async");
        assert_eq!(sanitize_rust_identifier("union", true), "union");

        // Test string context with non-keywords (should be unchanged)
        assert_eq!(sanitize_rust_identifier("name", true), "name");
        assert_eq!(sanitize_rust_identifier("id", true), "id");
        assert_eq!(sanitize_rust_identifier("description", true), "description");
        assert_eq!(sanitize_rust_identifier("my_variable", true), "my_variable");
    }
}
