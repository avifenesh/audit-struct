# API Specification

## JSON Schemas, REST Endpoints, and Data Contracts

---

## 1. CLI Output Schema

### 1.1 Report Schema (v1.0)

The CLI generates JSON reports conforming to this schema:

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "StructAuditReport",
  "type": "object",
  "required": ["meta", "structs"],
  "properties": {
    "meta": {
      "type": "object",
      "required": ["version", "timestamp"],
      "properties": {
        "version": {
          "type": "string",
          "description": "Schema version",
          "example": "1.0"
        },
        "binary_path": {
          "type": "string",
          "description": "Path to analyzed binary"
        },
        "binary_hash": {
          "type": "string",
          "description": "SHA256 hash of binary",
          "pattern": "^sha256:[a-f0-9]{64}$"
        },
        "compiler": {
          "type": "string",
          "description": "Compiler identification",
          "example": "rustc 1.75.0"
        },
        "arch": {
          "type": "string",
          "description": "Target architecture",
          "example": "x86_64-unknown-linux-gnu"
        },
        "timestamp": {
          "type": "string",
          "format": "date-time",
          "description": "Analysis timestamp"
        },
        "git_commit": {
          "type": "string",
          "description": "Git commit SHA",
          "pattern": "^[a-f0-9]{40}$"
        },
        "git_branch": {
          "type": "string",
          "description": "Git branch name"
        }
      }
    },
    "structs": {
      "type": "array",
      "items": { "$ref": "#/definitions/StructLayout" }
    },
    "summary": {
      "$ref": "#/definitions/ReportSummary"
    }
  },
  "definitions": {
    "StructLayout": {
      "type": "object",
      "required": ["name", "size", "alignment", "members", "metrics"],
      "properties": {
        "name": {
          "type": "string",
          "description": "Fully qualified struct name",
          "example": "my_app::orders::Order"
        },
        "size": {
          "type": "integer",
          "minimum": 0,
          "description": "Total size in bytes"
        },
        "alignment": {
          "type": "integer",
          "minimum": 1,
          "description": "Alignment requirement in bytes"
        },
        "source_file": {
          "type": "string",
          "description": "Source file path"
        },
        "source_line": {
          "type": "integer",
          "description": "Source line number"
        },
        "members": {
          "type": "array",
          "items": { "$ref": "#/definitions/MemberLayout" }
        },
        "metrics": {
          "$ref": "#/definitions/LayoutMetrics"
        }
      }
    },
    "MemberLayout": {
      "type": "object",
      "required": ["name", "type_name", "offset", "size"],
      "properties": {
        "name": {
          "type": "string",
          "description": "Field name"
        },
        "type_name": {
          "type": "string",
          "description": "Type name",
          "example": "uint64_t"
        },
        "offset": {
          "type": "integer",
          "minimum": 0,
          "description": "Byte offset from struct start"
        },
        "size": {
          "type": "integer",
          "minimum": 0,
          "description": "Size in bytes"
        },
        "alignment": {
          "type": "integer",
          "minimum": 1,
          "description": "Alignment requirement"
        },
        "bit_offset": {
          "type": "integer",
          "description": "Bit offset for bitfields"
        },
        "bit_size": {
          "type": "integer",
          "description": "Bit size for bitfields"
        }
      }
    },
    "LayoutMetrics": {
      "type": "object",
      "required": ["padding_bytes", "padding_percent", "cache_lines", "density"],
      "properties": {
        "padding_bytes": {
          "type": "integer",
          "minimum": 0,
          "description": "Total padding bytes"
        },
        "padding_percent": {
          "type": "number",
          "minimum": 0,
          "maximum": 100,
          "description": "Padding as percentage"
        },
        "cache_lines": {
          "type": "integer",
          "minimum": 1,
          "description": "Cache lines spanned"
        },
        "density": {
          "type": "number",
          "minimum": 0,
          "maximum": 1,
          "description": "Data density (0-1)"
        },
        "holes": {
          "type": "array",
          "items": { "$ref": "#/definitions/PaddingHole" }
        }
      }
    },
    "PaddingHole": {
      "type": "object",
      "required": ["offset", "size"],
      "properties": {
        "offset": {
          "type": "integer",
          "description": "Byte offset of hole"
        },
        "size": {
          "type": "integer",
          "description": "Size of hole in bytes"
        },
        "after_field": {
          "type": "string",
          "description": "Field before the hole"
        }
      }
    },
    "ReportSummary": {
      "type": "object",
      "properties": {
        "total_structs": { "type": "integer" },
        "total_size": { "type": "integer" },
        "total_padding": { "type": "integer" },
        "average_density": { "type": "number" }
      }
    }
  }
}
```

### 1.2 Example Report

```json
{
  "meta": {
    "version": "1.0",
    "binary_path": "./target/release/my_app",
    "binary_hash": "sha256:a1b2c3d4e5f6...",
    "compiler": "rustc 1.75.0",
    "arch": "x86_64-unknown-linux-gnu",
    "timestamp": "2024-05-20T10:00:00Z",
    "git_commit": "abc123def456...",
    "git_branch": "main"
  },
  "structs": [
    {
      "name": "my_app::Order",
      "size": 72,
      "alignment": 8,
      "source_file": "src/orders.rs",
      "source_line": 15,
      "members": [
        {
          "name": "id",
          "type_name": "u64",
          "offset": 0,
          "size": 8,
          "alignment": 8
        },
        {
          "name": "is_active",
          "type_name": "bool",
          "offset": 8,
          "size": 1,
          "alignment": 1
        },
        {
          "name": "price",
          "type_name": "f64",
          "offset": 16,
          "size": 8,
          "alignment": 8
        }
      ],
      "metrics": {
        "padding_bytes": 14,
        "padding_percent": 19.4,
        "cache_lines": 2,
        "density": 0.81,
        "holes": [
          {
            "offset": 9,
            "size": 7,
            "after_field": "is_active"
          }
        ]
      }
    }
  ],
  "summary": {
    "total_structs": 42,
    "total_size": 2048,
    "total_padding": 256,
    "average_density": 0.87
  }
}
```

---

## 2. Configuration Schema

### 2.1 `.struct-audit.yaml`

```yaml
# struct-audit configuration file
# Place in repository root

