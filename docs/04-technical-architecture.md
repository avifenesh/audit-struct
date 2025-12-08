# Technical Architecture

## System Design for CLI and SaaS Platform

---

## 1. Architecture Overview

The **struct-audit** platform follows a **distributed architecture** with clear separation between the analysis agent (CLI) and the intelligence platform (SaaS).

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         struct-audit Architecture                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                     Developer Workstation                        │   │
│  │  ┌─────────────┐                                                 │   │
│  │  │   Binary    │                                                 │   │
│  │  │  (ELF/PE/   │                                                 │   │
│  │  │  Mach-O)    │                                                 │   │
│  │  └──────┬──────┘                                                 │   │
│  │         │                                                         │   │
│  │         ▼                                                         │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │                  struct-audit CLI                        │    │   │
│  │  │  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐    │    │   │
│  │  │  │ Object  │─▶│  DWARF  │─▶│ Analysis│─▶│ Output  │    │    │   │
│  │  │  │ Loader  │  │ Parser  │  │ Engine  │  │ Format  │    │    │   │
│  │  │  └─────────┘  └─────────┘  └─────────┘  └─────────┘    │    │   │
│  │  └─────────────────────────────┬───────────────────────────┘    │   │
│  │                                │                                 │   │
│  └────────────────────────────────┼─────────────────────────────────┘   │
│                                   │ JSON Report                         │
│                                   ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                         CI/CD Pipeline                           │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │  GitHub Actions / GitLab CI / Jenkins                    │    │   │
│  │  │  • Run struct-audit check                                │    │   │
│  │  │  • Upload report to SaaS                                 │    │   │
│  │  │  • Receive pass/fail status                              │    │   │
│  │  └─────────────────────────────┬───────────────────────────┘    │   │
│  └────────────────────────────────┼─────────────────────────────────┘   │
│                                   │ HTTPS                               │
│                                   ▼                                     │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      SaaS Platform                               │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────┐  │   │
│  │  │   API       │  │  Database   │  │      Web Dashboard      │  │   │
│  │  │  Gateway    │──│ (Postgres)  │──│       (Next.js)         │  │   │
│  │  │  (Axum)     │  │             │  │                         │  │   │
│  │  └──────┬──────┘  └─────────────┘  └─────────────────────────┘  │   │
│  │         │                                                        │   │
│  │         ▼                                                        │   │
│  │  ┌─────────────────────────────────────────────────────────┐    │   │
│  │  │              GitHub/GitLab Integration                   │    │   │
│  │  │  • Webhook receiver                                      │    │   │
│  │  │  • PR comment poster                                     │    │   │
│  │  │  • Check status updater                                  │    │   │
│  │  └─────────────────────────────────────────────────────────┘    │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 2. CLI Architecture

### 2.1 Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                      struct-audit CLI                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Command Layer (clap)                   │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌─────────────────┐ │  │
│  │  │ inspect │ │  diff   │ │  check  │ │ report/suggest  │ │  │
│  │  └────┬────┘ └────┬────┘ └────┬────┘ └────────┬────────┘ │  │
│  └───────┼───────────┼───────────┼───────────────┼──────────┘  │
│          │           │           │               │              │
│          ▼           ▼           ▼               ▼              │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   Analysis Engine                         │  │
│  │  ┌────────────────────────────────────────────────────┐  │  │
│  │  │  StructAnalyzer                                     │  │  │
│  │  │  • calculate_padding()                              │  │  │
│  │  │  • detect_cache_boundaries()                        │  │  │
│  │  │  • compute_density()                                │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────────────┐  │  │
│  │  │  DiffEngine                                         │  │  │
│  │  │  • match_structs()                                  │  │  │
│  │  │  • compute_delta()                                  │  │  │
│  │  │  • detect_renames()                                 │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────────────┐  │  │
│  │  │  BudgetChecker                                      │  │  │
│  │  │  • load_config()                                    │  │  │
│  │  │  • evaluate_budgets()                               │  │  │
│  │  │  • generate_violations()                            │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    DWARF Parser Layer                     │  │
│  │  ┌────────────────────────────────────────────────────┐  │  │
│  │  │  DwarfContext (gimli wrapper)                       │  │  │
│  │  │  • iter_compilation_units()                         │  │  │
│  │  │  • find_struct_types()                              │  │  │
│  │  │  • resolve_type_chain()                             │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  │  ┌────────────────────────────────────────────────────┐  │  │
│  │  │  ExpressionEvaluator                                │  │  │
│  │  │  • evaluate_location()                              │  │  │
│  │  │  • handle_dwarf4_bitfield()                         │  │  │
│  │  │  • handle_dwarf5_bitfield()                         │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   Object Loader Layer                     │  │
│  │  ┌────────────────────────────────────────────────────┐  │  │
│  │  │  BinaryLoader (object crate)                        │  │  │
│  │  │  • load_elf()                                       │  │  │
│  │  │  • load_macho()                                     │  │  │
│  │  │  • load_pe()                                        │  │  │
│  │  │  • extract_debug_sections()                         │  │  │
│  │  └────────────────────────────────────────────────────┘  │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Output Formatters                      │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐ │  │
│  │  │  Table   │  │   JSON   │  │ Markdown │  │   HTML   │ │  │
│  │  │ (comfy)  │  │  (serde) │  │          │  │ (future) │ │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────┘ │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 2.2 Core Data Structures

