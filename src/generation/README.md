# Generation Domain Module

This module orchestrates the code generation workflow, transforming protocol contexts into generated code artifacts.

## Architecture

The generation module follows a clear workflow:
1. **Context Validation**: Ensures all required data is present
2. **Template Discovery**: Finds the appropriate template based on protocol/role/language
3. **Context Building**: Transforms generation context into render context
4. **Template Rendering**: Processes templates with context to create artifacts
5. **Post-Processing**: Applies final transformations to artifacts

## Core Components

### Types (`types.rs`)
- `Language`: Supported programming languages (Rust, Python, TypeScript, etc.)
- `TemplateDescriptor`: Identifies templates by protocol/role/language
- `Template`: Complete template with manifest and files
- `Artifact`: Generated file with content and metadata
- `OpenApiSpec`: Domain representation of OpenAPI specifications

### Context (`context.rs`)
- `GenerationContext`: The core aggregate containing all generation data
- `GenerationMetadata`: Project metadata (name, version, author, etc.)
- `RenderContext`: Simplified context for template rendering

### Orchestrator (`orchestrator.rs`)
- `GenerationOrchestrator`: Coordinates the generation workflow
- `GenerationOrchestratorBuilder`: Builder pattern for orchestrator creation

### Traits (`traits.rs`)
- `TemplateDiscovery`: Port for finding templates
- `ContextBuilder`: Port for building render contexts
- `TemplateRenderer`: Port for rendering templates
- `PostProcessor`: Port for post-processing artifacts
- `OpenApiLoader`: Port for loading OpenAPI specs
- `OperationTransformer`: Port for transforming operations

### Errors (`errors.rs`)
- `GenerationError`: Comprehensive error types for generation failures

## Usage

```rust
use agenterra::generation::{
    GenerationContext, GenerationOrchestrator, Language,
};
use agenterra::protocols::{Protocol, Role};

// Create generation context
let mut context = GenerationContext::new(
    Protocol::Mcp,
    Role::Server,
    Language::Rust,
);

// Add variables
context.add_variable("project_name".to_string(), json!("my-server"));
context.metadata.project_name = "my-server".to_string();

// Use orchestrator to generate
let orchestrator = GenerationOrchestrator::new(
    template_discovery,
    context_builder,
    template_renderer,
    post_processor,
);

let result = orchestrator.generate(context).await?;
```

## Design Decisions

1. **Separation of Concerns**: Generation logic is separate from protocol and template concerns
2. **Port Interfaces**: All external dependencies are behind trait boundaries
3. **Builder Pattern**: Complex objects use builders for construction
4. **Async by Default**: All I/O operations are async
5. **Rich Error Types**: Comprehensive error information for debugging

## Integration Points

The generation module integrates with:
- **Protocol Module**: Receives protocol/role information
- **Template Infrastructure**: Through port interfaces
- **OpenAPI Infrastructure**: Through loader/validator ports
- **File System**: Through artifact writing (in infrastructure layer)