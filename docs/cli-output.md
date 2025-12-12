# CLI Output Schema

## JSON Output Format

When using `--output json` or `-o json`, struct-audit outputs JSON conforming to this schema.

### Inspect Command

```json
{
  "version": "0.2.2",
  "structs": [
    {
      "name": "my_app::Order",
      "size": 72,
      "alignment": 8,
      "source_location": {
        "file": "file#1",
        "line": 15
      },
      "members": [
        {
          "name": "id",
          "type_name": "u64",
          "offset": 0,
          "size": 8
        },
        {
          "name": "flags",
          "type_name": "u32",
          "offset": 8,
          "size": 4,
          "bit_offset": 0,
          "bit_size": 3
        }
      ],
      "metrics": {
        "total_size": 72,
        "useful_size": 58,
        "padding_bytes": 14,
        "padding_percentage": 19.4,
        "cache_lines_spanned": 2,
        "cache_line_density": 45.3,
        "padding_holes": [
          {
            "offset": 9,
            "size": 7,
            "after_member": "is_active"
          }
        ],
        "partial": false
      }
    }
  ]
}
```

### Schema Definition

```typescript
interface Output {
  version: string;           // CLI version
  structs: StructLayout[];
}

interface StructLayout {
  name: string;              // Fully qualified name
  size: number;              // Total size in bytes
  alignment?: number;        // Alignment requirement (if known)
  source_location?: {
    file: string;            // Source file (may be "file#N" index)
    line: number;
  };
  members: MemberLayout[];
  metrics: LayoutMetrics;
}

interface MemberLayout {
  name: string;
  type_name: string;
  offset?: number;           // Byte offset (null if unknown)
  size?: number;             // Size in bytes (null if unknown)
  bit_offset?: number;       // For bitfields: bit offset within storage
  bit_size?: number;         // For bitfields: size in bits
}

interface LayoutMetrics {
  total_size: number;
  useful_size: number;
  padding_bytes: number;
  padding_percentage: number;
  cache_lines_spanned: number;
  cache_line_density: number;
  padding_holes: PaddingHole[];
  partial: boolean;          // True if analysis incomplete
}

interface PaddingHole {
  offset: number;
  size: number;
  after_member?: string;     // Member name before this hole
}
```

### Diff Command

```json
{
  "added": [
    { "name": "NewStruct", "size": 32, "padding_bytes": 4 }
  ],
  "removed": [
    { "name": "OldStruct", "size": 24, "padding_bytes": 2 }
  ],
  "changed": [
    {
      "name": "Order",
      "old_size": 64,
      "new_size": 72,
      "size_delta": 8,
      "old_padding": 6,
      "new_padding": 14,
      "padding_delta": 8,
      "member_changes": [
        {
          "kind": "Added",
          "name": "new_field",
          "details": "offset 64, size 8"
        }
      ]
    }
  ],
  "unchanged_count": 42
}
```

### Check Command

Exit codes:
- `0`: All budgets satisfied
- `1`: Budget violations found

Violations are printed to stderr. JSON output shows the same data structured.

## Budget Configuration

`.struct-audit.yaml`:

```yaml
budgets:
  # Exact struct name
  Order:
    max_size: 64
    max_padding: 8
    max_padding_percent: 15.0

  # Multiple constraints
  CriticalStruct:
    max_size: 128
```

All constraints are optional. Only specified constraints are checked.
