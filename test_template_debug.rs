use tera::{Tera, Context};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a minimal Tera instance with just the handlers_mod template
    let template_content = std::fs::read_to_string("templates/rust_axum/handlers_mod.rs.tera")?;
    
    let mut tera = Tera::new("templates/rust_axum/**/*")?;
    
    // Create a test context with all required variables
    let mut context = Context::new();
    context.insert("base_api_url", "https://petstore3.swagger.io/api/v3");
    context.insert("agent_instructions", "");
    context.insert("endpoints", &json!([
        {
            "endpoint": "test_endpoint",
            "fn_name": "test_fn",
            "parameters_type": "TestParams",
            "summary": "Test summary",
            "description": "Test description",
            "tags": ["test"]
        }
    ]));
    
    println!("Template names: {:?}", tera.get_template_names().collect::<Vec<_>>());
    
    // Try to render the template
    match tera.render("handlers_mod.rs.tera", &context) {
        Ok(result) => {
            println!("SUCCESS: Template rendered successfully");
            println!("Result length: {}", result.len());
            if result.len() < 500 {
                println!("Content:\n{}", result);
            } else {
                println!("Content preview:\n{}", &result[..500]);
            }
        }
        Err(e) => {
            println!("ERROR: Failed to render template");
            println!("Error: {}", e);
            println!("Error kind: {:?}", e.kind);
            
            // Try to get more info about the chain
            let mut current = &e as &dyn std::error::Error;
            let mut level = 0;
            while let Some(source) = current.source() {
                level += 1;
                println!("  Error level {}: {}", level, source);
                current = source;
            }
            
            return Err(e.into());
        }
    }
    
    Ok(())
}