# Product Specification

## Feature Requirements, User Stories, and Acceptance Criteria

---

## 1. Product Overview

**struct-audit** consists of two integrated components:

```
┌─────────────────────────────────────────────────────────────────┐
│                      struct-audit Platform                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────┐      ┌────────────────────────────┐  │
│  │     CLI Agent        │      │      SaaS Platform         │  │
│  │  (struct-audit)      │─────▶│   (app.struct-audit.io)    │  │
│  │                      │      │                            │  │
│  │  • Local analysis    │      │  • Historical storage      │  │
│  │  • Binary parsing    │      │  • Dashboards              │  │
│  │  • CI integration    │      │  • Team collaboration      │  │
│  │  • Offline capable   │      │  • GitHub/GitLab App       │  │
│  └──────────────────────┘      └────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. User Personas

### 2.1 Individual Developer (Primary)

**Name**: Alex, Systems Engineer  
**Context**: Works on performance-critical C++/Rust code  
**Goals**:
- Understand memory layout of structs during development
- Catch layout regressions before code review
- Learn best practices for cache-efficient design

**Pain Points**:
- `pahole` is Linux-only and lacks CI integration
- No easy way to compare layouts across commits
- Memory layout is "invisible" in normal workflow

### 2.2 Tech Lead / Architect (Secondary)

**Name**: Jordan, Principal Engineer  
**Context**: Responsible for system performance and team standards  
**Goals**:
- Enforce layout budgets on critical data structures
- Track memory efficiency trends over time
- Educate team on performance implications

**Pain Points**:
- No visibility into layout changes across PRs
- Can't set enforceable standards
- Junior developers unknowingly introduce regressions

### 2.3 Engineering Manager (Tertiary)

**Name**: Sam, Engineering Manager  
**Context**: Oversees multiple teams, reports on technical health  
**Goals**:
- Dashboard view of "memory health" across projects
- Demonstrate ROI of performance investments
- Identify systemic issues before they become critical

**Pain Points**:
- Performance is invisible until production issues
- No metrics to track improvement over time
- Difficult to justify performance work to leadership

---

## 3. CLI Features

### 3.1 Core Analysis (`struct-audit inspect`)

**User Story**: As a developer, I want to see the memory layout of structs in my binary so that I can identify padding and optimize cache usage.

**Command**:
```bash
struct-audit inspect ./target/release/my_app
```

**Output Example**:
```
┌─────────────────────────────────────────────────────────────────┐
│ struct my_app::Order                                            │
│ Size: 72 bytes | Padding: 14 bytes (19.4%) | Cache Lines: 2    │
├─────────┬────────────┬──────┬──────────────────────────────────┤
│ Offset  │ Size       │ Type │ Field                            │
├─────────┼────────────┼──────┼──────────────────────────────────┤
│ 0       │ 8          │ u64  │ id                               │
│ 8       │ 1          │ bool │ is_active                        │
│ 9       │ [7 bytes]  │ PAD  │ ████████████████████████████     │
│ 16      │ 8          │ f64  │ price                            │
│ 24      │ 8          │ f64  │ quantity                         │
│ 32      │ 1          │ u8   │ side                             │
│ 33      │ [7 bytes]  │ PAD  │ ████████████████████████████     │
│ 40      │ 8          │ u64  │ timestamp                        │
│ 48      │ 24         │ Str  │ symbol                           │
└─────────┴────────────┴──────┴──────────────────────────────────┘
│ ⚠️  Cache line boundary at offset 64                            │
└─────────────────────────────────────────────────────────────────┘
```

**Acceptance Criteria**:
- [x] Parses ELF, Mach-O, and PE binaries (DWARF debug info required)
- [x] Displays offset, size, type, and field name
- [x] Highlights padding holes with visual indicator
- [x] Shows cache line boundaries
- [x] Calculates padding percentage
- [x] Supports filtering by struct name (substring match)
- [x] Colorized output for terminal
- [x] JSON output format
- [x] Sorting by name, size, padding, padding percentage
- [x] `--top N` to limit results
- [x] `--min-padding N` to filter by minimum padding

---

### 3.2 Differential Analysis (`struct-audit diff`)

**User Story**: As a developer, I want to compare struct layouts between two binaries so that I can identify regressions before merging.

**Command**:
```bash
struct-audit diff ./main-binary ./feature-binary
```

**Output Example**:
```
┌─────────────────────────────────────────────────────────────────┐
│ LAYOUT CHANGES DETECTED                                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ ❌ REGRESSION: my_app::Order                                    │
│    Size: 64 → 72 bytes (+8 bytes, +12.5%)                       │
│    Padding: 6 → 14 bytes (+8 bytes)                             │
│    Cache Lines: 1 → 2 (CRITICAL: now spans cache line!)         │
│                                                                  │
│    Changes:                                                      │
│    + Added field: debug_info (String, 24 bytes at offset 48)    │
│                                                                  │
│ ✅ IMPROVED: my_app::Tick                                       │
│    Size: 48 → 40 bytes (-8 bytes, -16.7%)                       │
│    Padding: 12 → 4 bytes (-8 bytes)                             │
│                                                                  │
│ ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ │
│ SUMMARY: 2 changed, 1 regression, 1 improvement                 │
│ Total padding delta: +0 bytes                                   │
└─────────────────────────────────────────────────────────────────┘
```

**Acceptance Criteria**:
- [x] Matches structs by name
- [x] Detects added/removed structs
- [x] Detects member changes (added, removed, offset/size/type changed)
- [x] Calculates size and padding deltas
- [x] Returns non-zero exit code on regression (`--fail-on-regression`)
- [x] Supports JSON output for CI parsing
- [ ] *(Future)* Flags cache line boundary changes

---

### 3.3 CI Mode (`struct-audit check`)

**User Story**: As a tech lead, I want to fail CI builds when struct layouts exceed defined budgets so that regressions are caught automatically.

**Configuration** (`.struct-audit.yaml`):
```yaml
budgets:
  Order:
    max_size: 64
    max_padding: 8
    max_padding_percent: 15.0

  CriticalData:
    max_size: 128
    max_padding_percent: 10.0
