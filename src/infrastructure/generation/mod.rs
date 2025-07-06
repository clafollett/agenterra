//! Generation infrastructure implementations

pub mod context_builders;
pub mod default_renderer;
pub mod mcp_server_renderer;
pub mod post_processor;
pub mod renderer_factory;
pub mod template_renderer;

pub use default_renderer::DefaultTemplateRenderer;
pub use mcp_server_renderer::McpServerTemplateRenderer;
pub use post_processor::CompositePostProcessor;
pub use renderer_factory::select_renderer;
pub use template_renderer::TeraTemplateRenderer;
