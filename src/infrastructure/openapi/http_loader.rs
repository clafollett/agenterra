//! HTTP-based OpenAPI spec loader

use async_trait::async_trait;
use reqwest::Client;
use std::time::Duration;

use crate::generation::{GenerationError, OpenApiLoader, OpenApiSpec};

/// Loads OpenAPI specifications from HTTP/HTTPS URLs
pub struct HttpOpenApiLoader {
    client: Client,
}

impl HttpOpenApiLoader {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .expect("Failed to create HTTP client");

        Self { client }
    }
}

impl Default for HttpOpenApiLoader {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl OpenApiLoader for HttpOpenApiLoader {
    async fn load(&self, source: &str) -> Result<OpenApiSpec, GenerationError> {
        // Only handle HTTP(S) URLs
        if !source.starts_with("http://") && !source.starts_with("https://") {
            return Err(GenerationError::LoadError(format!(
                "HttpOpenApiLoader only handles HTTP(S) URLs, got: {}",
                source
            )));
        }

        // Fetch the content
        let response = self.client.get(source).send().await.map_err(|e| {
            GenerationError::LoadError(format!(
                "Failed to fetch OpenAPI spec from {}: {}",
                source, e
            ))
        })?;

        // Check status and get content type before consuming response
        let status = response.status();
        if !status.is_success() {
            return Err(GenerationError::LoadError(format!(
                "HTTP {} when fetching {}",
                status, source
            )));
        }

        // Get content type to determine format
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // Get the response text
        let content = response.text().await.map_err(|e| {
            GenerationError::LoadError(format!("Failed to read response body: {}", e))
        })?;

        // Parse based on content type or URL extension
        let spec_value = if content_type.contains("json") || source.ends_with(".json") {
            serde_json::from_str(&content).map_err(|e| GenerationError::SerializationError(e))?
        } else if content_type.contains("yaml")
            || source.ends_with(".yaml")
            || source.ends_with(".yml")
        {
            serde_yaml::from_str(&content)
                .map_err(|e| GenerationError::LoadError(format!("Failed to parse YAML: {}", e)))?
        } else {
            // Try JSON first, then YAML
            serde_json::from_str(&content)
                .or_else(|_| serde_yaml::from_str(&content))
                .map_err(|e| {
                    GenerationError::LoadError(format!("Failed to parse OpenAPI spec: {}", e))
                })?
        };

        // Use the dedicated parser to parse the complete specification
        let parser = super::parser::OpenApiParser::new(spec_value);
        parser.parse().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_http_loader_json() {
        // Start a mock server
        let mock_server = MockServer::start().await;

        // Set up the mock
        let spec_json = r#"{
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {}
        }"#;

        Mock::given(method("GET"))
            .and(path("/openapi.json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(spec_json)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        // Test loading
        let loader = HttpOpenApiLoader::new();
        let url = format!("{}/openapi.json", mock_server.uri());
        let result = loader.load(&url).await;

        assert!(result.is_ok());
        let spec = result.unwrap();
        assert_eq!(spec.version, "3.0.0");
        assert_eq!(spec.info.title, "Test API");
    }

    #[tokio::test]
    async fn test_http_loader_yaml() {
        let mock_server = MockServer::start().await;

        let spec_yaml = r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths: {}"#;

        Mock::given(method("GET"))
            .and(path("/openapi.yaml"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(spec_yaml)
                    .insert_header("content-type", "application/x-yaml"),
            )
            .mount(&mock_server)
            .await;

        let loader = HttpOpenApiLoader::new();
        let url = format!("{}/openapi.yaml", mock_server.uri());
        let result = loader.load(&url).await;

        assert!(result.is_ok());
        let spec = result.unwrap();
        assert_eq!(spec.info.title, "Test API");
    }

    #[tokio::test]
    async fn test_http_loader_404() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/notfound"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        let loader = HttpOpenApiLoader::new();
        let url = format!("{}/notfound", mock_server.uri());
        let result = loader.load(&url).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            GenerationError::LoadError(msg) => {
                assert!(msg.contains("HTTP 404"));
            }
            _ => panic!("Expected LoadError"),
        }
    }

    #[tokio::test]
    async fn test_http_loader_non_http_url() {
        let loader = HttpOpenApiLoader::new();
        let result = loader.load("file:///path/to/spec.yaml").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            GenerationError::LoadError(msg) => {
                assert!(msg.contains("only handles HTTP"));
            }
            _ => panic!("Expected LoadError"),
        }
    }
}
