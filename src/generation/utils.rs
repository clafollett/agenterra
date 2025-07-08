//! String transformation utilities for code generation
//!
//! These utilities belong in the generation domain as they are used
//! for transforming identifiers during code generation.

/// Converts a string to snake_case format for Rust identifiers.
///
/// This function handles various input formats including camelCase, PascalCase,
/// kebab-case, and space-separated strings, converting them all to snake_case.
///
/// # Arguments
/// * `s` - The input string to convert
///
/// # Returns
/// A new String in snake_case format
///
/// # Examples
/// ```
/// use generation::utils::to_snake_case;
///
/// assert_eq!(to_snake_case("findPetsByStatus"), "find_pets_by_status");
/// assert_eq!(to_snake_case("FindPetsByStatus"), "find_pets_by_status");
/// assert_eq!(to_snake_case("find-pets-by-status"), "find_pets_by_status");
/// assert_eq!(to_snake_case("get HTTP Response"), "get_http_response");
/// ```
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_is_lowercase = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            // Add underscore before uppercase letter if:
            // - Not at the start
            // - Previous character was lowercase
            if i > 0 && prev_is_lowercase {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap());
            prev_is_lowercase = false;
        } else if ch.is_alphanumeric() {
            result.push(ch);
            prev_is_lowercase = ch.is_lowercase();
        } else if ch == '-' || ch == '_' || ch == ' ' {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
            prev_is_lowercase = false;
        }
    }

    // Remove duplicate underscores and trim
    let mut final_result = String::new();
    let mut prev_underscore = false;
    for ch in result.chars() {
        if ch == '_' {
            if !prev_underscore && !final_result.is_empty() {
                final_result.push(ch);
            }
            prev_underscore = true;
        } else {
            final_result.push(ch);
            prev_underscore = false;
        }
    }

    final_result.trim_matches('_').to_string()
}

/// Converts a string to UpperCamelCase (PascalCase) format for Rust type names.
///
/// This function normalizes the input through snake_case conversion first,
/// then capitalizes each word to create proper PascalCase identifiers.
///
/// # Arguments
/// * `s` - The input string to convert
///
/// # Returns
/// A new String in UpperCamelCase format
///
/// # Examples
/// ```
/// use generation::utils::to_proper_case;
///
/// assert_eq!(to_proper_case("find_pets_by_status"), "FindPetsByStatus");
/// assert_eq!(to_proper_case("http_response"), "HttpResponse");
/// assert_eq!(to_proper_case("find-pets-by-status"), "FindPetsByStatus");
/// ```
pub fn to_proper_case(s: &str) -> String {
    // First convert to snake_case to normalize the input
    let snake = to_snake_case(s);

    // Then split on underscores and capitalize each word
    snake
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}

/// Converts a string to camelCase format for JavaScript/TypeScript identifiers.
///
/// This function normalizes the input through snake_case conversion first,
/// then capitalizes each word except the first to create proper camelCase identifiers.
///
/// # Arguments
/// * `s` - The input string to convert
///
/// # Returns
/// A new String in camelCase format
///
/// # Examples
/// ```
/// use generation::utils::to_camel_case;
///
/// assert_eq!(to_camel_case("find_pets_by_status"), "findPetsByStatus");
/// assert_eq!(to_camel_case("http_response"), "httpResponse");
/// assert_eq!(to_camel_case("find-pets-by-status"), "findPetsByStatus");
/// ```
pub fn to_camel_case(s: &str) -> String {
    let pascal = to_proper_case(s);
    if pascal.is_empty() {
        return pascal;
    }

    let mut chars = pascal.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

/// Sanitizes a string to be a valid Rust field name.
///
/// This function handles Rust reserved keywords by appending an underscore.
/// It also converts the string to snake_case for consistency.
///
/// # Arguments
/// * `s` - The input string to sanitize
///
/// # Returns
/// A new String that is a valid Rust field name
///
/// # Examples
/// ```
/// use generation::utils::sanitize_rust_field_name;
///
/// assert_eq!(sanitize_rust_field_name("type"), "type_");
/// assert_eq!(sanitize_rust_field_name("self"), "self_");
/// assert_eq!(sanitize_rust_field_name("firstName"), "first_name");
/// ```
pub fn sanitize_rust_field_name(s: &str) -> String {
    let snake_case = to_snake_case(s);

    // List of Rust reserved keywords
    match snake_case.as_str() {
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern" | "false"
        | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match" | "mod" | "move"
        | "mut" | "pub" | "ref" | "return" | "self" | "Self" | "static" | "struct" | "super"
        | "trait" | "true" | "type" | "unsafe" | "use" | "where" | "while" | "async" | "await"
        | "dyn" | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override"
        | "priv" | "typeof" | "unsized" | "virtual" | "yield" | "try" => format!("{snake_case}_"),
        _ => snake_case,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("findPetsByStatus"), "find_pets_by_status");
        assert_eq!(to_snake_case("FindPetsByStatus"), "find_pets_by_status");
        assert_eq!(to_snake_case("find-pets-by-status"), "find_pets_by_status");
        assert_eq!(to_snake_case("find_pets_by_status"), "find_pets_by_status");
        assert_eq!(to_snake_case("HTTPResponse"), "httpresponse");
        assert_eq!(to_snake_case("getHTTPResponse"), "get_httpresponse");
        assert_eq!(to_snake_case("get HTTP Response"), "get_http_response");
    }

    #[test]
    fn test_to_proper_case() {
        assert_eq!(to_proper_case("find_pets_by_status"), "FindPetsByStatus");
        assert_eq!(to_proper_case("findPetsByStatus"), "FindPetsByStatus");
        assert_eq!(to_proper_case("find-pets-by-status"), "FindPetsByStatus");
        assert_eq!(to_proper_case("FIND_PETS_BY_STATUS"), "FindPetsByStatus");
        assert_eq!(to_proper_case("http_response"), "HttpResponse");
    }

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("find_pets_by_status"), "findPetsByStatus");
        assert_eq!(to_camel_case("FindPetsByStatus"), "findPetsByStatus");
        assert_eq!(to_camel_case("find-pets-by-status"), "findPetsByStatus");
        assert_eq!(to_camel_case("http_response"), "httpResponse");
        assert_eq!(to_camel_case("get_http_response"), "getHttpResponse");
    }

    #[test]
    fn test_sanitize_rust_field_name() {
        // Test reserved keywords
        assert_eq!(sanitize_rust_field_name("type"), "type_");
        assert_eq!(sanitize_rust_field_name("self"), "self_");
        assert_eq!(sanitize_rust_field_name("match"), "match_");
        assert_eq!(sanitize_rust_field_name("async"), "async_");

        // Test normal field names
        assert_eq!(sanitize_rust_field_name("firstName"), "first_name");
        assert_eq!(sanitize_rust_field_name("user_id"), "user_id");
        assert_eq!(sanitize_rust_field_name("HTTPResponse"), "httpresponse");

        // Test that already snake_case keywords still get underscore
        assert_eq!(sanitize_rust_field_name("for"), "for_");
    }
}
