# Codemap: meta_skill (ms)

> Auto-generated code structure for AI agents
> **Updated**: 2026-01-23 | **Commit**: [`0024af0`](../../commit/0024af0)

---

## Core Modules

| Module | Purpose |
|--------|---------|
| `src/lib.rs` | Library root, re-exports |
| `src/main.rs` | CLI entry point (clap) |
| `src/config/` | Configuration (TOML, env vars, precedence) |
| `src/error.rs` | Error types (`MsError`, `Result`) |
| `src/skill/` | Skill parsing, storage, SKILL.md format |
| `src/search/` | Hybrid search (BM25 + hash embeddings + RRF) |
| `src/db/` | SQLite persistence layer |

---

## Feature Modules

| Module | Purpose |
|--------|---------|
| `src/agent_detection/` | Detect AI agents (Claude Code, Cursor, Codex, etc.) |
| `src/agent_mail/` | MCP-based agent messaging |
| `src/antipatterns/` | Failure pattern detection and mining |
| `src/auth/` | JFP Cloud authentication (device code flow) |
| `src/beads/` | Beads integration (issue tracking) |
| `src/bundler/` | Skill bundle creation, installation, signing |
| `src/cass/` | CASS integration (session search) |
| `src/graph/` | Dependency graph analysis (via bv) |
| `src/mcp/` | MCP server (expose skills as tools) |
| `src/security/` | ACIP (injection defense) + DCG (command safety) |
| `src/sync/` | Multi-machine synchronization |

---

## Key Types

### Skills
```rust
// src/skill/types.rs
pub struct Skill {
    pub id: String,
    pub name: String,
    pub description: String,
    pub sections: Vec<Section>,
    pub tags: Vec<String>,
    pub context: Option<ContextSpec>,
    // ...
}
```

### Search
```rust
// src/search/mod.rs
pub enum SearchType { Bm25, Semantic, Hybrid }
pub struct SearchResult { skill_id, score, matched_sections }
```

### Beads Integration
```rust
// src/beads/types.rs
pub struct Issue { id, title, status, priority, ... }
pub enum IssueStatus { Open, InProgress, Blocked, Done, ... }
```

---

## CLI Commands (src/main.rs)

| Command | Description |
|---------|-------------|
| `ms init` | Initialize .ms/ directory |
| `ms index` | Index skill paths |
| `ms search` | Hybrid search |
| `ms suggest` | Context-aware recommendations |
| `ms load` | Progressive disclosure loading |
| `ms show` | Full skill details |
| `ms graph` | Dependency analysis (via bv) |
| `ms mcp serve` | Start MCP server |
| `ms security` | ACIP prompt injection defense |
| `ms safety` | DCG command safety gates |
| `ms bundle` | Bundle operations |
| `ms sync` | Multi-machine sync |
| `ms doctor` | Health checks |

---

## Storage Layout

```
.ms/
├── ms.db           # SQLite (queries, metadata, FTS5)
├── archive/        # Git repository (audit trail)
├── index/          # Tantivy search index
├── backups/        # Snapshots
├── sync/           # Sync state
└── config.toml     # Local config
```

---

## Agent Integration Points

**MCP Server:**
```bash
ms mcp serve              # stdio transport
ms mcp serve --port 8080  # HTTP transport
```

**Robot mode (JSON output):**
```bash
ms search "query" --robot
ms load skill-name --robot
ms suggest --robot
```

**Key tools exposed via MCP:**
- `search` - Query skills
- `load` - Retrieve skill content
- `evidence` - Get provenance
- `list` - Enumerate skills
- `show` - Full details
- `doctor` - Health check

---

## External Dependencies

| Dependency | Purpose |
|------------|---------|
| `bv` (beads_viewer) | Graph analysis algorithms |
| `cass` | Session search |
| `cm` | CASS memory/playbook |
| `bd` | Beads issue tracking |
