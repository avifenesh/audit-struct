# Technical Specifications

## Executive Summary

This document provides RFC-level technical specifications for struct-audit's data formats, APIs, and interfaces. These specs serve as the contract between CLI and SaaS components.

---

## 1. Report JSON Schema

The CLI produces JSON reports that can be consumed by the SaaS platform or other tools.

### Schema Definition

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "StructAuditReport",
  "version": "1.0.0",
  "type": "object",
  "required": ["meta", "structs"],
  "properties": {
    "meta": {
      "type": "object",
      "required": ["version", "timestamp", "binary"],
      "properties": {
        "version": {
          "type": "string",
          "description": "Report schema version",
          "example": "1.0.0"
        },
        "timestamp": {
          "type": "string",
          "format": "date-time",
          "description": "ISO 8601 timestamp"
        },
        "binary": {
          "type": "object",
          "required": ["path", "hash", "format"],
          "properties": {
            "path": { "type": "string" },
            "hash": { "type": "string", "pattern": "^sha256:[a-f0-9]{64}$" },
            "format": { "enum": ["elf", "macho", "pe"] },
            "size_bytes": { "type": "integer" }
          }
        },
        "build": {
          "type": "object",
          "properties": {
            "compiler": { "type": "string" },
            "version": { "type": "string" },
            "target": { "type": "string" },
            "profile": { "type": "string" },
            "dwarf_version": { "type": "integer" }
          }
        },
        "git": {
          "type": "object",
          "properties": {
            "commit_sha": { "type": "string", "pattern": "^[a-f0-9]{40}$" },
            "branch": { "type": "string" },
            "author": { "type": "string" },
            "message": { "type": "string" },
            "dirty": { "type": "boolean" }
          }
        }
      }
    },
    "summary": {
      "type": "object",
      "properties": {
        "struct_count": { "type": "integer" },
        "total_size_bytes": { "type": "integer" },
        "total_padding_bytes": { "type": "integer" },
        "total_data_bytes": { "type": "integer" },
        "average_density": { "type": "number", "minimum": 0, "maximum": 1 }
      }
    },
    "structs": {
      "type": "array",
      "items": { "$ref": "#/definitions/StructInfo" }
    }
  },
  "definitions": {
    "StructInfo": {
      "type": "object",
      "required": ["name", "size", "alignment", "members"],
      "properties": {
        "name": {
          "type": "string",
          "description": "Fully qualified struct name"
        },
        "name_hash": {
          "type": "string",
          "description": "SHA256 hash of name (for privacy mode)"
        },
        "size": {
          "type": "integer",
          "description": "Total size in bytes"
        },
        "alignment": {
          "type": "integer",
          "description": "Required alignment in bytes"
        },
        "padding_bytes": {
          "type": "integer",
          "description": "Total padding within struct"
        },
        "density": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "description": "Ratio of data to total size"
        },
        "cache_lines": {
          "type": "integer",
          "description": "Number of 64-byte cache lines spanned"
        },
        "source_location": {
          "type": "object",
          "properties": {
            "file": { "type": "string" },
            "line": { "type": "integer" }
          }
        },
        "members": {
          "type": "array",
          "items": { "$ref": "#/definitions/MemberInfo" }
        },
        "padding_holes": {
          "type": "array",
          "items": { "$ref": "#/definitions/PaddingHole" }
        }
      }
    },
    "MemberInfo": {
      "type": "object",
      "required": ["name", "offset", "size", "type_name"],
      "properties": {
        "name": { "type": "string" },
        "offset": { "type": "integer" },
        "size": { "type": "integer" },
        "type_name": { "type": "string" },
        "alignment": { "type": "integer" },
        "is_bitfield": { "type": "boolean" },
        "bit_offset": { "type": "integer" },
        "bit_size": { "type": "integer" },
        "straddles_cache_line": { "type": "boolean" }
      }
    },
    "PaddingHole": {
      "type": "object",
      "required": ["offset", "size", "kind"],
      "properties": {
        "offset": { "type": "integer" },
        "size": { "type": "integer" },
        "kind": { "enum": ["leading", "internal", "tail"] },
        "after_member": { "type": "string" }
      }
    }
  }
}
```

### Example Report

```json
{
  "meta": {
    "version": "1.0.0",
    "timestamp": "2024-06-15T10:30:00Z",
    "binary": {
      "path": "./target/release/trading-engine",
      "hash": "sha256:a1b2c3d4e5f6...",
      "format": "elf",
      "size_bytes": 15728640
    },
    "build": {
      "compiler": "rustc",
      "version": "1.78.0",
      "target": "x86_64-unknown-linux-gnu",
      "profile": "release",
      "dwarf_version": 5
    },
    "git": {
      "commit_sha": "abc123def456789...",
      "branch": "feature/order-optimization",
      "author": "dev@example.com",
      "message": "Optimize Order struct layout",
      "dirty": false
    }
  },
  "summary": {
    "struct_count": 142,
    "total_size_bytes": 12456,
    "total_padding_bytes": 1234,
    "total_data_bytes": 11222,
    "average_density": 0.901
  },
  "structs": [
    {
      "name": "trading::Order",
      "size": 72,
      "alignment": 8,
      "padding_bytes": 8,
      "density": 0.889,
      "cache_lines": 2,
      "source_location": {
        "file": "src/trading/order.rs",
        "line": 15
      },
      "members": [
        { "name": "id", "offset": 0, "size": 8, "type_name": "u64", "alignment": 8 },
        { "name": "timestamp", "offset": 8, "size": 8, "type_name": "i64", "alignment": 8 },
        { "name": "price", "offset": 16, "size": 8, "type_name": "f64", "alignment": 8 },
        { "name": "quantity", "offset": 24, "size": 4, "type_name": "u32", "alignment": 4 },
        { "name": "symbol", "offset": 32, "size": 32, "type_name": "[u8; 32]", "alignment": 1 },
        { "name": "is_active", "offset": 64, "size": 1, "type_name": "bool", "alignment": 1 }
      ],
      "padding_holes": [
        { "offset": 28, "size": 4, "kind": "internal", "after_member": "quantity" },
        { "offset": 65, "size": 7, "kind": "tail" }
      ]
    }
  ]
}
```

---

## 2. Configuration File Specification

### File: `.struct-audit.yaml`

```yaml
# struct-audit configuration
# Place in repository root

