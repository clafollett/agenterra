Generate ngsi-v2 MCP server:

agenterra scaffold mcp server --schema-path ./ngsi-mcp-server/ngsiv2-openapi.json --project-name ngsi-v2-mcp --port 3300 --base-url http://localhost:1026

works for genration, but problem with parameters or attributes in openapi spec schema file, which are rust keywords, at compile time,

they need then to be escaped like: 'type' which must be preceded by 'r#' to become 'r#type' in rust code !


agenterra scaffold mcp server --schema-path ./ngsi-mcp-server/ngsiv2-openapi.json --project-name ngsi-v2-mcp-2 --port 3300 --base-url http://localhost:1026

code is generated but at compile step:

error: expected identifier, found keyword `type`
  --> src/handlers/list_entities.rs:28:9
   |
20 | pub struct ListEntitiesParams {
   |            ------------------ while parsing this struct
...
28 |     pub type: Option<String>,
   |         ^^^^ expected identifier, found keyword
   |
help: escape `type` to use it as an identifier
   |
28 |     pub r#type: Option<String>,
   |         ++

error: expected identifier, found keyword `type`
  --> src/handlers/list_entities.rs:83:34
   |
83 |         if let Some(val) = &self.type {
   |                                  ^^^^ expected identifier, found keyword
   |
help: escape `type` to use it as an identifier
   |
83 |         if let Some(val) = &self.r#type {