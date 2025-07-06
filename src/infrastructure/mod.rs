//! Infrastructure layer - concrete implementations of domain ports

pub mod generation;
pub mod openapi;
pub mod output;
pub mod shell;
pub mod templates;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infrastructure_module_exists() {
        // Basic test to ensure module compiles
        assert!(true);
    }
}