```rust
/// Represents a parsed struct from DWARF
pub struct StructLayout {
    /// Fully qualified name (e.g., "my_app::Order")
    pub name: String,
    /// Total size in bytes
    pub size: u64,
    /// Alignment requirement
    pub alignment: u64,
    /// Source file location
    pub source_location: Option<SourceLocation>,
    /// Ordered list of members
    pub members: Vec<MemberLayout>,
    /// Computed metrics
    pub metrics: LayoutMetrics,
}

/// Represents a single field within a struct
pub struct MemberLayout {
    pub name: String,
    pub type_name: String,
    pub offset: u64,
    pub size: u64,
    pub alignment: u64,
    /// For bitfields
    pub bit_offset: Option<u64>,
    pub bit_size: Option<u64>,
}

/// Computed metrics for a struct layout
pub struct LayoutMetrics {
    /// Total padding bytes
    pub padding_bytes: u64,
    /// Padding as percentage of total size
    pub padding_percent: f64,
    /// Number of cache lines spanned
    pub cache_lines: u64,
    /// Data density (useful bytes / total cache line bytes)
    pub density: f64,
    /// List of padding holes
    pub holes: Vec<PaddingHole>,
    /// Members that straddle cache line boundaries
    pub cache_line_violations: Vec<CacheLineViolation>,
}

/// A gap in the struct layout
pub struct PaddingHole {
    pub offset: u64,
    pub size: u64,
    pub after_field: String,
}
```

### 2.3 Analysis Pipeline

```
┌─────────┐    ┌──────────────┐    ┌─────────┐    ┌──────────┐    ┌────────┐
│  Load   │───▶│ Contextualize│───▶│ Iterate │───▶│ Calculate│───▶│ Output │
│ Binary  │    │    DWARF     │    │  DIEs   │    │ Metrics  │    │ Format │
└─────────┘    └──────────────┘    └─────────┘    └──────────┘    └────────┘
     │               │                  │              │               │
     ▼               ▼                  ▼              ▼               ▼
 Memory-map     Initialize         Traverse       Sort members     Generate
 the file       gimli::Dwarf      CUs/DIEs       Detect gaps      table/JSON
                context           Filter by       Cache analysis
                                  struct type
```

**Step-by-Step**:

1. **Load**: Memory-map the target binary using `memmap2` to avoid loading the entire file into RAM
2. **Contextualize**: Initialize `gimli::Dwarf` context, loading auxiliary sections (`.debug_str`, `.debug_abbrev`)
3. **Iterate**: Traverse Compilation Units, looking for `DW_TAG_structure_type`
4. **Filter**: Apply user-defined filters (regex on struct names)
5. **Calculate**: For each struct:
   - Sort members by offset
   - Detect gaps between members
   - Check cache line boundaries
   - Compute density metrics
6. **Output**: Generate result in requested format

---

## 3. SaaS Platform Architecture

