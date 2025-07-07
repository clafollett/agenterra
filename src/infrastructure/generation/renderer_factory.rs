//! Factory for selecting appropriate template rendering strategy

use std::sync::Arc;

use crate::generation::TemplateRenderingStrategy;
use crate::protocols::{Protocol, Role};

use super::{DefaultTemplateRenderer, McpServerTemplateRenderer};

/// Select the appropriate template rendering strategy based on protocol and role
pub fn select_renderer(protocol: Protocol, role: Role) -> Arc<dyn TemplateRenderingStrategy> {
    match (protocol, role) {
        (Protocol::Mcp, Role::Server) => {
            // MCP servers need special handling for OpenAPI operations
            Arc::new(McpServerTemplateRenderer::new())
        }
        _ => {
            // Everything else uses the default renderer
            Arc::new(DefaultTemplateRenderer::new())
        }
    }
}