version: 1

# Global settings
settings:
  cache_line_size: 64  # bytes (default: 64)
  ignore_patterns:
    - "std::*"
    - "core::*"
    - "__*"  # Anonymous types

# Struct budgets
budgets:
  # Exact name match
  - name: "my_app::Order"
    max_size: 64
    max_padding_percent: 15
    max_cache_lines: 1
    
  # Glob pattern match
  - pattern: "my_app::hot_path::*"
    max_size: 128
    max_padding_percent: 10
    
  # Namespace-wide
  - pattern: "my_app::network::*"
    max_cache_lines: 2

# CI behavior
ci:
  fail_on_any_regression: true
  fail_on_new_struct_over_cache_line: true
  max_total_padding_increase: 100  # bytes
  
# Output settings
output:
  format: "table"  # table, json, markdown
  show_source_locations: true
  colorize: true
```

### 2.2 Configuration Schema (JSON Schema)

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "StructAuditConfig",
  "type": "object",
  "properties": {
    "version": {
      "type": "integer",
      "const": 1
    },
    "settings": {
      "type": "object",
      "properties": {
        "cache_line_size": {
          "type": "integer",
          "default": 64
        },
        "ignore_patterns": {
          "type": "array",
          "items": { "type": "string" }
        }
      }
    },
    "budgets": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "name": { "type": "string" },
          "pattern": { "type": "string" },
          "max_size": { "type": "integer" },
          "max_padding_percent": { "type": "number" },
          "max_cache_lines": { "type": "integer" }
        },
        "oneOf": [
          { "required": ["name"] },
          { "required": ["pattern"] }
        ]
      }
    },
    "ci": {
      "type": "object",
      "properties": {
        "fail_on_any_regression": { "type": "boolean" },
        "fail_on_new_struct_over_cache_line": { "type": "boolean" },
        "max_total_padding_increase": { "type": "integer" }
      }
    }
  }
}
```

---

## 3. REST API Specification

### 3.1 Authentication

All API requests require authentication via Bearer token:

```
Authorization: Bearer <api_token>
```

Tokens are obtained via OAuth flow or generated in dashboard.

### 3.2 Base URL

```
Production: https://api.struct-audit.io/v1
```

### 3.3 Endpoints

#### POST /reports

Upload a new analysis report.

**Request**:
```http
POST /v1/reports HTTP/1.1
Host: api.struct-audit.io
Authorization: Bearer <token>
Content-Type: application/json

{
  "repository": "owner/repo",
  "commit_sha": "abc123...",
  "branch": "feature/new-order",
  "report": { ... }  // StructAuditReport object
}
```

**Response** (201 Created):
```json
{
  "id": "rpt_abc123",
  "repository": "owner/repo",
  "commit_sha": "abc123...",
  "created_at": "2024-05-20T10:00:00Z",
  "summary": {
    "total_structs": 42,
    "regressions": 2,
    "improvements": 5
  },
  "dashboard_url": "https://app.struct-audit.io/reports/rpt_abc123"
}
```

