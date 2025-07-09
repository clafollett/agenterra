# Integration Tests

This directory contains integration tests for the {{ project_name }} MCP client.

## Mock Server Tests

The `test_with_mock_server.rs` file contains integration tests that require a mock MCP server. These tests are marked as `#[ignore]` by default because they require an external server.

### Running the Tests

To run these tests, you need to provide a path to an MCP server binary:

```bash
MOCK_SERVER_PATH=/path/to/mcp/server cargo test -- --ignored
```

### Creating a Mock Server

You can use any MCP server that implements the protocol. For testing purposes, here's a minimal example using Python:

```python
#!/usr/bin/env python3
# save as mock_server.py and make executable: chmod +x mock_server.py

import json
import sys

def handle_request(request):
    method = request.get("method", "")
    
    if method == "initialize":
        return {
            "protocolVersion": "2025-06-18",
            "capabilities": {
                "tools": {"listChanged": False},
                "resources": {"listChanged": False},
                "prompts": {"listChanged": False}
            },
            "serverInfo": {
                "name": "mock-server",
                "version": "1.0.0"
            }
        }
    
    elif method == "tools/list":
        return {
            "tools": [
                {
                    "name": "echo",
                    "description": "Echo tool",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "message": {"type": "string"}
                        }
                    }
                },
                {
                    "name": "test_disabled",
                    "description": "Test disabled tool"
                },
                {
                    "name": "test_allowed", 
                    "description": "Test allowed tool"
                }
            ]
        }
    
    elif method == "tools/call":
        name = request.get("params", {}).get("name", "")
        if name == "echo":
            message = request.get("params", {}).get("arguments", {}).get("message", "")
            return {
                "content": [{"type": "text", "text": f"Echo: {message}"}]
            }
        else:
            return {
                "content": [{"type": "text", "text": f"Called {name}"}]
            }
    
    return {"error": {"code": -32601, "message": "Method not found"}}

# Simple JSON-RPC over stdio
while True:
    try:
        line = sys.stdin.readline()
        if not line:
            break
        
        request = json.loads(line)
        result = handle_request(request)
        
        response = {
            "jsonrpc": "2.0",
            "id": request.get("id"),
            "result": result
        }
        
        print(json.dumps(response))
        sys.stdout.flush()
        
    except Exception as e:
        sys.stderr.write(f"Error: {e}\n")
```

Then run the tests:

```bash
MOCK_SERVER_PATH=./mock_server.py cargo test -- --ignored
```