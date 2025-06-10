# Test Fixtures

This directory contains test fixtures for the `Agenterra` project, including canonical OpenAPI Petstore specs for integration, codegen, and reproducibility.

---

## 📦 Petstore Fixture Updater

To keep fixtures up-to-date and always human-readable, use the provided helper script:

```bash
chmod +x update_petstore_fixtures.sh
./update_petstore_fixtures.sh
```

- **Purpose:** Downloads the latest Swagger Petstore OpenAPI v2 and v3 JSON specs and pretty-prints them for offline/reproducible testing.
- **Dependencies:** `curl`, `jq` (install with `brew install jq` on macOS or your Linux package manager)
- **Result:** Files are always pretty-printed for easy review and diffing.
- **Location:** [`update_petstore_fixtures.sh`](./update_petstore_fixtures.sh)

---

## 🔎 Validating Fixture JSON

To validate all fixture JSON files:

```bash
jq . openapi/*.json
```

---

## Swagger Petstore OpenAPI Spec (v2)

- **petstore.swagger.v2.json** is derived from the [Swagger Petstore sample API](https://github.com/swagger-api/swagger-petstore), licensed under the Apache License 2.0.
- The file is used for integration and codegen testing.

**License Notice (v2):**

> This file is derived from the Swagger Petstore sample API (https://github.com/swagger-api/swagger-petstore)
> Licensed under the Apache License, Version 2.0 (the "License");
> you may not use this file except in compliance with the License.
> You may obtain a copy of the License at http://www.apache.org/licenses/LICENSE-2.0

---

## Swagger Petstore OpenAPI Spec (v3)

- **petstore.openapi.v3.json** is derived from [https://petstore3.swagger.io/api/v3/openapi.json](https://petstore3.swagger.io/api/v3/openapi.json) and the [Swagger Petstore sample API](https://github.com/swagger-api/swagger-petstore), licensed under the Apache License 2.0.
- The file is used for integration and codegen testing.

**License Notice (v3):**

> This file is derived from the Swagger Petstore sample API (https://github.com/swagger-api/swagger-petstore) and https://petstore3.swagger.io/api/v3/openapi.json
> Licensed under the Apache License, Version 2.0 (the "License");
> you may not use this file except in compliance with the License.
> You may obtain a copy of the License at http://www.apache.org/licenses/LICENSE-2.0

If you redistribute this repository, please retain this notice and comply with the terms of the Apache 2.0 license.