version: 1

# Analysis settings
analysis:
  # Cache line size in bytes (default: 64)
  cache_line_size: 64

  # Target architecture (auto-detected from binary if not set)
  # Options: x86_64, aarch64, arm, riscv64, wasm32
  architecture: auto

# Include/exclude patterns (regex)
filters:
  include:
    - "^my_app::.*"
    - "^core_lib::.*"

  exclude:
    - "^std::.*"
    - "^alloc::.*"
    - "^core::.*"
    - ".*Test.*"
    - ".*Mock.*"

# Size and padding budgets
budgets:
  # Global defaults (optional)
  defaults:
    max_padding_ratio: 0.20  # Max 20% padding

  # Per-struct budgets
  structs:
    - pattern: "my_app::Order"
      max_size: 64
      max_padding: 0
      must_fit_cache_line: true

    - pattern: "my_app::Tick"
      max_size: 32

    - pattern: "my_app::Config"
      max_padding: 16

# CI behavior
ci:
  # Fail if any budget is exceeded
  fail_on_budget_exceeded: true

  # Fail if total padding increases vs. baseline
  fail_on_padding_increase: true

  # Fail if any struct size increases
  fail_on_size_increase: false

  # Baseline for comparison
  baseline:
    # Branch to compare against
    branch: main
    # Or specific commit
    # commit: abc123

# Output settings
output:
  # Default format: text, json, markdown
  format: text

  # Show detailed member info
  verbose: false

  # Colorize output (auto, always, never)
  color: auto

# SaaS integration (optional)
saas:
  # API endpoint
  endpoint: https://api.struct-audit.io

  # Organization slug
  org: my-company

  # Repository name (defaults to git remote)
  repo: auto
```

---

## 3. CLI Interface Specification

### Commands

```
struct-audit 0.1.0
Memory layout analysis for systems programming