```

**Command**:
```bash
struct-audit check ./target/release/my_app --config .struct-audit.yaml
```

**Output** (failure):
```
Budget violations:
  Order: size 72 exceeds budget 64 (+8 bytes)
  Order: padding 19.4% exceeds budget 15.0% (+4.4 percentage points)

Error: Budget check failed: 2 violation(s)
```

**Acceptance Criteria**:
- [x] Reads configuration from YAML file
- [x] Supports exact struct name matching
- [x] Enforces size budgets (`max_size`)
- [x] Enforces padding byte budgets (`max_padding`)
- [x] Enforces padding percentage budgets (`max_padding_percent`)
- [x] Validates budget values (rejects negative %, >100%, zero size)
- [x] Returns exit code 1 on budget violation
- [ ] *(Future)* Glob pattern matching for budget names
- [ ] *(Future)* `max_cache_lines` budget
- [ ] *(Future)* `--baseline` flag for diff-based checks

---

### 3.4 Report Generation (`struct-audit report`) — *Future*

> **Status**: Planned for Phase 3/SaaS integration. Not yet implemented.

**User Story**: As a developer, I want to generate a JSON report of all struct layouts so that I can upload it to the SaaS platform.

**Command**:
```bash
struct-audit report ./target/release/my_app --output report.json
```

**Acceptance Criteria**:
- [ ] Generates JSON conforming to schema (see API spec)
- [ ] Includes build metadata (commit SHA, timestamp, compiler)
- [ ] Compresses output for efficient upload
- [ ] Supports `--upload` flag for direct SaaS submission

---

### 3.5 Optimization Suggestions (`struct-audit suggest`) — *Future*

> **Status**: Planned for Phase 3. Not yet implemented.

**User Story**: As a developer, I want to see suggestions for improving struct layout so that I can reduce padding without manual analysis.

**Command**:
```bash
struct-audit suggest ./target/release/my_app --struct "my_app::Order"
```

**Output**:
```
┌─────────────────────────────────────────────────────────────────┐
│ OPTIMIZATION SUGGESTIONS: my_app::Order                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│ Current: 72 bytes (14 bytes padding, 19.4%)                     │
│ Optimal: 58 bytes (0 bytes padding, 0%)                         │
│ Savings: 14 bytes per instance (19.4%)                          │
│                                                                  │
│ Suggested field order:                                           │
│                                                                  │
│   1. id: u64           (8 bytes, align 8)                       │
│   2. price: f64        (8 bytes, align 8)                       │
│   3. quantity: f64     (8 bytes, align 8)                       │
│   4. timestamp: u64    (8 bytes, align 8)                       │
│   5. symbol: String    (24 bytes, align 8)                      │
│   6. is_active: bool   (1 byte, align 1)                        │
│   7. side: u8          (1 byte, align 1)                        │
│                                                                  │
│ ⚠️  Note: Reordering may affect serialization compatibility      │
└─────────────────────────────────────────────────────────────────┘
```

**Acceptance Criteria**:
- [ ] Solves bin-packing problem for optimal ordering
- [ ] Shows before/after comparison
- [ ] Warns about potential compatibility issues
- [ ] Supports `--apply` to generate patch (future)

---

## 4. SaaS Platform Features

### 4.1 Dashboard Overview

**User Story**: As an engineering manager, I want a dashboard showing memory health across all projects so that I can track trends and identify issues.

**Features**:
- Total padding bytes trend (line chart)
- Top 10 "worst offenders" (structs with most waste)
- Recent regressions (last 7 days)
- Project health score (0-100)

**Mockup**:
```
┌─────────────────────────────────────────────────────────────────┐
│ 📊 struct-audit Dashboard                    [Project ▼] [7d ▼] │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Health Score: 78/100 ████████████████████░░░░░                 │
│                                                                  │
│  ┌─────────────────────────────┐  ┌─────────────────────────┐  │
│  │ Total Padding (7d)          │  │ Top Offenders           │  │
│  │                             │  │                         │  │
│  │   1.2MB ─────────╮         │  │ 1. Order      14 bytes  │  │
│  │   1.1MB          │         │  │ 2. User       12 bytes  │  │
│  │   1.0MB ─────────╯         │  │ 3. Session    10 bytes  │  │
│  │        M  T  W  T  F       │  │ 4. Config      8 bytes  │  │
│  └─────────────────────────────┘  └─────────────────────────┘  │
│                                                                  │
│  Recent Regressions:                                            │
│  ┌────────────────────────────────────────────────────────────┐ │
│  │ ❌ PR #142: Order grew from 64→72 bytes (cache line!)      │ │
│  │ ⚠️  PR #139: Added 3 new structs with >20% padding          │ │
│  └────────────────────────────────────────────────────────────┘ │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

