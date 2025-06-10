//! Template-specific types for code generation

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Parameter kind based on OpenAPI "in" field - language agnostic
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterKind {
    Path,
    Query,
    Header,
    Cookie,
}

/// Language-agnostic parameter info with target language type
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateParameterInfo {
    pub name: String,
    pub target_type: String,
    pub description: Option<String>,
    pub example: Option<JsonValue>,
    pub kind: ParameterKind,
}
