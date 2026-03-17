# helm-forge

Helm chart generator backend for iac-forge. Implements `iac_forge::Backend` to generate
Helm charts from TOML resource specs.

## Build & Test

```bash
cargo build
cargo test
```

## Architecture

For each `IacResource`, generates a complete Helm chart:

```
charts/<resource-name>/
  Chart.yaml              # apiVersion: v2, depends on pleme-lib
  values.yaml             # IacAttribute fields as values
  values.schema.json      # JSON Schema from IacType mapping
  templates/
    _helpers.tpl           # Delegates to pleme-lib named templates
    deployment.yaml        # {{- include "pleme-lib.deployment" . }}
    service.yaml           # {{- include "pleme-lib.service" . }}
    configmap.yaml         # Non-sensitive attributes
    secret.yaml            # Sensitive attributes
  tests/
    deployment_test.yaml   # helm-unittest
```

## IacType → JSON Schema Mapping

| IacType | JSON Schema |
|---------|-------------|
| String | `{"type": "string"}` |
| Integer | `{"type": "integer"}` |
| Float | `{"type": "number"}` |
| Boolean | `{"type": "boolean"}` |
| List(T) | `{"type": "array", "items": schema(T)}` |
| Set(T) | `{"type": "array", "items": schema(T), "uniqueItems": true}` |
| Map(T) | `{"type": "object", "additionalProperties": schema(T)}` |
| Object | `{"type": "object", "properties": {...}}` |
| Enum | `{"type": "string", "enum": [...]}` |
| Any | `{}` |

## Modules

| Module | Purpose |
|--------|---------|
| `helm_backend.rs` | `HelmBackend` struct implementing `Backend` trait |
| `chart_gen.rs` | Chart.yaml generation |
| `values_gen.rs` | values.yaml generation (config + secrets sections) |
| `schema_gen.rs` | values.schema.json generation |
| `template_gen.rs` | Helm template generation (delegates to pleme-lib) |
| `test_gen.rs` | helm-unittest test generation |
| `type_map.rs` | IacType → JSON Schema type mapping |
| `naming.rs` | HelmNamingConvention (kebab-case charts, snake_case values) |

## Integration

Used by `iac-forge-cli` with `--backend helm` flag (feature-gated).
Used by `forge-gen` with `--helm helm` flag (invokes iac-forge-cli).

## Sensitive vs Non-Sensitive

- Non-sensitive attributes → `config:` section in values.yaml → ConfigMap template
- Sensitive attributes → `secrets:` section in values.yaml → Secret template
- Generated charts depend on pleme-lib for deployment/service/networkpolicy templates