### 4.2 Struct History View

**User Story**: As a tech lead, I want to see the history of a specific struct over time so that I can identify when regressions were introduced.

**Features**:
- Size over time (sparkline)
- Commit-by-commit changes
- Blame: who changed what
- Annotations for significant events

---

### 4.3 PR Integration

**User Story**: As a developer, I want struct-audit to comment on my PRs with layout changes so that reviewers can see the impact.

**GitHub PR Comment**:
```markdown
## 📐 struct-audit Report

### Layout Changes Detected

| Struct | Size Change | Padding | Cache Lines |
|--------|-------------|---------|-------------|
| `Order` | 64 → 72 (+12.5%) | 6 → 14 | 1 → 2 ⚠️ |
| `Tick` | 48 → 40 (-16.7%) | 12 → 4 | 1 → 1 ✅ |

### ⚠️ Warning
`Order` now spans **2 cache lines** (was 1). This may impact hot-path performance.

---
[View full report](https://app.struct-audit.io/reports/abc123)
```

---

### 4.4 Budget Configuration (Web UI)

**User Story**: As a tech lead, I want to configure budgets via the web UI so that I don't need to commit YAML files.

**Features**:
- Visual budget editor
- Sync to `.struct-audit.yaml` in repo
- Override hierarchy (repo < org < global)

---

## 5. Non-Functional Requirements

### 5.1 Performance

| Metric | Target |
|--------|--------|
| CLI analysis of 100MB binary | < 5 seconds |
| CLI analysis of 1GB binary | < 30 seconds |
| Memory usage | < 2x binary size |
| SaaS API response time | < 200ms (p95) |

### 5.2 Compatibility

| Platform | Support Level |
|----------|---------------|
| Linux (x86_64, ARM64) | Full |
| macOS (x86_64, ARM64) | Full |
| Windows (x86_64) | Full |
| WASM binaries | Planned (v2) |

| Language | Support Level |
|----------|---------------|
| C | Full |
| C++ | Full (including templates) |
| Rust | Full |
| Go | Planned (v1.1) |

### 5.3 Security

- No source code uploaded (only layout metadata)
- Optional struct name hashing for IP protection
- SOC 2 compliance (Enterprise tier)
- Self-hosted option for air-gapped environments

---

## 6. Release Milestones

### v0.1.0 - Alpha (Phase 1)
- [ ] `inspect` command with basic output
- [ ] ELF binary support
- [ ] Padding detection

### v0.2.0 - Beta (Phase 2)
- [ ] `diff` command
- [ ] Mach-O and PE support
- [ ] `check` command with budgets
- [ ] JSON output

### v1.0.0 - GA (Phase 3)
- [ ] SaaS platform MVP
- [ ] GitHub App integration
- [ ] PR comments
- [ ] Historical tracking

### v1.1.0 - Enhanced
- [ ] `suggest` command
- [ ] Go language support
- [ ] False sharing detection

---

## Next Steps

→ [Technical Architecture](./04-technical-architecture.md) - System design and components