USAGE:
    struct-audit <COMMAND>

COMMANDS:
    inspect    Analyze struct layouts in a binary
    diff       Compare layouts between two binaries
    check      CI mode: validate against budgets
    upload     Upload report to struct-audit SaaS
    explain    Show detailed explanation of a specific struct
    init       Create a default .struct-audit.yaml
    help       Print help information

OPTIONS:
    -h, --help       Print help
    -V, --version    Print version
```

### `inspect` Command

```
Analyze struct layouts in a binary

USAGE:
    struct-audit inspect [OPTIONS] <BINARY>

ARGS:
    <BINARY>    Path to binary file (ELF, Mach-O, or PE)

OPTIONS:
    -f, --format <FORMAT>      Output format [default: text]
                               [possible values: text, json, markdown]
    -o, --output <FILE>        Write output to file
    --include <PATTERN>        Include structs matching pattern (regex)
    --exclude <PATTERN>        Exclude structs matching pattern (regex)
    --config <FILE>            Path to config file [default: .struct-audit.yaml]
    --cache-line <SIZE>        Cache line size in bytes [default: 64]
    --sort <FIELD>             Sort structs by field
                               [possible values: name, size, padding, density]
    --limit <N>                Show only top N structs
    --verbose                  Show detailed member information
    --no-color                 Disable colored output
    -h, --help                 Print help
```

### `diff` Command

```
Compare layouts between two binaries

USAGE:
    struct-audit diff [OPTIONS] <BASE> <HEAD>

ARGS:
    <BASE>    Base binary (e.g., main branch build)
    <HEAD>    Head binary (e.g., feature branch build)

OPTIONS:
    -f, --format <FORMAT>      Output format [default: text]
    -o, --output <FILE>        Write output to file
    --include <PATTERN>        Include structs matching pattern
    --exclude <PATTERN>        Exclude structs matching pattern
    --config <FILE>            Path to config file
    --only-changes             Show only changed structs
    --no-color                 Disable colored output
    -h, --help                 Print help
```

### `check` Command

```
CI mode: validate against budgets

USAGE:
    struct-audit check [OPTIONS] <BINARY>

ARGS:
    <BINARY>    Path to binary file

OPTIONS:
    --config <FILE>            Path to config file [default: .struct-audit.yaml]
    --baseline <BINARY>        Baseline binary for comparison
    --fail-on-growth           Fail if any struct grew in size
    --fail-on-new-padding      Fail if any new padding was introduced
    -f, --format <FORMAT>      Output format for report
    -o, --output <FILE>        Write report to file
    -h, --help                 Print help

EXIT CODES:
    0    All checks passed
    1    Budget exceeded or regression detected
    2    Configuration error
    3    Binary parsing error
```

### `upload` Command

```
Upload report to struct-audit SaaS

USAGE:
    struct-audit upload [OPTIONS] <BINARY>

ARGS:
    <BINARY>    Path to binary file

