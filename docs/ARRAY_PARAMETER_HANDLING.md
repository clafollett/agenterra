# Array Parameter Handling in Generated Code

This document describes how Agenterra handles array parameters when generating code from OpenAPI specifications.

## Overview

When an OpenAPI specification defines a parameter with `type: array`, Agenterra generates code that handles these arrays appropriately for HTTP query parameters.

## Implementation Details

### 1. Type Detection

Array parameters are detected in the OpenAPI schema:
```yaml
parameters:
  - name: tags
    in: query
    schema:
      type: array
      items:
        type: string
```

### 2. Code Generation

For array parameters, the generated Rust code:

1. **Parameter Type**: Uses `Option<Vec<T>>` where T is the item type
   ```rust
   pub struct FindPetsByTagsParams {
       pub tags: Option<Vec<String>>,
   }
   ```

2. **Serialization**: Converts arrays to comma-separated strings
   ```rust
   {% if p.target_type is containing("Vec") -%}
   // Handle array parameters by joining values
   params.insert("{{ p.name }}".to_string(), val.join(","));
   {%- else -%}
   params.insert("{{ p.name }}".to_string(), val.to_string());
   {%- endif %}
   ```

3. **HTTP Request**: Sends as query parameter with comma-separated values
   - Input: `vec!["tag1", "tag2", "tag3"]`
   - Query: `?tags=tag1,tag2,tag3`
   - URL encoded: `?tags=tag1%2Ctag2%2Ctag3`

## Server-Side Parsing

Servers receiving these requests need to:

1. Split the parameter value by commas
2. Trim whitespace from each value
3. Handle empty strings appropriately

Example implementation:
```rust
fn parse_array_param(param: Option<String>) -> Vec<String> {
    param
        .map(|s| s.split(',')
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
            .collect())
        .unwrap_or_default()
}
```

## Alternative Approaches

While Agenterra uses comma-separated values, other common approaches include:

1. **Repeated parameters**: `?tags=tag1&tags=tag2&tags=tag3`
   - Not supported with current `HashMap<String, String>` structure
   - Would require `HashMap<String, Vec<String>>`

2. **Bracket notation**: `?tags[]=tag1&tags[]=tag2&tags[]=tag3`
   - PHP/Rails style
   - Not supported in current implementation

3. **JSON arrays**: `?tags=["tag1","tag2","tag3"]`
   - Would need URL encoding
   - More complex parsing

## Rationale

The comma-separated approach was chosen because:

1. Simple to implement with `HashMap<String, String>`
2. Widely supported by REST APIs
3. Easy to parse on server side
4. Works well with URL encoding

## Testing

Array parameter handling is tested in:
- `/tests/test_array_params.rs` - Unit tests for array serialization
- `/tests/test_mcp_array_handling.rs` - Tests for generated code
- `/tests/test_array_params_integration.rs` - Integration tests

## Special Considerations

1. **Empty Arrays**: Empty arrays result in no parameter being sent (not `?tags=`)
2. **Special Characters**: Values containing commas need escaping on server side
3. **URL Encoding**: Commas are encoded as `%2C` in URLs