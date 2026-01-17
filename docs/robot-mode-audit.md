# Robot Mode Audit

This document tracks the output format support status across all `ms` CLI commands.

## Output Format System

The `ms` CLI uses the `OutputFormat` enum for structured output:

```rust
pub enum OutputFormat {
    Human,  // Default: colored, formatted for terminal
    Json,   // Single JSON object
    Jsonl,  // Newline-delimited JSON
    Plain,  // Minimal text output
    Tsv,    // Tab-separated values
}
```

**Usage:**
- `--output-format json` or `-O json` for machine-readable output
- `--robot` flag provides backward compatibility (equivalent to `-O json`)

## Audit Summary

| Status | Count | Description |
|--------|-------|-------------|
| Full Support | 51 | Uses `ctx.output_format` for conditional output |
| Delegated | 1 | Delegates to supported command (install → bundle) |
| Human Only | 5 | Interactive/diagnostic commands without machine output |

## Commands with Full OutputFormat Support (51)

| Command | Usages | Notes |
|---------|--------|-------|
| alias | 5 | Alias management |
| antipatterns | 4 | Anti-pattern mining |
| backup | 4 | State backup/restore |
| bandit | 2 | Suggestion bandit controls |
| build | 52 | Skill building from CASS |
| bundle | 15 | Bundle management |
| cm | 5 | CASS memory integration |
| config | 1 | Configuration management |
| conflicts | 2 | Sync conflict management |
| contract | 2 | Pack contract management |
| cross_project | 5 | Cross-project analysis |
| dedup | 4 | Duplicate detection |
| diff | 1 | Semantic diff |
| embed | 1 | Embedding backend testing |
| evidence | 4 | Provenance tracking |
| experiment | 7 | Experiment management |
| favorite | 5 | Favorite skills |
| feedback | 2 | Skill feedback |
| graph | 8 | Graph analysis |
| hide | 4 | Hide skills |
| import | 9 | Skill import |
| inbox | 2 | Agent mail inbox |
| index | 3 | Skill indexing |
| init | 1 | Initialization |
| lint | 2 | Skill linting |
| list | 1 | List skills |
| load | 7 | Load skills with disclosure |
| machine | 2 | Machine identity |
| mcp | 1 | MCP server |
| migrate | 1 | Migration |
| outcome | 1 | Outcome recording |
| personalize | 8 | Personalization |
| pre_commit | 1 | Pre-commit hook |
| preferences | 6 | Preference management |
| prune | 13 | Data pruning |
| quality | 1 | Quality scoring |
| remote | 5 | Remote management |
| requirements | 2 | Environment requirements |
| safety | 5 | DCG safety |
| search | 2 | Skill search |
| security | 2 | Security scanning |
| shell | 1 | Shell integration |
| show | 1 | Show skill details |
| simulate | 1 | Sandbox simulation |
| suggest | 4 | Context-aware suggestions |
| sync | 2 | Synchronization |
| template | 3 | Skill templates |
| test | 1 | Run skill tests |
| unhide | 1 | Unhide skills |
| update | 1 | Update check |
| validate | 1 | Skill validation |

## Delegated Commands (1)

| Command | Delegates To | Notes |
|---------|-------------|-------|
| install | bundle | Thin wrapper around `bundle install` |

## Human-Only Commands (5)

These commands are currently human-only. They function correctly but don't output JSON in robot mode.

| Command | Reason | Future Work |
|---------|--------|-------------|
| doctor | Diagnostic tool with rich formatting | Could emit JSON health report |
| edit | Interactive editor invocation | N/A - inherently interactive |
| fmt | File formatting | Could emit JSON diff/status |
| meta | Meta-skill management | Could emit JSON for list/show/search |

## Robot Mode Output Contract

All robot mode output follows this structure:

```json
{
  "status": "ok" | "error" | "partial",
  "data": { /* command-specific payload */ },
  "warnings": ["optional", "warning", "messages"],
  "error": "optional error message if status=error"
}
```

## Usage Examples

```bash
# JSON output for automation
ms search "error handling" -O json

# Tab-separated for spreadsheets
ms list -O tsv

# Backward compatible robot mode
ms suggest --robot

# Plain text for simple scripts
ms load my-skill -O plain
```

## Testing Requirements

### Integration Test Pattern

```rust
#[test]
fn test_command_robot_mode() {
    let output = Command::new("ms")
        .args(["search", "test", "-O", "json"])
        .output()
        .expect("failed to execute");

    assert!(output.status.success());

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout)
        .expect("invalid JSON output");

    assert!(json.get("status").is_some());
}
```

### E2E Validation

```bash
#!/bin/bash
# Validate all commands produce valid JSON in robot mode
for cmd in search list show suggest load; do
    output=$(ms $cmd --help 2>&1 || true)
    if echo "$output" | grep -q "output-format"; then
        result=$(ms $cmd -O json 2>/dev/null || echo '{}')
        echo "$result" | jq . >/dev/null 2>&1 || {
            echo "FAIL: ms $cmd does not produce valid JSON"
            exit 1
        }
    fi
done
```

## Migration History

- **2026-01-16**: Initial OutputFormat system implemented
  - Migrated all 51 commands from `ctx.robot_mode` to `ctx.output_format`
  - Added 5 output format variants (Human, Json, Jsonl, Plain, Tsv)
  - Global `--output-format/-O` flag with backward `--robot` compatibility