OPTIONS:
    --token <TOKEN>            API token (or set STRUCT_AUDIT_TOKEN env var)
    --endpoint <URL>           API endpoint [default: https://api.struct-audit.io]
    --org <ORG>                Organization slug
    --repo <REPO>              Repository name [default: auto-detect from git]
    --commit <SHA>             Git commit SHA [default: HEAD]
    --branch <BRANCH>          Git branch name [default: current branch]
    --config <FILE>            Path to config file
    --hash-names               Hash struct names for privacy
    --hash-salt <SALT>         Salt for name hashing (or set STRUCT_AUDIT_SALT)
    -h, --help                 Print help
```

---

## 4. REST API Specification

### Base URL

```
Production: https://api.struct-audit.io/v1
```

### Authentication

All API requests require an API token in the header:

```
Authorization: Bearer <api_token>
```

### Endpoints

#### Upload Report

```http
POST /reports

Content-Type: application/json
Authorization: Bearer <token>

Request Body: StructAuditReport (see JSON schema above)

Response 201:
{
  "id": "rpt_abc123",
  "status": "processed",
  "summary": {
    "struct_count": 142,
    "total_padding_bytes": 1234,
    "regressions": 2,
    "improvements": 1
  },
  "diff_url": "https://struct-audit.io/owner/repo/reports/rpt_abc123"
}

Response 400:
{
  "error": "invalid_report",
  "message": "Missing required field: meta.binary.hash"
}

Response 401:
{
  "error": "unauthorized",
  "message": "Invalid or expired API token"
}
```

#### Get Struct History

```http
GET /repos/{owner}/{repo}/structs/{struct_name}/history

Query Parameters:
  - from: ISO 8601 date (optional)
  - to: ISO 8601 date (optional)
  - branch: branch name (default: main)
  - limit: max records (default: 100)

Response 200:
{
  "struct_name": "trading::Order",
  "branch": "main",
  "history": [
    {
      "commit_sha": "abc123",
      "timestamp": "2024-06-15T10:30:00Z",
      "size": 72,
      "padding_bytes": 8,
      "density": 0.889,
      "cache_lines": 2
    },
    {
      "commit_sha": "def456",
      "timestamp": "2024-06-14T09:00:00Z",
      "size": 64,
      "padding_bytes": 0,
      "density": 1.0,
      "cache_lines": 1
    }
  ]
}
```

#### Compare Commits

```http
GET /repos/{owner}/{repo}/diff

Query Parameters:
  - base: commit SHA or branch name (required)
  - head: commit SHA or branch name (required)

Response 200:
{
  "base": {
    "commit_sha": "abc123",
    "branch": "main"
  },
  "head": {
    "commit_sha": "def456",
    "branch": "feature/xyz"
  },
  "summary": {
    "added": 3,
    "removed": 1,
    "changed": 5,
    "unchanged": 133,
    "total_size_delta": 256,
    "total_padding_delta": 48
  },
  "regressions": [
    {
      "struct_name": "trading::Order",
      "size_delta": 8,
      "padding_delta": 8,
      "severity": "warning"
    }
  ],
  "improvements": [
    {
      "struct_name": "trading::Config",
      "padding_delta": -8
    }
  ]
}
```

#### List Repository Structs

```http
GET /repos/{owner}/{repo}/structs

Query Parameters:
  - branch: branch name (default: main)
  - sort: name|size|padding|density (default: name)
  - order: asc|desc (default: asc)
  - limit: max records (default: 100)
  - offset: pagination offset

Response 200:
{
  "branch": "main",
  "commit_sha": "abc123",
  "total_count": 142,
  "structs": [
    {
      "name": "trading::Order",
      "size": 72,
      "padding_bytes": 8,
      "density": 0.889,
      "cache_lines": 2,
      "last_changed": "2024-06-15T10:30:00Z"
    }
  ]
}
```

### Webhook Events

The SaaS sends webhooks for integration with external systems:

```http
POST <your-webhook-url>

Headers:
  X-Struct-Audit-Signature: sha256=<hmac>
  X-Struct-Audit-Event: report.processed

Body:
{
  "event": "report.processed",
  "timestamp": "2024-06-15T10:30:00Z",
  "payload": {
    "report_id": "rpt_abc123",
    "repo": "owner/repo",
    "commit_sha": "abc123",
    "regressions_count": 2,
    "check_status": "failure"
  }
}
```

---

## 5. Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success, no issues |
| 1 | Budget exceeded or regression detected |
| 2 | Configuration error (invalid YAML, missing file) |
| 3 | Binary parsing error (not a valid binary, missing debug info) |
| 4 | Network error (upload failed, API unreachable) |
| 5 | Authentication error (invalid token) |

---

## 6. Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `STRUCT_AUDIT_TOKEN` | API token for SaaS uploads | (none) |
| `STRUCT_AUDIT_ENDPOINT` | API endpoint URL | https://api.struct-audit.io |
| `STRUCT_AUDIT_SALT` | Salt for name hashing | (none) |
| `STRUCT_AUDIT_CONFIG` | Path to config file | .struct-audit.yaml |
| `NO_COLOR` | Disable colored output | (none) |

---

*Previous: [Roadmap](./07-roadmap.md) | Next: [Tasks](./09-tasks.md)*
