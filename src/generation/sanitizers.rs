//! Language-agnostic sanitizer functions for OpenAPI code generation
//!
//! This module provides utilities to sanitize various strings used in code generation
//! to ensure they are valid for their intended use across all target languages.

use regex::Regex;

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
}
