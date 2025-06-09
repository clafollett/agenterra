//! String transformation utilities for code generation

/// Convert a string to snake_case
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

/// Convert a string to UpperCamelCase (PascalCase)
pub fn to_upper_camel_case(s: &str) -> String {
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

/// Convert a string to lowerCamelCase
pub fn to_lower_camel_case(s: &str) -> String {
    let upper_camel = to_upper_camel_case(s);
    if upper_camel.is_empty() {
        return upper_camel;
    }

    let mut chars = upper_camel.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
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
    fn test_to_upper_camel_case() {
        assert_eq!(
            to_upper_camel_case("find_pets_by_status"),
            "FindPetsByStatus"
        );
        assert_eq!(to_upper_camel_case("findPetsByStatus"), "FindPetsByStatus");
        assert_eq!(
            to_upper_camel_case("find-pets-by-status"),
            "FindPetsByStatus"
        );
        assert_eq!(
            to_upper_camel_case("FIND_PETS_BY_STATUS"),
            "FindPetsByStatus"
        );
        assert_eq!(to_upper_camel_case("http_response"), "HttpResponse");
    }

    #[test]
    fn test_to_lower_camel_case() {
        assert_eq!(
            to_lower_camel_case("find_pets_by_status"),
            "findPetsByStatus"
        );
        assert_eq!(to_lower_camel_case("FindPetsByStatus"), "findPetsByStatus");
        assert_eq!(
            to_lower_camel_case("find-pets-by-status"),
            "findPetsByStatus"
        );
        assert_eq!(
            to_lower_camel_case("FIND_PETS_BY_STATUS"),
            "findPetsByStatus"
        );
        assert_eq!(to_lower_camel_case("http_response"), "httpResponse");
    }
}
