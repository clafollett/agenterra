//! OpenAPI loading implementations

pub mod composite_loader;
pub mod context;
pub mod file_loader;
pub mod http_loader;
pub mod parser;

pub use composite_loader::CompositeOpenApiLoader;
pub use file_loader::FileOpenApiLoader;
pub use http_loader::HttpOpenApiLoader;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generation::OpenApiLoader;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_file_openapi_loader_json() {
        let loader = FileOpenApiLoader::new();

        // Create temp file with JSON OpenAPI spec
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let spec_json = r#"{
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {}
        }"#;

        temp_file
            .write_all(spec_json.as_bytes())
            .expect("Failed to write temp file");
        temp_file.flush().expect("Failed to flush temp file");

        // Load the spec
        let result = loader.load(temp_file.path().to_str().unwrap()).await;
        assert!(result.is_ok());

        let spec = result.unwrap();
        assert_eq!(spec.version, "3.0.0");
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.info.version, "1.0.0");
    }

    #[tokio::test]
    async fn test_file_openapi_loader_yaml() {
        let loader = FileOpenApiLoader::new();

        // Create temp file with YAML OpenAPI spec
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let spec_yaml = r#"openapi: 3.0.0
info:
  title: Test API
  version: 1.0.0
paths: {}"#;

        temp_file
            .write_all(spec_yaml.as_bytes())
            .expect("Failed to write temp file");
        temp_file.flush().expect("Failed to flush temp file");

        // Load the spec
        let result = loader.load(temp_file.path().to_str().unwrap()).await;
        assert!(result.is_ok());

        let spec = result.unwrap();
        assert_eq!(spec.version, "3.0.0");
        assert_eq!(spec.info.title, "Test API");
    }

    #[tokio::test]
    async fn test_file_openapi_loader_not_found() {
        let loader = FileOpenApiLoader::new();

        let result = loader.load("/nonexistent/file.yaml").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_composite_loader_file() {
        let loader = CompositeOpenApiLoader::new();

        // Create temp file
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let spec_json = r#"{
            "openapi": "3.0.0",
            "info": {
                "title": "Test API",
                "version": "1.0.0"
            },
            "paths": {}
        }"#;

        temp_file
            .write_all(spec_json.as_bytes())
            .expect("Failed to write temp file");
        temp_file.flush().expect("Failed to flush temp file");

        // Should load from file
        let result = loader.load(temp_file.path().to_str().unwrap()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_composite_loader_http() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let spec_json = r#"{
            "openapi": "3.0.0",
            "info": {
                "title": "HTTP Test API",
                "version": "2.0.0"
            },
            "paths": {}
        }"#;

        Mock::given(method("GET"))
            .and(path("/api-spec.json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(spec_json)
                    .insert_header("content-type", "application/json"),
            )
            .mount(&mock_server)
            .await;

        let loader = CompositeOpenApiLoader::new();
        let url = format!("{}/api-spec.json", mock_server.uri());
        let result = loader.load(&url).await;

        assert!(result.is_ok());
        let spec = result.unwrap();
        assert_eq!(spec.info.title, "HTTP Test API");
        assert_eq!(spec.info.version, "2.0.0");
    }
}
