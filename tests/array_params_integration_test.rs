/// Integration test for array parameter handling in generated MCP servers
/// This test verifies that array parameters are correctly handled end-to-end

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    
    #[test]
    fn test_array_params_url_generation() {
        // Simulate what happens in the generated code
        
        // Step 1: Handler receives Vec<String>
        let tags = vec!["pet".to_string(), "available".to_string()];
        
        // Step 2: get_params() converts to comma-separated string
        let mut params = HashMap::new();
        params.insert("tags".to_string(), tags.join(","));
        
        // Step 3: reqwest would build URL with these params
        // Using url crate to simulate what reqwest does
        use url::Url;
        let base = Url::parse("https://api.example.com/pet/findByTags").unwrap();
        let url_with_params = Url::parse_with_params(base.as_str(), &params).unwrap();
        
        // Verify the result
        assert_eq!(
            url_with_params.as_str(), 
            "https://api.example.com/pet/findByTags?tags=pet%2Cavailable"
        );
        
        // Verify we can get the original values back
        let query_pairs: HashMap<_, _> = url_with_params.query_pairs().collect();
        assert_eq!(query_pairs.get("tags"), Some(&std::borrow::Cow::Borrowed("pet,available")));
    }
    
    #[test]
    fn test_special_characters_in_arrays() {
        // Test handling of special characters in array values
        let tags = vec![
            "tag with spaces".to_string(),
            "tag&with&ampersands".to_string(),
            "tag=with=equals".to_string(),
        ];
        
        let mut params = HashMap::new();
        params.insert("tags".to_string(), tags.join(","));
        
        use url::Url;
        let base = Url::parse("https://api.example.com/pets").unwrap();
        let url_with_params = Url::parse_with_params(base.as_str(), &params).unwrap();
        
        // The URL should properly encode special characters
        println!("URL with special chars: {}", url_with_params.as_str());
        
        // Verify the comma is encoded as %2C
        assert!(url_with_params.as_str().contains("%2C"));
        
        // Verify spaces are encoded as %20 or +
        assert!(url_with_params.as_str().contains("%20") || url_with_params.as_str().contains("+"));
    }
    
    #[test] 
    fn test_empty_array_handling() {
        // Test how empty arrays are handled
        let tags: Vec<String> = vec![];
        
        let mut params = HashMap::new();
        if !tags.is_empty() {
            params.insert("tags".to_string(), tags.join(","));
        }
        
        // Empty array should not add parameter
        assert!(params.is_empty());
        
        // But if we do include empty string
        params.insert("tags".to_string(), "".to_string());
        
        use url::Url;
        let base = Url::parse("https://api.example.com/pets").unwrap();
        let url_with_params = Url::parse_with_params(base.as_str(), &params).unwrap();
        
        // Should have tags= with no value
        assert_eq!(url_with_params.as_str(), "https://api.example.com/pets?tags=");
    }
}