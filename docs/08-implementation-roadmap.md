# Implementation Roadmap

## Phased Development Plan

> **Context**: Solo side project. No fixed calendar deadlines. Focus on shipping MVP before expanding scope.

---

## Overview

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      Implementation Phases                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  PHASE 1 (MVP)       PHASE 2            PHASE 3          PHASE 4        │
│  Core CLI            Enhanced CLI       Advanced         SaaS           │
│                                                          (Deferred)     │
│  ┌─────────┐        ┌─────────┐        ┌─────────┐      ┌─────────┐    │
│  │ ELF     │        │ Mach-O  │        │ False   │      │ Backend │    │
│  │ Loader  │───────▶│ PE      │───────▶│ Sharing │─ ─ ─▶│ API     │    │
│  │         │        │         │        │         │      │         │    │
│  │ Basic   │        │ Diff    │        │ Suggest │      │ GitHub  │    │
│  │ Inspect │        │ Check   │        │ Command │      │ App     │    │
│  └─────────┘        └─────────┘        └─────────┘      └─────────┘    │
│                                                                          │
│  v0.1.0             v0.2.0             v0.3.0           v1.0.0          │
│  Alpha              Beta               Release          (If needed)     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Phase 1: Core CLI MVP (v0.1.0-alpha)

### Objective

Ship `struct-audit inspect <binary>` for ELF binaries with accurate layout analysis and modern developer UX.

### Scope

**In Scope**:
- ELF binary support (Linux)
- Constant member offset parsing
- Basic type resolution (primitives, pointers, typedefs, arrays)
- Padding detection and visualization
- Cache line analysis
- Table and JSON output
- `inspect` command only

**Explicitly Out of Scope**:
- Mach-O/PE support
- Expression-based offsets (C++ virtual inheritance)
- DWARF 5 bitfields
- `diff` command
- `check` command
- SaaS

### Milestones

| # | Milestone | Deliverable |
|---|-----------|-------------|
| 1 | Project Setup | Cargo.toml, project structure, CI |
| 2 | Test Infrastructure | Test binary corpus, integration tests |
| 3 | Binary Loading | ELF loader with debug section extraction |
| 4 | DWARF Parsing | gimli context, struct finder, attribute extraction |
| 5 | Member Extraction | Constant offsets, basic type resolution |
| 6 | Analysis | Padding detection, cache line analysis |
| 7 | Output | Table formatter, JSON formatter |
| 8 | Polish | Error handling, README, release |

### Success Criteria

- [ ] Parses test fixtures without panics
- [ ] Output matches manual inspection for test structs
- [ ] Runs on real-world binary (e.g., your own Rust project)
- [ ] Clear error message when debug info missing

### Dependencies

```toml
[dependencies]
gimli = "0.28"
object = "0.32"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
memmap2 = "0.9"
comfy-table = "7"
colored = "2"
thiserror = "1"
anyhow = "1"
```

---

## Phase 2: Enhanced CLI (v0.2.0-beta)

### Objective

Handle edge cases, add comparison capabilities, enable CI integration.

### Prerequisites

- Phase 1 complete
- MVP used on at least one real project
- Feedback collected on missing features

### Scope

**New Features**:
- Mach-O loader (macOS)
- PE loader (Windows)
- Expression evaluator for complex member offsets
- DWARF 4/5 bitfield support
- `diff` command (compare two binaries)
- `check` command (budget enforcement)
- YAML configuration (`.struct-audit.yaml`)
- GitHub Action

### Milestones

| # | Milestone | Deliverable |
|---|-----------|-------------|
| 1 | Expression Evaluator | Handle DW_OP_* for C++ offsets |
| 2 | Bitfield Support | DWARF 4 and 5 bitfield parsing |
| 3 | Cross-Platform | Mach-O and PE loaders |
| 4 | Diff Command | Binary comparison with regression detection |
| 5 | CI Mode | Budget config, check command, exit codes |
| 6 | GitHub Action | action.yml, CI integration |

### Success Criteria

- [ ] Parses C++ binaries with virtual inheritance
- [ ] Diff output matches expected for test binaries
- [ ] CI mode correctly fails on budget violations
- [ ] GitHub Action works in sample repository
- [ ] Runs on Linux, macOS, Windows

---

## Phase 3: Advanced Analysis (v0.3.0)

### Objective

Add detection and optimization features for power users.

### Prerequisites

- Phase 2 complete
- Users requesting advanced features
- CLI stable and reliable

### Scope

**New Features**:
- False sharing detection (atomics on same cache line)
- `suggest` command (optimal field ordering)
- C++ inheritance visualization
- Go language support (experimental)

### Milestones

| # | Milestone | Deliverable |
|---|-----------|-------------|
| 1 | False Sharing | Detect atomics, warn on shared cache lines |
| 2 | Suggestions | Bin-packing algorithm, before/after comparison |
| 3 | Inheritance | C++ base class visualization |
| 4 | Go Support | Parse Go DWARF output |

---

## Phase 4: SaaS Platform (v1.0.0) - Deferred

> **Note**: Do not start until Prerequisites are met. See `docs/30-future-saas-vision.md` for archived plans.

### Prerequisites (All Required)

1. CLI v0.2.0+ is stable
2. CLI has real users (not just you)
3. Multiple users request historical tracking
4. You're willing to maintain hosted infrastructure

### Scope (When Ready)

- Backend API (Axum + PostgreSQL)
- GitHub/GitLab App integration
- Web dashboard (Next.js)
- PR comments
- Historical tracking
- Budget management UI

---

## Technical Decisions

### Why ELF First?

1. Most common target for systems programming
2. Best DWARF support (GCC, Clang)
3. Easiest to test (Linux CI runners)
4. Mach-O/PE can reuse most parsing logic

### Why Constant Offsets Only for MVP?

1. Covers 90%+ of real-world structs
2. Expression evaluation is complex and error-prone
3. Can mark "unknown" offsets without blocking release
4. Users can still see struct size, alignment, most members

### Why Test Infrastructure in Phase 1?

1. DWARF has many edge cases discovered via testing
2. Can't validate correctness without known-good fixtures
3. Prevents regressions during development
4. Enables validation against manually-inspected layouts

---

## Risk Mitigation

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| DWARF edge cases | High | Medium | Extensive test suite, graceful fallback |
| Large binary performance | Medium | High | Memory mapping, lazy parsing |
| Cross-platform issues | Medium | Medium | CI on all platforms (Phase 2) |

### Scope Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Scope creep | High | High | Strict phase boundaries, explicit non-goals |
| Premature SaaS | Medium | High | Deferred to Phase 4 with prerequisites |
| pahole comparison | Medium | Medium | Position as "complementary", not "replacement" |

---

## Definition of Done

### Per-Task

- Code compiles without warnings
- Tests pass (if applicable)
- No new clippy warnings
- Committed to branch

### Per-Phase

- All P0 tasks complete
- Tests passing
- README updated
- Version tagged
- Release notes written

---

## Version History

| Version | Phase | Description |
|---------|-------|-------------|
| v0.1.0-alpha | 1 | MVP: ELF + inspect command |
| v0.2.0-beta | 2 | Cross-platform + diff/check |
| v0.3.0 | 3 | Advanced analysis |
| v1.0.0 | 4 | SaaS (if needed) |

---

## Navigation

- [Task Breakdown](./11-task-breakdown.md) - Detailed implementation tasks
- [Technical Architecture](./04-technical-architecture.md) - System design
- [Future SaaS Vision](./30-future-saas-vision.md) - Deferred SaaS plans
