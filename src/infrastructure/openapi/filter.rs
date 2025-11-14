use openapiv3::{OpenAPI, PathItem, ReferenceOr, Schema, SchemaKind, Type};
use indexmap::IndexMap;
pub fn filter_openapi_spec(mut spec: OpenAPI) -> OpenAPI {
    // 1. Remove all responses from endpoints
    for path_item in spec.paths.paths.values_mut() {
        if let ReferenceOr::Item(item) = path_item {
            for operation in item_operations_mut(item) {
                operation.responses = Default::default(); // Removes all responses
            }
        }
    }

    // 1.5. Remove all responses from components to avoid combinatorial explosion
    if let Some(components) = spec.components.as_mut() {
        components.responses = Default::default();
    }

    // 2. Remove examples and readOnly fields from schemas
    if let Some(components) = spec.components.as_mut() {
        // Clone the schemas map for reference resolution to avoid mutable/immutable borrow conflict
        let immutable_schemas = components.schemas.clone(); // This will be an IndexMap
        let schema_names: Vec<String> = components.schemas.keys().cloned().collect();

        for schema_name in schema_names {
            if let Some(ReferenceOr::Item(schema_object)) = components.schemas.get_mut(&schema_name) {
                filter_schema_object(&immutable_schemas, schema_object);
            }
        }
    }

    spec
}

/// Returns a mutable iterator over the operations of a PathItem.
fn item_operations_mut(item: &mut PathItem) -> impl Iterator<Item = &mut openapiv3::Operation> {
    [
        &mut item.get,
        &mut item.put,
        &mut item.post,
        &mut item.delete,
        &mut item.options,
        &mut item.head,
        &mut item.patch,
        &mut item.trace,
    ]
    .into_iter()
    .filter_map(|op| op.as_mut())
}

/// Resolves a schema reference within the provided schemas map.
fn resolve_schema_ref<'a>(schemas: &'a IndexMap<String, ReferenceOr<Schema>>, reference: &'a ReferenceOr<Schema>) -> Option<&'a Schema> {
    match reference {
        ReferenceOr::Item(schema) => Some(schema),
        ReferenceOr::Reference { reference } => {
            let parts: Vec<&str> = reference.split('/').collect();
            if parts.len() == 4 && parts[0] == "#" && parts[1] == "components" && parts[2] == "schemas" {
                let schema_name = parts[3];
                schemas.get(schema_name)?.as_item()
            } else {
                None
            }
        }
    }
}

/// Cleans an OpenAPI schema object (for nested references and allOf).
fn filter_schema_object(schemas_for_ref: &IndexMap<String, ReferenceOr<Schema>>, schema: &mut Schema) {
    schema.schema_data.example = None;
    schema.schema_data.read_only = false;

    match &mut schema.schema_kind {
        SchemaKind::Type(Type::Object(object_schema)) => {
            // The type is Option<AdditionalProperties>, not Option<Box<...>>.
            // We use as_ref() to get an Option<&AdditionalProperties> to match against.
            if let Some(openapiv3::AdditionalProperties::Any(false)) =
                object_schema.additional_properties.as_ref()
            {
                object_schema.additional_properties = None;
            }

            for prop_schema in object_schema.properties.values_mut() {
                if let ReferenceOr::Item(s) = prop_schema {
                    filter_schema_object(schemas_for_ref, s);
                }
            }
        }
        SchemaKind::AllOf { all_of } => {
            // Flatten allOf schemas
            let mut merged_properties: IndexMap<String, ReferenceOr<Box<Schema>>> = IndexMap::new();
            let mut merged_required = Vec::new();

            for sub_schema_ref in all_of.drain(..) {
                if let Some(sub_schema) = resolve_schema_ref(schemas_for_ref, &sub_schema_ref) {
                    // Recursively filter sub-schemas before merging
                    let mut mutable_sub_schema = sub_schema.clone(); // Clone to modify
                    filter_schema_object(schemas_for_ref, &mut mutable_sub_schema);

                    if let SchemaKind::Type(Type::Object(obj_schema)) = &mut mutable_sub_schema.schema_kind {
                        for (name, prop_boxed) in obj_schema.properties.iter() {
                            let boxed_prop = match prop_boxed {
                                ReferenceOr::Item(boxed_s) => ReferenceOr::Item(boxed_s.clone()),
                                ReferenceOr::Reference { reference } => ReferenceOr::Reference { reference: reference.clone() },
                            };
                            merged_properties.insert(name.clone(), boxed_prop);
                        }
                        let mut required_props_vec = std::mem::take(&mut obj_schema.required);
                        merged_required.extend(required_props_vec.drain(..));
                    }
                }
            }

            // Replace the AllOf with a single Object schema
            use openapiv3::ObjectType;
            schema.schema_kind = SchemaKind::Type(Type::Object(ObjectType {
                properties: merged_properties,
                required: merged_required,
                ..Default::default()
            }));

            // Recursively filter the newly created object schema
            if let SchemaKind::Type(Type::Object(object_schema)) = &mut schema.schema_kind {
                for prop_schema in object_schema.properties.values_mut() {
                    if let ReferenceOr::Item(s) = prop_schema {
                        filter_schema_object(schemas_for_ref, s);
                    }
                }
            }
        }
        SchemaKind::OneOf { one_of } => {
            // Fix invalid oneOf with strings instead of schema objects
            let mut fixed_one_of = Vec::new();
            for item in one_of.iter() {
                match item {
                    ReferenceOr::Item(schema) => {
                        // If it's already a schema, keep it
                        fixed_one_of.push(ReferenceOr::Item(schema.clone()));
                    }
                    ReferenceOr::Reference { reference } => {
                        // If it's a reference, keep it
                        fixed_one_of.push(ReferenceOr::Reference { reference: reference.clone() });
                    }
                }
            }
            // Replace the one_of with the fixed version
            *one_of = fixed_one_of;
        }
        _ => {}
    }
}