#### GET /reports/:id

Get a specific report.

**Response** (200 OK):
```json
{
  "id": "rpt_abc123",
  "repository": "owner/repo",
  "commit_sha": "abc123...",
  "branch": "main",
  "created_at": "2024-05-20T10:00:00Z",
  "report": { ... }  // Full StructAuditReport
}
```

#### GET /repos/:owner/:repo/structs

List all structs in a repository.

**Query Parameters**:
- `branch` (optional): Filter by branch
- `limit` (optional): Max results (default: 100)
- `offset` (optional): Pagination offset

**Response** (200 OK):
```json
{
  "structs": [
    {
      "name": "my_app::Order",
      "latest_size": 72,
      "latest_padding_percent": 19.4,
      "trend": "stable",  // stable, improving, degrading
      "last_changed": "2024-05-15T08:00:00Z"
    }
  ],
  "total": 42,
  "limit": 100,
  "offset": 0
}
```

#### GET /repos/:owner/:repo/structs/:name/history

Get history of a specific struct.

**Query Parameters**:
- `since` (optional): Start date (ISO 8601)
- `until` (optional): End date (ISO 8601)
- `limit` (optional): Max results

**Response** (200 OK):
```json
{
  "struct_name": "my_app::Order",
  "history": [
    {
      "commit_sha": "abc123...",
      "timestamp": "2024-05-20T10:00:00Z",
      "size": 72,
      "padding_bytes": 14,
      "cache_lines": 2,
      "change_type": "size_increase"
    },
    {
      "commit_sha": "def456...",
      "timestamp": "2024-05-15T08:00:00Z",
      "size": 64,
      "padding_bytes": 6,
      "cache_lines": 1,
      "change_type": null
    }
  ]
}
```

#### POST /repos/:owner/:repo/budgets

Create a new budget.

**Request**:
```json
{
  "pattern": "my_app::Order",
  "max_size": 64,
  "max_padding_percent": 15,
  "max_cache_lines": 1
}
```

**Response** (201 Created):
```json
{
  "id": "bdg_xyz789",
  "pattern": "my_app::Order",
  "max_size": 64,
  "max_padding_percent": 15,
  "max_cache_lines": 1,
  "created_at": "2024-05-20T10:00:00Z"
}
```

### 3.4 Error Responses

All errors follow this format:

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid report format",
    "details": {
      "field": "report.structs[0].size",
      "reason": "must be non-negative"
    }
  }
}
```

**Error Codes**:

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Invalid or missing token |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `NOT_FOUND` | 404 | Resource not found |
| `VALIDATION_ERROR` | 400 | Invalid request body |
| `RATE_LIMITED` | 429 | Too many requests |
| `INTERNAL_ERROR` | 500 | Server error |

---

## 4. Webhook Payloads

### 4.1 GitHub Check Run

Posted to GitHub Checks API:

```json
{
  "name": "struct-audit",
  "head_sha": "abc123...",
  "status": "completed",
  "conclusion": "failure",
  "output": {
    "title": "Layout Regression Detected",
    "summary": "2 structs exceeded budgets",
    "text": "## Violations\n\n| Struct | Issue |\n|--------|-------|\n| `Order` | Size 72 > 64 bytes |"
  }
}
```

### 4.2 PR Comment

Markdown posted as PR comment:

```markdown
## 📐 struct-audit Report

### Summary
- **Analyzed**: 42 structs
- **Regressions**: 2 ❌
- **Improvements**: 5 ✅

### Regressions

| Struct | Change | Impact |
|--------|--------|--------|
| `Order` | 64 → 72 bytes | ⚠️ Now spans 2 cache lines |
| `User` | Padding 10% → 18% | |

### Improvements

| Struct | Change |
|--------|--------|
| `Tick` | 48 → 40 bytes (-16.7%) |

---
[View full report](https://app.struct-audit.io/reports/rpt_abc123) | [Configure budgets](https://app.struct-audit.io/repos/owner/repo/settings)
```

---

## 5. Rate Limits

| Tier | Requests/min | Reports/day |
|------|--------------|-------------|
| Community | 60 | 100 |
| Pro | 300 | 1,000 |
| Enterprise | Unlimited | Unlimited |

Rate limit headers included in responses:
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1621512000
```

---

## Next Steps

→ [Future Roadmap](./10-future-roadmap.md) - Advanced features and long-term vision


