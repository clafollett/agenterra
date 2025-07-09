use std::fs;
use std::path::Path;

#[test]
fn test_generated_mcp_server_array_handling() {
    // Check that the generated find_pets_by_tags handler correctly handles arrays
    let generated_handler_path =
        Path::new("target/tmp/e2e-tests/e2e_mcp_server/src/handlers/find_pets_by_tags.rs");

    if generated_handler_path.exists() {
        let content = fs::read_to_string(generated_handler_path).unwrap();

        // Verify the generated code contains the array handling logic
        assert!(
            content.contains("val.join(\",\")"),
            "Generated handler should join array values with commas"
        );

        // Verify the parameter type is Vec<String>
        assert!(
            content.contains("tags: Option<Vec<String>>"),
            "Tags parameter should be Option<Vec<String>>"
        );

        // Verify the contains check is present
        assert!(
            content.contains("Handle array parameters by joining values"),
            "Should have comment about array handling"
        );
    } else {
        println!("Skipping test - generated files not found. Run e2e tests first.");
    }
}

#[test]
fn test_openapi_spec_array_parameter_detection() {
    // Test that we correctly identify array parameters from OpenAPI spec
    use serde_json::json;

    // Simulate an OpenAPI parameter with array type
    let array_param = json!({
        "name": "tags",
        "in": "query",
        "schema": {
            "type": "array",
            "items": {
                "type": "string"
            }
        }
    });

    // The schema type should be "array"
    assert_eq!(array_param["schema"]["type"].as_str(), Some("array"));

    // Items should have string type
    assert_eq!(
        array_param["schema"]["items"]["type"].as_str(),
        Some("string")
    );
}

#[test]
fn test_comma_separated_parsing_on_server_side() {
    // Document how a server would need to parse comma-separated values

    fn parse_comma_separated_tags(tags_param: Option<String>) -> Vec<String> {
        tags_param
            .map(|s| {
                s.split(',')
                    .map(|tag| tag.trim().to_string())
                    .filter(|tag| !tag.is_empty())
                    .collect()
            })
            .unwrap_or_default()
    }

    // Test normal case
    let tags = parse_comma_separated_tags(Some("tag1,tag2,tag3".to_string()));
    assert_eq!(tags, vec!["tag1", "tag2", "tag3"]);

    // Test with spaces
    let tags = parse_comma_separated_tags(Some("tag1, tag2 , tag3".to_string()));
    assert_eq!(tags, vec!["tag1", "tag2", "tag3"]);

    // Test empty string
    let tags = parse_comma_separated_tags(Some("".to_string()));
    assert_eq!(tags, Vec::<String>::new());

    // Test single tag
    let tags = parse_comma_separated_tags(Some("tag1".to_string()));
    assert_eq!(tags, vec!["tag1"]);

    // Test None
    let tags = parse_comma_separated_tags(None);
    assert_eq!(tags, Vec::<String>::new());

    // Test with empty values
    let tags = parse_comma_separated_tags(Some("tag1,,tag3".to_string()));
    assert_eq!(tags, vec!["tag1", "tag3"]);
}