### 3.1 High-Level Design

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SaaS Platform                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        Load Balancer                             │   │
│  │                     (Cloudflare / AWS ALB)                       │   │
│  └────────────────────────────┬────────────────────────────────────┘   │
│                               │                                         │
│          ┌────────────────────┼────────────────────┐                   │
│          │                    │                    │                    │
│          ▼                    ▼                    ▼                    │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐             │
│  │  API Server  │    │  API Server  │    │  API Server  │             │
│  │   (Axum)     │    │   (Axum)     │    │   (Axum)     │             │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘             │
│         │                   │                   │                       │
│         └───────────────────┼───────────────────┘                       │
│                             │                                           │
│                             ▼                                           │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Message Queue                               │   │
│  │                    (Redis / RabbitMQ)                            │   │
│  └────────────────────────────┬────────────────────────────────────┘   │
│                               │                                         │
│          ┌────────────────────┼────────────────────┐                   │
│          │                    │                    │                    │
│          ▼                    ▼                    ▼                    │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐             │
│  │   Worker     │    │   Worker     │    │   Worker     │             │
│  │  (Analysis)  │    │  (GitHub)    │    │  (Alerts)    │             │
│  └──────────────┘    └──────────────┘    └──────────────┘             │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        Data Layer                                │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │   │
│  │  │  PostgreSQL  │  │    Redis     │  │     S3       │           │   │
│  │  │  (Primary)   │  │   (Cache)    │  │  (Reports)   │           │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Frontend (Next.js)                          │   │
│  │                    Deployed on Vercel/Render                     │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 Database Schema

```sql
-- Organizations (for team features)
CREATE TABLE organizations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) UNIQUE NOT NULL,
    plan VARCHAR(50) DEFAULT 'community',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Repositories
CREATE TABLE repositories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    organization_id UUID REFERENCES organizations(id),
    provider VARCHAR(50) NOT NULL, -- 'github', 'gitlab'
    provider_id VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    full_name VARCHAR(512) NOT NULL, -- 'owner/repo'
    default_branch VARCHAR(255) DEFAULT 'main',
    config JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(provider, provider_id)
);

-- Commits (snapshots)
CREATE TABLE commits (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID REFERENCES repositories(id) ON DELETE CASCADE,
    sha VARCHAR(40) NOT NULL,
    branch VARCHAR(255),
    author VARCHAR(255),
    message TEXT,
    build_metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(repository_id, sha)
);

-- Struct snapshots (deduplicated)
CREATE TABLE struct_layouts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Content-addressable hash for deduplication
    content_hash VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(512) NOT NULL,
    size BIGINT NOT NULL,
    alignment BIGINT NOT NULL,
    padding_bytes BIGINT NOT NULL,
    padding_percent DECIMAL(5,2) NOT NULL,
    cache_lines INTEGER NOT NULL,
    density DECIMAL(5,4) NOT NULL,
    -- Full layout stored as JSONB
    layout JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Junction table: commit <-> struct_layout
CREATE TABLE commit_structs (
    commit_id UUID REFERENCES commits(id) ON DELETE CASCADE,
    struct_layout_id UUID REFERENCES struct_layouts(id),
    PRIMARY KEY (commit_id, struct_layout_id)
);

-- Budgets configuration
CREATE TABLE budgets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    repository_id UUID REFERENCES repositories(id) ON DELETE CASCADE,
    pattern VARCHAR(512) NOT NULL, -- struct name or glob
    max_size BIGINT,
    max_padding_percent DECIMAL(5,2),
    max_cache_lines INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for common queries
CREATE INDEX idx_commits_repo_created ON commits(repository_id, created_at DESC);
CREATE INDEX idx_commit_structs_struct ON commit_structs(struct_layout_id);
CREATE INDEX idx_struct_layouts_name ON struct_layouts(name);
```

### 3.3 API Endpoints

