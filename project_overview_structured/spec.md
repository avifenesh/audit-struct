# Product Specification

This document captures the functional specification derived from the feasibility study and technical write-up in `project_overview.md`.

## CLI Specification

### Inputs

- Compiled binaries with DWARF debug info:
  - ELF (Linux), Mach-O (macOS), PE/COFF (Windows).
- Command-line arguments:
  - Target binary path(s).
  - Filters (by module, namespace, regex).
  - Output mode: table / JSON.
  - CI-related flags (e.g., `--fail-on-growth`, `--config`, `--struct`).

### Outputs

- Human-readable table view:
  - Struct name, size, total padding.
  - Per-member rows: offset, type, size, padding gaps.
  - Highlighted padding regions and cache-line crossings.
- JSON output:
  - Machine-friendly representation with explicit fields for:
    - Struct metadata.
    - Member metadata.
    - Padding segments.
    - Cache-line metrics.
  - Top-level `meta` section with compiler, arch, and build metadata.

### Core Functional Requirements

- Enumerate all struct-like types:
  - `DW_TAG_structure_type`, `DW_TAG_union_type`, and relevant class tags.
- Resolve for each:
  - `DW_AT_name`, `DW_AT_byte_size`.
  - Member names, types, sizes, and locations.
- Handle both simple and expression-based `DW_AT_data_member_location`.
- Support DWARF 4 and 5 bitfield encodings with correct handling of endianness.
- Provide a "diff" mode for pairs of binaries:
  - Match structs by fully-qualified name.
  - Show per-member additions/removals/offset changes.
  - Report total padding delta and size delta.

## CI & Policy Specification

### Configuration

- Project-level configuration file (e.g., `.struct-audit.yaml`) supporting:
  - Global budgets (e.g., maximum allowed padding growth per build).
  - Per-struct budgets (e.g., `Event <= 64 bytes`, `Order <= 64 bytes`).
  - Allow lists / ignore patterns (e.g., third-party libraries).
  - Per-build flavor tags (e.g., `linux-gcc-release`, `macos-clang-debug`).

### Behavior

- CI mode (`--ci` or via dedicated subcommand):
  - Produces deterministic JSON output.
  - Applies configured budgets:
    - Fails the process with non-zero exit code when budgets are violated.
    - Writes a short summary for logs (which structs regressed and by how much).
  - Optionally posts results to the SaaS backend.

## SaaS / API Specification

### API

- `POST /api/reports`:
  - Accepts JSON payload matching the CLI output schema.
  - Validates version and schema compatibility.
  - Associates report with repository, branch, commit SHA, and build ID.
- `GET /api/structs/{struct_id}/history`:
  - Returns time-series data for size, padding, and cache metrics.
- `GET /api/repos/{repo_id}/summary`:
  - Returns high-level metrics for dashboards.

### Data Model (Conceptual)

- Repositories:
  - Source control integration details.
  - Configuration and budgets.
- Commits / Builds:
  - SHA, branch, build status, timestamps.
  - Build flavor (compiler, flags, target).
- Structs:
  - Logical identity (name + namespace + language).
  - Associated snapshots over time.
- Snapshots:
  - One record per struct per build.
  - Fields for size, padding bytes, cache-line metrics, and layout hash.

### JSON Schema (Simplified)

The detailed schema is outlined in `project_overview.md`. At a high level:

```json
{
  "meta": {
    "version": "1.0",
    "binary_hash": "sha256:...",
    "compiler": "rustc 1.75.0",
    "arch": "x86_64-unknown-linux-gnu",
    "timestamp": "2024-05-20T10:00:00Z"
  },
  "structs": [
    {
      "name": "my_app::orders::Order",
      "size_bytes": 64,
      "padding_bytes": 8,
      "members": [
        /* ... */
      ],
      "cache_metrics": {
        /* ... */
      }
    }
  ]
}
```

## Non-Functional Requirements

- Performance:
  - Must handle multi-gigabyte binaries within acceptable CI time budgets.
  - Zero-copy and lazy parsing wherever possible.
- Robustness:
  - Safe handling of malformed or partial DWARF info.
  - No crashes on unexpected constructs; degrade gracefully.
- Security & Privacy:
  - Configurable hashing/anonymization of struct names before upload.
  - Clear self-hosted option for highly sensitive customers.