```
┌─────────────────────────────────────────────────────────────────┐
│                        API Endpoints                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Authentication                                                  │
│  ├── POST   /auth/github          GitHub OAuth callback          │
│  ├── POST   /auth/gitlab          GitLab OAuth callback          │
│  └── DELETE /auth/logout          Logout                         │
│                                                                  │
│  Reports                                                         │
│  ├── POST   /api/v1/reports       Upload new report              │
│  ├── GET    /api/v1/reports/:id   Get report by ID               │
│  └── GET    /api/v1/reports       List reports (paginated)       │
│                                                                  │
│  Repositories                                                    │
│  ├── GET    /api/v1/repos         List connected repos           │
│  ├── GET    /api/v1/repos/:id     Get repo details               │
│  ├── PUT    /api/v1/repos/:id     Update repo config             │
│  └── GET    /api/v1/repos/:id/stats  Get repo statistics         │
│                                                                  │
│  Structs                                                         │
│  ├── GET    /api/v1/repos/:id/structs        List all structs    │
│  ├── GET    /api/v1/repos/:id/structs/:name  Get struct history  │
│  └── GET    /api/v1/repos/:id/structs/:name/timeline             │
│                                                                  │
│  Budgets                                                         │
│  ├── GET    /api/v1/repos/:id/budgets        List budgets        │
│  ├── POST   /api/v1/repos/:id/budgets        Create budget       │
│  ├── PUT    /api/v1/repos/:id/budgets/:bid   Update budget       │
│  └── DELETE /api/v1/repos/:id/budgets/:bid   Delete budget       │
│                                                                  │
│  Webhooks (GitHub/GitLab)                                        │
│  ├── POST   /webhooks/github      Handle GitHub events           │
│  └── POST   /webhooks/gitlab      Handle GitLab events           │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## 4. Technology Stack

### 4.1 CLI

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Language | **Rust** | Memory safety, performance, ecosystem |
| Binary Parsing | `object` | Unified ELF/Mach-O/PE interface |
| DWARF Parsing | `gimli` | Zero-copy, lazy evaluation |
| CLI Framework | `clap` | Industry standard, derive macros |
| Serialization | `serde` + `serde_json` | JSON output |
| Table Output | `comfy-table` | Beautiful terminal tables |
| Memory Mapping | `memmap2` | Efficient large file handling |
| Colored Output | `colored` | Terminal colors |
| Config Parsing | `serde_yaml` | YAML budget files |

### 4.2 SaaS Backend

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Language | **Rust** (Axum) | Type safety, performance |
| Web Framework | `axum` | Modern, async, tower ecosystem |
| Database | PostgreSQL | Relational + JSONB flexibility |
| Cache | Redis | Session storage, rate limiting |
| Object Storage | S3/R2 | Report artifact storage |
| Auth | GitHub/GitLab OAuth | Developer-friendly |
| Background Jobs | `tokio` + Redis | Async job processing |

### 4.3 Frontend

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Framework | **Next.js 14** | React, SSR, API routes |
| Styling | Tailwind CSS | Utility-first, rapid development |
| Charts | Recharts | React-native charting |
| State | React Query | Server state management |
| Auth | NextAuth.js | OAuth integration |

### 4.4 Infrastructure

| Component | Technology | Rationale |
|-----------|------------|-----------|
| Hosting | **Render** | Simple deployment, scaling |
| CDN | Cloudflare | Edge caching, DDoS protection |
| Monitoring | Sentry | Error tracking |
| CI/CD | GitHub Actions | Native integration |

---

## 5. Security Architecture

### 5.1 Data Flow Security

```
┌─────────────────────────────────────────────────────────────────┐
│                      Security Boundaries                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  CLI (User Machine)                                              │
│  ├── Binary never leaves machine                                 │
│  ├── Only metadata (struct names, sizes) uploaded                │
│  └── Optional: Hash struct names for IP protection               │
│                                                                  │
│  Transport                                                       │
│  ├── TLS 1.3 for all API calls                                   │
│  ├── API key authentication                                      │
│  └── Rate limiting per API key                                   │
│                                                                  │
│  SaaS Platform                                                   │
│  ├── Data encrypted at rest (AES-256)                           │
│  ├── Database connections over TLS                               │
│  ├── No source code stored                                       │
│  └── Audit logging for Enterprise                                │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 5.2 IP Protection Mode

For security-conscious customers (HFT, Defense):

```rust
// CLI can hash struct names before upload
pub struct HashingConfig {
    enabled: bool,
    salt: String, // Per-organization salt
}

// Original: "my_app::trading::AlphaStrategy"
// Hashed:   "struct_9a8b7c6d5e4f3a2b"

// Local mapping file for reverse lookup
// .struct-audit-mapping.json (gitignored)
{
    "struct_9a8b7c6d5e4f3a2b": "my_app::trading::AlphaStrategy"
}
```

---

## 6. Scalability Considerations

### 6.1 CLI Performance

| Scenario | Target | Strategy |
|----------|--------|----------|
| 100MB binary | < 5s | Memory mapping, lazy parsing |
| 1GB binary | < 30s | Parallel CU processing |
| 10GB binary | < 5min | Streaming, bounded memory |

### 6.2 SaaS Scaling

| Component | Scaling Strategy |
|-----------|------------------|
| API Servers | Horizontal (stateless) |
| Database | Read replicas, connection pooling |
| Background Jobs | Worker pool scaling |
| Storage | S3 (unlimited) |

---

## Next Steps

→ [DWARF Technical Deep Dive](./05-dwarf-technical-deep-dive.md) - Parsing implementation details


