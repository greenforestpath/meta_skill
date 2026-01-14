# PLAN_TO_MAKE_METASKILL_CLI.md

> **Project Codename:** `ms` (meta_skill)
> **Architecture Pattern:** Follow `/data/projects/xf` exactly
> **Primary Innovation:** Mining CASS sessions to generate production-quality skills

---

## 0. Background Information (Self-Contained Context)

This section provides complete context for understanding this plan. Another LLM or human reader should be able to fully evaluate and improve this plan using only the information contained herein.

### 0.1 The AI Coding Agent Landscape (2025-2026)

AI coding agents are LLM-powered tools that autonomously write, modify, and debug code. Unlike simple code completion (GitHub Copilot circa 2022), modern agents can:

- Execute shell commands and observe output
- Read and write files across entire codebases
- Make multi-file changes to implement features
- Run tests, interpret failures, and fix issues
- Use external tools via function calling / tool use

**Major AI Coding Agents (as of early 2026):**

| Agent | Provider | Interface | Key Characteristics |
|-------|----------|-----------|---------------------|
| **Claude Code** | Anthropic | CLI (terminal) | Anthropic's official agentic CLI for Claude. Uses tool-use API. |
| **Codex CLI** | OpenAI | CLI | OpenAI's terminal agent, similar architecture to Claude Code |
| **Gemini CLI** | Google | CLI | Google's agent, supports Gemini models |
| **Cursor** | Cursor Inc | IDE | VS Code fork with deeply integrated AI |
| **Aider** | Open Source | CLI | Python-based, supports multiple models |
| **Continue** | Open Source | IDE Plugin | VS Code/JetBrains plugin |

These agents share a common interaction pattern:
1. User provides a natural language request
2. Agent reasons about the task
3. Agent uses tools (file read/write, bash, search) to accomplish the task
4. Agent presents results and awaits next instruction

**Session transcripts** are the complete logs of these interactions—every user message, agent response, tool call, and tool result. A single coding session might be 10,000-100,000+ tokens.

### 0.2 What Are Claude Code Skills?

**Skills** are Claude Code's mechanism for extending agent capabilities with domain-specific knowledge. A skill is a markdown file (conventionally `SKILL.md`) that gets injected into the agent's context when relevant.

**Why skills exist:** LLMs have general knowledge but lack specific knowledge about:
- Your company's coding conventions
- Your project's architecture decisions
- Specialized workflows (deployment, review processes)
- Tool-specific expertise (your CLI tools, your APIs)
- Lessons learned from past debugging sessions

**Skill file structure:**

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 11 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**SKILL.md anatomy:**

[Code block omitted: example block (lang='markdown').]
- Block length: 50 line(s).
- Block contains illustrative content referenced by the surrounding text.

**Conditional blocks:** The `::: block` syntax allows version-specific content. At load time, `ms` evaluates predicates against the project environment (package.json, Cargo.toml, etc.) and strips blocks whose conditions evaluate false. The agent never sees irrelevant version-specific content.

**How skills are loaded:** Claude Code discovers skills from configured paths (e.g., `~/.claude/skills/`, `.claude/skills/` in projects). When a user invokes a skill (explicitly or via auto-suggestion), its content is injected into the conversation context.

**The token budget problem:** Each skill consumes context window tokens. A 500-line skill might use 3,000-5,000 tokens. With context windows of 128K-200K tokens and complex codebases already consuming much of that, skill loading must be strategic.

### 0.3 What Is CASS (Coding Agent Session Search)?

**CASS** (Coding Agent Session Search) is a Rust CLI tool that indexes and searches across historical coding agent sessions from multiple tools. It solves the problem: "I know I solved this before, but I can't find where."

**What CASS indexes:**
- Claude Code sessions (`.jsonl` files in `~/.claude/projects/`)
- Codex CLI sessions
- Gemini CLI sessions
- Cursor sessions (exported)
- ChatGPT code-heavy conversations (exported)
- Custom agent transcripts

**Key CASS capabilities:**

[Code block omitted: shell command examples (12 lines).]
- Unique tools referenced: 1.
- Tool invoked: cass
- Commands illustrate typical workflows and integrations.

**Robot mode:** All CASS commands support `--robot` for machine-readable JSON output. This is critical for programmatic integration—ms will call CASS as a subprocess and parse its JSON output.

**CASS search technology:**
- **Lexical search:** Tantivy (Rust port of Lucene) for BM25 full-text search
- **Semantic search:** Hash-based embeddings (no ML model required)
- **Hybrid fusion:** Reciprocal Rank Fusion (RRF) combines both rankings

**Session structure:** A session is a sequence of messages:
[Code block omitted: JSON example payload/schema.]
- Block length: 4 line(s).
- Keys extracted: 4.
- Key: role
- Key: content
- Key: tool_calls
- Key: tool_call_id
- Example illustrates machine-readable output contract.

**Why CASS matters for ms:** CASS contains thousands of solved problems. When an agent successfully debugged an auth issue, that solution is in CASS. When a deployment workflow was refined over 10 sessions, the evolution is in CASS. ms can mine this to generate skills automatically.

### 0.4 What Is xf (X Archive Search)?

**xf** is a Rust CLI tool for searching personal X (Twitter) data archives. It's the architectural template for ms because it exemplifies the exact patterns we want:

**xf's technical stack:**
- **Language:** Rust (Edition 2024, ~23,000 LOC)
- **CLI framework:** clap with derive macros
- **Storage:** SQLite with WAL mode, FTS5 virtual tables
- **Search:** Tantivy BM25 + hash-based embeddings + RRF fusion
- **Async runtime:** Tokio
- **Output:** Human-readable by default, `--format json` for automation
- **Auto-update:** Self-updating binary via GitHub Releases

**Why follow xf exactly:**
1. **Proven patterns:** xf is battle-tested on real workloads
2. **Same author:** The ms author created xf, so patterns are familiar
3. **Same constraints:** Both are local-first CLI tools with search + indexing
4. **Code reuse:** Many modules can be adapted directly

**Key xf patterns to adopt:**

| Pattern | xf Implementation | ms Adaptation |
|---------|-------------------|---------------|
| Hybrid search | BM25 + hash embeddings + RRF | Same, for skill search |
| Hash embeddings | FNV-1a hash, 384 dimensions | Same, no ML dependency |
| SQLite storage | WAL mode, PRAGMA tuning | Same, for skill registry |
| Robot mode | `--format json` flag | `--robot` flag |
| Auto-update | GitHub Releases + SHA256 | Same mechanism |
| Indexing | Background thread, progress bar | Same UX |

**Hash embeddings explained:** Instead of using a neural network (BERT, etc.) to generate embeddings, xf uses a deterministic hash function:

1. Tokenize text into words
2. Hash each word with FNV-1a
3. Use hash bits to determine which embedding dimensions to increment/decrement
4. Normalize the resulting vector

This provides ~80-90% of neural embedding quality with zero dependencies, instant speed, and perfect reproducibility.

### 0.5 What Is mcp_agent_mail (Agent Mail)?

**Agent Mail** is a coordination system for multi-agent workflows. It provides:
- **Message passing:** Agents send messages to each other
- **File reservations:** Advisory locks to prevent edit conflicts
- **Project identity:** Agents know they're working on the same project

**The dual persistence pattern:** Agent Mail writes data to both SQLite (for queries) and Git (for auditability):

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 15 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Why ms adopts this:** Skills benefit from both:
- **SQLite:** Fast search, usage tracking, quality scores
- **Git:** Version history, collaborative editing, sync across machines

**Two-Phase Commit (2PC):** To prevent drift between SQLite and Git, ms uses a
lightweight two-phase commit for all write operations.

**File reservation pattern:** When an agent wants to edit a file, it requests a reservation:
[Code block omitted: shell command examples (2 lines).]
- Unique tools referenced: 1.
- Tool invoked: agent_mail
- Commands illustrate typical workflows and integrations.

ms can use similar reservations for skill editing to prevent conflicts.

### 0.6 What Is NTM (Named Tmux Manager)?

**NTM** is a Go CLI that transforms tmux into a multi-agent command center. It spawns and orchestrates multiple AI coding agents in parallel.

**Why NTM matters for ms:**
1. **Multi-agent skill loading:** When NTM spawns agents, each needs appropriate skills
2. **Skill coordination:** Multiple agents shouldn't redundantly load same skills
3. **Context rotation:** As agents exhaust context, skills must transfer to fresh agents

**NTM agent types:**
[Code block omitted: shell command examples (2 lines).]
- Unique tools referenced: 1.
- Tool invoked: ntm
- Commands illustrate typical workflows and integrations.

**Integration point:** ms should provide:
[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 2.
- Example: ms suggest (flags: --for-ntm)
- Example: ms suggest (flags: --for-ntm, --agents, --budget, --objective)
- Example: ms load (flags: --level)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 0.7 What Is BV (Beads Viewer) and the Beads System?

**Beads** is a lightweight issue/task tracking system designed for AI agent workflows. Unlike Jira/Linear, beads are:
- **File-based:** Stored in `.beads/` directory
- **Git-native:** Tracked in version control
- **Agent-friendly:** Simple enough for agents to read/write

**Bead structure:**
[Code block omitted: YAML example.]
- Block length: 10 line(s).
- Keys extracted: 9.
- Key: id
- Key: title
- Key: type
- Key: status
- Key: priority
- Key: created
- Key: assignee
- Key: blocks
- Key: blocked_by
- Example encodes structured test/spec or config data.

**BV (Beads Viewer)** is the CLI for interacting with beads:
[Code block omitted: shell command examples (5 lines).]
- Unique tools referenced: 1.
- Tool invoked: bd
- Commands illustrate typical workflows and integrations.

**Why this matters for ms:** Skills can be tracked as beads. A skill-building session could be:
[Code block omitted: ms CLI command examples (1 lines).]
- Unique ms commands: 1.
- Example: ms build (flags: --name, --from-cass)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 0.8 The Agent Flywheel Ecosystem

The **Agent Flywheel** is an integrated suite of tools that compound AI agent effectiveness:

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 40 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Other flywheel tools:**

| Tool | Purpose |
|------|---------|
| **CM** (Cass Memory) | Procedural memory—learns rules from session analysis |
| **DCG** (Destructive Command Guard) | Safety system blocking dangerous commands |
| **UBS** (Ultimate Bug Scanner) | Static analysis for catching bugs pre-commit |
| **RU** (Repo Updater) | Syncs GitHub repos, handles updates |
| **XF** (X Archive Search) | Twitter archive search (architectural template) |

### 0.9 Token Efficiency: Why It Matters

**The context window constraint:** LLMs have finite context windows:
- Claude 3.5 Sonnet: 200K tokens
- GPT-4 Turbo: 128K tokens
- Gemini 1.5 Pro: 2M tokens (but performance degrades at scale)

**A typical coding session consumes:**
- System prompt: 2,000-5,000 tokens
- Project context (AGENTS.md, etc.): 3,000-10,000 tokens
- Codebase reading: 10,000-50,000+ tokens
- Conversation history: Grows throughout session
- **Skills:** Variable, 500-5,000 tokens each

**Token density imperative:** Skills must maximize information per token:

| Bad (Verbose) | Good (Token-Dense) |
|---------------|-------------------|
| "When you are working with Git and you need to..." | "Git operations:" |
| "It's important to remember that..." | (Just state the fact) |
| "Here's an example of how you might..." | "Example:" |
| Repeating information across sections | Single source of truth |
| Generic advice applicable anywhere | Specific, actionable guidance |

**Progressive disclosure solves this:** Don't load entire 5,000-token skills when 500 tokens suffice:

| Level | Tokens | Content |
|-------|--------|---------|
| Minimal | ~100 | Name + one-line description |
| Overview | ~500 | + Section headings, key points |
| Standard | ~1,500 | + Main content, truncated examples |
| Full | Variable | Complete SKILL.md |
| Complete | Variable | + scripts/ + references/ |

### 0.10 Robot Mode: The AI-Native CLI Pattern

Every CLI tool in the flywheel ecosystem supports **robot mode**—machine-readable JSON output for programmatic consumption.

**Why robot mode is essential:**

1. **Agent tool use:** LLMs can reliably parse JSON, not human-formatted tables
2. **Composability:** Tools can call other tools and process results
3. **Stability:** JSON schemas are versioned; human output changes frequently
4. **Completeness:** JSON includes metadata humans wouldn't want to see

**Robot mode convention:**

[Code block omitted: ms CLI command examples (2 lines).]
- Unique ms commands: 1.
- Example: ms search (no flags shown)
- Example: ms search (flags: --robot)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Robot mode rules:**
- stdout = data only (valid JSON)
- stderr = diagnostics, progress, errors
- Exit code 0 = success, non-zero = failure
- Schema documented and versioned

### 0.11 The Problem ms Solves

**Current state (without ms):**

1. **Skill creation is manual:** Writing SKILL.md from scratch requires:
   - Remembering what patterns worked
   - Articulating tacit knowledge explicitly
   - Formatting correctly for Claude Code
   - Testing and iterating

2. **Skills are scattered:** Stored in various places:
   - `~/.claude/skills/`
   - `.claude/skills/` in projects
   - Random GitHub repos
   - Local directories

3. **No discovery:** No way to search skills by content or find related skills

4. **No sharing infrastructure:** Sharing skills requires manual file copying

5. **No learning loop:** Successful coding sessions don't automatically improve skills

**Future state (with ms):**

1. **Skill creation from history:**
   ```bash
   ms build --from-cass "how I debug memory leaks"
   # Mines sessions, extracts patterns, generates skill draft
   # Interactive refinement until satisfied
   # Published skill captures real-world patterns
   ```

2. **Unified registry:**
   ```bash
   ms index  # Discovers all skills from configured paths
   ms search "async error"  # Finds relevant skills instantly
   ```

3. **Context-aware suggestions:**
   ```bash
   ms suggest --cwd /data/projects/rust-api
   # Suggests skills based on project type, current files, recent commands
   ```

4. **Sharing infrastructure:**
   ```bash
   ms bundle create my-rust-skills --tags rust
   ms bundle publish my-rust-skills --repo user/skill-bundle
   # Others install with:
   ms bundle install user/skill-bundle
   ```

5. **Learning flywheel:**
   - Agents work → Sessions recorded by CASS
   - ms mines CASS → Skills generated
   - Skills loaded → Agents work better
   - Better work → Better sessions → Repeat

### 0.12 Key Terminology Reference

| Term | Definition |
|------|------------|
| **Skill** | A SKILL.md file with optional scripts/references that extends agent capabilities |
| **Session** | Complete log of agent-user interaction (tool calls, responses, etc.) |
| **CASS** | Coding Agent Session Search—indexes and searches sessions |
| **xf** | X archive search CLI—architectural template for ms |
| **Agent Mail** | Coordination system for multi-agent messaging and file reservations |
| **NTM** | Named Tmux Manager—spawns and orchestrates multiple agents |
| **BV/Beads** | Lightweight issue tracking designed for agent workflows |
| **Robot mode** | JSON output mode for programmatic tool consumption |
| **RRF** | Reciprocal Rank Fusion—combines multiple ranking signals |
| **Progressive disclosure** | Revealing skill content incrementally based on need |
| **Token density** | Information per token—high density = efficient skills |
| **Dual persistence** | Writing to both SQLite (queries) and Git (audit) |
| **Hash embeddings** | Deterministic embeddings without ML models |
| **FTS5** | SQLite's full-text search extension |
| **Tantivy** | Rust search library (Lucene port) for BM25 |

### 0.13 Reference Implementations

This plan draws from three production codebases:

**1. xf (X Archive Search)** — `/data/projects/xf`
- ~23,000 LOC Rust
- SQLite + Tantivy + hash embeddings
- Auto-update via GitHub Releases
- Robot mode JSON output
- *Contribution:* Overall architecture, search, CLI patterns

**2. CASS (Coding Agent Session Search)** — `/data/projects/coding_agent_session_search`
- Multi-agent session indexing
- Hybrid search with RRF fusion
- Robot mode API
- *Contribution:* Session mining, pattern extraction integration

**3. Agent Mail** — `/data/projects/mcp_agent_mail`
- SQLite + Git dual persistence
- File reservation system
- MCP protocol implementation
- *Contribution:* Dual persistence pattern, coordination patterns

### 0.14 Success Criteria for This Plan

This plan should enable:

1. **Complete understanding:** A reader unfamiliar with these tools can evaluate the plan
2. **Implementation guidance:** An LLM can implement ms following this plan
3. **Architecture decisions:** Trade-offs are documented with rationale
4. **Testable milestones:** Each phase has clear deliverables
5. **Extension points:** Future features have designated integration points

### 0.15 Real-World Motivating Example: The UI/UX Polish Session

This example from an actual session illustrates exactly what ms should extract and generalize:

[Code block omitted: example block (lang='n/a').]
- Block length: 16 line(s).
- Block contains illustrative content referenced by the surrounding text.

**What ms extracts from this session:**

| Specific Instance | Generalized Pattern | Skill Category |
|-------------------|---------------------|----------------|
| "Added handleScroll() call on mount" | Check initial state on component mount | React Patterns |
| "Added prefersReducedMotion check" | Always respect `prefers-reduced-motion` | Accessibility |
| "Added aria-hidden on decorative icons" | Decorative elements need aria-hidden | Accessibility |
| "transition: all is too broad" | Use specific transition properties | CSS Best Practices |
| "Deep codebase audit" command | Systematic audit workflow | Review Methodology |

**The transformation:**
1. **Specific:** "I fixed this exact bug in hero.tsx"
2. **General:** "When reviewing React/Next.js UIs, always check for these 8 categories of issues"
3. **Skill:** A complete "nextjs-ui-polish" skill with checklists, examples, and THE EXACT PROMPTS

---

## 1. Vision & Philosophy

### 1.1 The Core Insight

Skills are crystallized wisdom from successful coding sessions. Instead of manually writing skills from scratch, we can **mine existing session history** to extract patterns that actually worked. CASS already indexes thousands of coding sessions—ms transforms that raw material into polished, production-ready skills.

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 10 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 1.2 What ms Does

1. **Index** - Discover and catalog all skills across configured paths
2. **Load** - Progressive disclosure of skill content to agents
3. **Suggest** - Context-aware skill recommendations
4. **Bundle** - Package skills for sharing (GitHub, local, enterprise)
5. **Build** - The killer feature: generate skills from CASS sessions
6. **Maintain** - Auto-update, versioning, deprecation tracking

### 1.3 Design Principles

| Principle | Implementation |
|-----------|----------------|
| **Follow xf exactly** | Same Rust patterns, same crate choices, same file layout |
| **SQLite + Git dual persistence** | From mcp_agent_mail: structured data in SQLite, human-readable in Git |
| **Robot mode everywhere** | Every command has `--robot` JSON output for automation |
| **Progressive disclosure** | Skills reveal depth as needed, not all at once |
| **Idempotent operations** | Safe to run any command multiple times |
| **Offline-first** | Full functionality without network, sync when available |

---

## 2. Architecture Overview

### 2.1 High-Level Components

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 24 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 2.2 Data Flow

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 31 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 2.3 File Layout (Following xf Pattern)

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 81 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Runtime Artifacts:**
- `.ms/skillpack.bin` (or per-skill pack objects) caches parsed spec, slices,
  embeddings, and predicate analysis for low-latency loads.
- Markdown is compiled; runtime uses binary packs by default.

---

## 3. Core Data Models

### 3.1 Skill Structure

[Code block omitted: Rust example code (types/logic).]
- Block length: 351 line(s).
- Counts: 23 struct(s), 8 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 90 field(s).
- Struct: Skill
- Struct: SkillSpec
- Struct: SkillSectionSpec
- Struct: SpecLens
- Struct: BlockLens
- Struct: SkillMetadata
- Struct: SkillPolicy
- Struct: DeprecationInfo
- Struct: SkillTrigger
- Struct: ToolchainConstraint
- Struct: SkillRequirements
- Struct: ToolRequirement
- Struct: SkillAssets
- Struct: TestFile
- Struct: SkillSource
- Struct: SkillComputed
- Struct: SkillSlice
- Struct: SlicePredicate
- Struct: SkillSliceIndex
- Struct: SkillEvidenceIndex
- Struct: EvidenceRef
- Struct: EvidenceCoverage
- Struct: UncertaintyItem
- Enum: SkillBlockSpec
- Enum: Platform
- Enum: NetworkRequirement
- Enum: SkillLayer
- Enum: PredicateType
- Enum: VersionOp
- Enum: SliceType
- Enum: UncertaintyStatus
- Impl block: Default
- Function/method: default
- Function/method: default_required
- Field/key: id
- Field/key: metadata
- Field/key: body
- Field/key: assets
- Field/key: source
- Field/key: computed
- Field/key: evidence
- Field/key: format_version
- Field/key: sections
- Field/key: generated_at
- Field/key: title
- Field/key: level
- Field/key: blocks
- Field/key: block_id
- Field/key: section
- Field/key: block_type
- Field/key: byte_range
- Field/key: name
- Field/key: description
- Field/key: version
- Field/key: author
- Field/key: tags
- Field/key: aliases
- Field/key: requires
- Field/key: provides
- Field/key: triggers
- Field/key: priority
- Field/key: deprecated
- Field/key: toolchains
- Field/key: requirements
- Field/key: fixes
- Field/key: policies
- Field/key: pattern_type
- Field/key: pattern
- Field/key: severity
- Field/key: message
- Field/key: since
- Field/key: reason
- Field/key: replaced_by
- Field/key: sunset_at
- Field/key: trigger_type
- Field/key: boost
- Field/key: min_version
- Field/key: max_version
- Field/key: notes
- Field/key: platforms
- Field/key: tools
- Field/key: env
- Field/key: network
- Field/key: required
- Field/key: NetworkRequirement
- Field/key: scripts
- Field/key: references
- Field/key: tests
- Field/key: path
- Field/key: test_type
- Field/key: layer
- Field/key: git_remote
- Field/key: git_commit
- Field/key: modified_at
- Field/key: content_hash
- Field/key: token_count
- Field/key: disclosure_levels
- Field/key: quality_score
- Field/key: embedding
- Field/key: slices
- Field/key: slice_type
- Field/key: token_estimate
- Field/key: utility_score
- Field/key: coverage_group
- Field/key: condition
- Field/key: content
- Field/key: expr
- Field/key: predicate_type
- Field/key: rules
- Field/key: coverage
- Field/key: session_id
- Field/key: message_range
- Field/key: snippet_hash
- Field/key: excerpt
- Field/key: excerpt_path
- Field/key: confidence
- Field/key: captured_at
- Field/key: total_rules
- Field/key: rules_with_evidence
- Field/key: avg_confidence
- Field/key: pattern_candidate
- Field/key: suggested_queries
- Field/key: status
- Field/key: created_at
- Block defines core structures or algorithms referenced by surrounding text.

### 3.2 SQLite Schema

[Code block omitted: SQL schema snippet.]
- Block length: 225 line(s).
- Counts: 19 table(s), 10 index(es), 3 trigger(s).
- Table: skills
  - Column: id
  - Column: name
  - Column: description
  - Column: version
  - Column: author
  - Column: source_path
  - Column: source_layer
  - Column: git_remote
  - Column: git_commit
  - Column: content_hash
  - Column: body
  - Column: metadata_json
  - Column: assets_json
  - Column: token_count
  - Column: quality_score
  - Column: indexed_at
  - Column: modified_at
  - Column: is_deprecated
  - Column: deprecation_reason
- Table: skill_aliases
  - Column: alias
  - Column: skill_id
  - Column: alias_type
  - Column: created_at
- Table: skill_embeddings
  - Column: skill_id
  - Column: embedding
  - Column: created_at
- Table: skill_slices
  - Column: skill_id
  - Column: slices_json
  - Column: updated_at
- Table: skill_evidence
  - Column: skill_id
  - Column: rule_id
  - Column: evidence_json
  - Column: coverage_json
  - Column: updated_at
- Table: skill_rules
  - Column: skill_id
  - Column: rule_id
  - Column: strength
  - Column: updated_at
- Table: uncertainty_queue
  - Column: id
  - Column: pattern_json
  - Column: reason
  - Column: confidence
  - Column: suggested_queries
  - Column: status
  - Column: created_at
- Table: redaction_reports
  - Column: id
  - Column: session_id
  - Column: report_json
  - Column: created_at
- Table: injection_reports
  - Column: id
  - Column: session_id
  - Column: report_json
  - Column: created_at
- Table: skill_usage
  - Column: id
  - Column: skill_id
  - Column: project_path
  - Column: used_at
  - Column: disclosure_level
  - Column: context_keywords
  - Column: success_signal
  - Column: experiment_id
  - Column: variant_id
- Table: skill_usage_events
  - Column: id
  - Column: skill_id
  - Column: session_id
  - Column: loaded_at
  - Column: disclosure_level
  - Column: discovery_method
  - Column: experiment_id
  - Column: variant_id
  - Column: outcome
  - Column: feedback
- Table: rule_outcomes
  - Column: id
  - Column: skill_id
  - Column: rule_id
  - Column: session_id
  - Column: followed
  - Column: outcome
  - Column: created_at
- Table: skill_experiments
  - Column: id
  - Column: skill_id
  - Column: variants_json
  - Column: allocation_json
  - Column: status
  - Column: started_at
- Table: skill_dependencies
  - Column: skill_id
  - Column: depends_on
- Table: skill_capabilities
  - Column: capability
  - Column: skill_id
- Table: build_sessions
  - Column: id
  - Column: name
  - Column: status
  - Column: cass_queries
  - Column: patterns_json
  - Column: draft_skill_json
  - Column: skill_spec_json
  - Column: iteration_count
  - Column: last_feedback
  - Column: created_at
  - Column: updated_at
- Table: config
  - Column: key
  - Column: value
  - Column: updated_at
- Table: tx_log
  - Column: id
  - Column: entity_type
  - Column: entity_id
  - Column: phase
  - Column: payload_json
  - Column: created_at
- Table: cass_fingerprints
  - Column: session_id
  - Column: content_hash
  - Column: updated_at
- Index: idx_skill_aliases_skill
- Index: idx_evidence_skill
- Index: idx_uncertainty_status
- Index: idx_redaction_session
- Index: idx_injection_session
- Index: idx_skills_name
- Index: idx_skills_modified
- Index: idx_skills_quality
- Index: idx_usage_skill
- Index: idx_usage_time
- Trigger: skills_ai
- Trigger: skills_ad
- Trigger: skills_au

### 3.3 Git Archive Structure (Human-Readable Persistence)

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 45 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 3.4 Dependency Graph and Resolution

Skills declare dependencies (`requires`), capabilities (`provides`), and environment requirements
(platforms, tools, env vars) in metadata.
ms builds a dependency graph to resolve load order, detect cycles, and auto-load prerequisites.

[Code block omitted: Rust example code (types/logic).]
- Block length: 41 line(s).
- Counts: 5 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 11 field(s).
- Struct: DependencyGraph
- Struct: DependencyEdge
- Struct: ResolvedDependencyPlan
- Struct: SkillLoadPlan
- Struct: DependencyResolver
- Enum: DependencyLoadMode
- Impl block: DependencyResolver
- Function/method: resolve
- Field/key: nodes
- Field/key: edges
- Field/key: skill_id
- Field/key: depends_on
- Field/key: ordered
- Field/key: missing
- Field/key: cycles
- Field/key: disclosure
- Field/key: reason
- Field/key: registry
- Field/key: max_depth
- Block defines core structures or algorithms referenced by surrounding text.

Default behavior: `ms load` uses `DependencyLoadMode::Auto` (load dependencies
at `overview` disclosure, root skill at the requested level).

#### 3.4.1 Skill Aliases and Deprecation

Renames are inevitable. ms preserves backward compatibility by maintaining
alias mappings (old id → canonical id) and surfacing deprecations with explicit
replacements.

[Code block omitted: Rust example code (types/logic).]
- Block length: 17 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 4 field(s).
- Struct: AliasResolver
- Struct: AliasResolution
- Impl block: AliasResolver
- Function/method: resolve
- Field/key: db
- Field/key: canonical_id
- Field/key: alias_type
- Field/key: replaced_by
- Block defines core structures or algorithms referenced by surrounding text.

**Behavior:**
- `ms load legacy-id` resolves to canonical skill and emits a warning if deprecated.
- `ms search` and `ms suggest` exclude deprecated skills by default unless explicitly requested.
- If `deprecated.replaced_by` is set, ms highlights the replacement in output.
- Indexing upserts alias records from `metadata.aliases` and from `deprecated.replaced_by`
  (alias_type = `deprecated`), and rejects collisions with existing skill ids.

### 3.5 Layering and Conflict Resolution

Skills can exist in multiple layers. Higher layers override lower layers when
conflicts occur.

Layer order (default):
[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.

**Layered Skill Registry:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 45 line(s).
- Counts: 3 struct(s), 3 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 8 field(s).
- Struct: LayeredRegistry
- Struct: ResolvedSkill
- Struct: ConflictDetail
- Enum: ConflictStrategy
- Enum: MergeStrategy
- Enum: ConflictResolution
- Impl block: LayeredRegistry
- Function/method: effective
- Field/key: layers
- Field/key: registries
- Field/key: skill
- Field/key: conflicts
- Field/key: section
- Field/key: higher_layer
- Field/key: lower_layer
- Field/key: resolution
- Block defines core structures or algorithms referenced by surrounding text.

**Resolution Rules:**
- If only one layer provides a skill, use it directly.
- If multiple layers provide the same skill id:
  - Prefer higher layer by default
  - If both edit the same section, record a conflict detail
  - If conflict strategy is `interactive`, require explicit choice
  - If merge strategy is `prefer_sections`, keep higher-layer rules/pitfalls but
    append or preserve lower-layer examples/references when non-identical

**Conflict Auto-Diff and Merge Policies:**

To reduce manual resolution, ms computes section-level diffs and applies a
merge policy before falling back to interactive mode.

[Code block omitted: Rust example code (types/logic).]
- Block length: 40 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 4 fn(s), 7 field(s).
- Struct: ConflictMerger
- Impl block: ConflictMerger
- Function/method: resolve
- Function/method: section_diff
- Function/method: merge_sections
- Function/method: merge_by_section_preference
- Field/key: higher
- Field/key: lower
- Field/key: strategy
- Field/key: merge_strategy
- Field/key: skill
- Field/key: conflicts
- Field/key: ConflictStrategy
- Block defines core structures or algorithms referenced by surrounding text.

When conflicts remain, ms surfaces a guided diff in `ms resolve` showing the
exact section differences and suggested merges.

**Block-Level Overlays:**

Beyond whole-skill overrides, higher layers can provide **overlay files** that patch
specific block IDs without copying the entire skill. This enables surgical policy
additions and reduces duplication/drift.

[Code block omitted: Rust example code (types/logic).]
- Block length: 56 line(s).
- Counts: 1 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 4 field(s).
- Struct: SkillOverlay
- Enum: OverlayOp
- Impl block: LayeredRegistry
- Function/method: apply_overlays
- Field/key: skill_id
- Field/key: layer
- Field/key: operations
- Field/key: OverlayOp
- Block defines core structures or algorithms referenced by surrounding text.

**Overlay File Format:**

Overlays are stored in the layer's skill directory as `skill.overlay.json`:

[Code block omitted: JSON example payload/schema.]
- Block length: 23 line(s).
- Keys extracted: 8.
- Key: skill_id
- Key: operations
- Key: type
- Key: block_id
- Key: content
- Key: items
- Key: rule
- Key: id
- Example illustrates machine-readable output contract.

**Benefits:**

- **No duplication:** Org/user layers don't copy entire skills
- **Drift prevention:** Base skill updates propagate automatically
- **Surgical policy:** Add compliance rules without rewriting
- **Clear provenance:** Each block records which layer modified it

### 3.6 Skill Spec and Deterministic Compilation

SKILL.md is a rendered artifact. The source-of-truth is a structured `SkillSpec`
that can be deterministically compiled into SKILL.md. This ensures reproducible
output, stable diffs, and safe automated edits.

[Code block omitted: Rust example code (types/logic).]
- Block length: 15 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 0 field(s).
- Struct: SkillCompiler
- Impl block: SkillCompiler
- Function/method: compile
- Function/method: validate
- Block defines core structures or algorithms referenced by surrounding text.

By default, `ms build` outputs `skill.spec.json`, then compiles it to SKILL.md.
Manual edits should update the spec, not the rendered artifact.

**Spec-Only Editing (Hard Invariant):**
- `SkillSpec` is the *only* editable source of truth.
- Direct edits to SKILL.md are detected and blocked by default.
- A “repair/import” flow can ingest Markdown diffs into spec with warnings, but
  remains opt-in and records a provenance note.
- This guarantees stable block IDs, predicates, slice mapping, and semantic diffs.

**Multi-Target Compilation (Agent Adapters):**
- `ms compile --target claude|openai|cursor|generic-md`
- Same `SkillSpec`, different renderers/frontmatter, optional tool-call hints.
- Prevents ecosystem forks and makes skills portable across agent stacks.

**Semantic Diff Everywhere:**
- `ms diff --semantic` becomes the default view for review/merge/resolve.
- `ms review <skill>` groups changes by rule type (critical, pitfalls, examples).
- Bundle updates and conflict resolution show *meaning changes*, not formatting.

**Runtime Skillpack Cache:**
- On `ms index`, emit `.ms/skillpack.bin` (or per-skill pack objects) containing
  parsed spec, pre-tokenized slices, embeddings, predicate pre-analysis, and
  provenance pointers.
- `ms load/suggest` reads skillpack first for low-latency and consistent behavior.

**Round-Trip Editing (Spec ↔ Markdown):**
- `ms edit <skill>` opens a structured view, parses edits back into `SkillSpec`,
  and re-renders `SKILL.md` deterministically.
- The compiler emits `spec.lens.json` to map block IDs to byte ranges so edits
  can be attributed to the correct spec blocks.
- If parsing fails, `--allow-lossy` permits a best-effort import with warnings.
- `ms fmt` re-renders from spec; `ms diff --semantic` compares spec blocks.

### 3.7 Two-Phase Commit for Dual Persistence

All writes that touch both SQLite and Git are wrapped in a lightweight two-phase
commit to avoid split-brain states.

[Code block omitted: Rust example code (types/logic).]
- Block length: 27 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 9 field(s).
- Struct: TxManager
- Struct: TxRecord
- Impl block: TxManager
- Function/method: write_skill
- Field/key: db
- Field/key: git
- Field/key: tx_dir
- Field/key: id
- Field/key: entity_type
- Field/key: entity_id
- Field/key: phase
- Field/key: payload_json
- Field/key: created_at
- Block defines core structures or algorithms referenced by surrounding text.

Recovery is automatic on startup and via `ms doctor --fix`.

**2PC Hardening:**
- Tx IDs are monotonic (timestamp + counter) for deterministic recovery.
- Phase files written via atomic rename; directory entries fsynced where supported.
- Recovery uses a deterministic state machine and never “guesses” payloads.

**Index Invariants + Self-Repair (Non-Destructive):**
- `ms doctor --fix` can rehydrate missing DB rows from Git, recompute embeddings/slices,
  and mark stale artifacts with tombstones (never delete).
- Lossy repairs emit beads for human review.

### 3.7.1 Global File Locking

While SQLite handles internal concurrency with WAL mode, the dual-persistence
pattern (SQLite + Git) requires coordination when multiple `ms` processes run
concurrently (e.g., parallel agent invocations, IDE background indexer + CLI).

**Optional Single-Writer Daemon (`msd`):**
- Holds hot indices/caches in memory and serializes all write operations.
- CLI becomes a thin client when daemon is running (lower p95 latency).
- Reduces lock contention and simplifies crash recovery in swarm scenarios.

[Code block omitted: Rust example code (types/logic).]
- Block length: 102 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 4 fn(s), 3 field(s).
- Struct: GlobalLock
- Impl block: GlobalLock
- Impl block: Drop
- Function/method: acquire
- Function/method: try_acquire
- Function/method: acquire_timeout
- Function/method: drop
- Field/key: lock_file
- Field/key: lock_path
- Field/key: std
- Block defines core structures or algorithms referenced by surrounding text.

**Locked TxManager:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 18 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 0 field(s).
- Impl block: TxManager
- Function/method: write_skill_locked
- Function/method: write_skills_batch
- Block defines core structures or algorithms referenced by surrounding text.

**Lock behavior by command:**

| Command | Lock Type | Rationale |
|---------|-----------|-----------|
| `ms index` | Exclusive | Bulk writes to both stores |
| `ms load` | None (read-only) | SQLite WAL handles read concurrency |
| `ms search` | None (read-only) | FTS queries are read-only |
| `ms suggest` | None (read-only) | Query-only operation |
| `ms edit` | Exclusive | Modifies SKILL.md and SQLite |
| `ms mine` | Exclusive | Writes new skills |
| `ms calibrate` | Exclusive | Updates rule strengths |
| `ms doctor --fix` | Exclusive | May modify both stores |

**Diagnostics:**

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 2.
- Example: ms doctor (flags: --check-lock)
- Example: ms doctor (flags: --break-lock)
- Example: ms lock (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

The lock file includes a JSON payload with holder PID and timestamp, enabling
stale lock detection (process no longer running) and diagnostics.

---

## 4. CLI Command Reference

### 4.1 Core Commands

[Code block omitted: ms CLI command examples (60 lines).]
- Unique ms commands: 14.
- Example: ms init (no flags shown)
- Example: ms init (flags: --global)
- Example: ms index (no flags shown)
- Example: ms index (flags: --path)
- Example: ms index (flags: --all)
- Example: ms index (flags: --watch)
- Example: ms index (flags: --cass-incremental)
- Example: ms search (no flags shown)
- Example: ms search (flags: --limit)
- Example: ms search (flags: --tags)
- Example: ms search (flags: --min-quality)
- Example: ms search (flags: --include-deprecated)
- Example: ms search (flags: --layer)
- Example: ms load (no flags shown)
- Example: ms load (flags: --level)
- Example: ms load (flags: --level)
- Example: ms load (flags: --level)
- Example: ms load (flags: --full)
- Example: ms load (flags: --pack)
- Example: ms load (flags: --pack, --mode)
- Example: ms load (flags: --pack, --mode, --max-per-group)
- Example: ms load (flags: --deps)
- Example: ms load (flags: --deps)
- Example: ms load (flags: --deps)
- Example: ms load (flags: --robot)
- Example: ms suggest (no flags shown)
- Example: ms suggest (flags: --cwd)
- Example: ms suggest (flags: --file)
- Example: ms suggest (flags: --query)
- Example: ms suggest (flags: --pack)
- Example: ms suggest (flags: --explain)
- Example: ms suggest (flags: --pack, --mode, --max-per-group)
- Example: ms suggest (flags: --include-deprecated)
- Example: ms suggest (flags: --for-ntm, --agents, --budget, --objective)
- Example: ms show (no flags shown)
- Example: ms show (flags: --usage)
- Example: ms show (flags: --deps)
- Example: ms show (flags: --layer)
- Example: ms edit (no flags shown)
- Example: ms edit (flags: --allow-lossy)
- Example: ms fmt (no flags shown)
- Example: ms diff (flags: --semantic)
- Example: ms alias (no flags shown)
- Example: ms alias (no flags shown)
- Example: ms alias (no flags shown)
- Example: ms alias (no flags shown)
- Example: ms requirements (no flags shown)
- Example: ms requirements (flags: --project)
- Example: ms requirements (flags: --robot)
- Example: ms deps (no flags shown)
- Example: ms deps (flags: --graph, --format)
- Example: ms resolve (no flags shown)
- Example: ms resolve (flags: --strategy)
- Example: ms resolve (flags: --diff)
- Example: ms resolve (flags: --merge-strategy)
- Example: ms evidence (no flags shown)
- Example: ms evidence (flags: --rule)
- Example: ms evidence (flags: --graph)
- Example: ms evidence (flags: --timeline)
- Example: ms evidence (flags: --open)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 4.2 Build Commands (CASS Integration)

[Code block omitted: ms CLI command examples (19 lines).]
- Unique ms commands: 4.
- Example: ms build (no flags shown)
- Example: ms build (flags: --name)
- Example: ms build (flags: --from-cass)
- Example: ms build (flags: --from-cass, --sessions)
- Example: ms build (flags: --from-cass, --redaction-report)
- Example: ms build (flags: --from-cass, --no-redact)
- Example: ms build (flags: --from-cass, --no-antipatterns)
- Example: ms build (flags: --from-cass, --output-spec)
- Example: ms build (flags: --from-cass, --min-session-quality)
- Example: ms build (flags: --from-cass, --no-injection-filter)
- Example: ms build (flags: --from-cass, --generalize)
- Example: ms build (flags: --from-cass, --generalize, --llm-critique)
- Example: ms build (flags: --resume)
- Example: ms build (flags: --auto, --from-cass, --min-confidence)
- Example: ms compile (flags: --out)
- Example: ms spec (no flags shown)
- Example: ms build (flags: --resolve-uncertainties)
- Example: ms uncertainties (no flags shown)
- Example: ms uncertainties (flags: --mine)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 4.3 Bundle Commands

[Code block omitted: ms CLI command examples (13 lines).]
- Unique ms commands: 1.
- Example: ms bundle (flags: --skills)
- Example: ms bundle (flags: --tags, --min-quality)
- Example: ms bundle (flags: --repo)
- Example: ms bundle (flags: --gist)
- Example: ms bundle (flags: --sign, --key)
- Example: ms bundle (no flags shown)
- Example: ms bundle (flags: --skills)
- Example: ms bundle (flags: --channel, --verify)
- Example: ms bundle (no flags shown)
- Example: ms bundle (no flags shown)
- Example: ms bundle (no flags shown)
- Example: ms bundle (flags: --channel)
- Example: ms bundle (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 4.4 Maintenance Commands

[Code block omitted: ms CLI command examples (28 lines).]
- Unique ms commands: 8.
- Example: ms update (flags: --check)
- Example: ms update (no flags shown)
- Example: ms doctor (no flags shown)
- Example: ms doctor (flags: --fix)
- Example: ms doctor (flags: --check)
- Example: ms doctor (flags: --check)
- Example: ms doctor (flags: --check)
- Example: ms prune (flags: --scope)
- Example: ms prune (flags: --scope, --dry-run)
- Example: ms prune (flags: --scope, --min-uses, --window)
- Example: ms prune (flags: --scope, --similarity, --emit-beads)
- Example: ms prune (flags: --scope, --apply, --require-confirmation)
- Example: ms config (no flags shown)
- Example: ms config (no flags shown)
- Example: ms config (no flags shown)
- Example: ms config (no flags shown)
- Example: ms stats (no flags shown)
- Example: ms stats (flags: --skill)
- Example: ms stats (flags: --period)
- Example: ms stale (no flags shown)
- Example: ms stale (flags: --project)

**New Ops-Grade Maintenance Additions:**
- `ms doctor --preflight <context>` returns a small “safe to proceed” JSON blob.
- `ms load <skill> --pack --preview` shows exact injected slices + token counts.
- `ms compile --target claude|openai|cursor|generic-md`
- Optional `msd` daemon: `msd start|stop|status` with CLI auto-detect.
- Perf telemetry in `ms doctor --check=perf` (p50/p95/p99 suggest/load, pack time).
- Example: ms stale (flags: --min-severity)
- Example: ms test (no flags shown)
- Example: ms test (flags: --report)
- Example: ms test (flags: --all)
- Example: ms simulate (no flags shown)
- Example: ms simulate (flags: --project)
- Example: ms simulate (flags: --report)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 4.5 Robot Mode (Comprehensive Specification)

Following the xf pattern exactly, robot mode provides machine-readable JSON output for all operations. This enables tight integration with orchestration tools (NTM, BV) and other agents.

**Core Protocol:**

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 18 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Robot Mode Commands:**

[Code block omitted: ms CLI command examples (15 lines).]
- Unique ms commands: 15.
- Example: ms --robot-status (no flags shown)
- Example: ms --robot-health (no flags shown)
- Example: ms --robot-suggest (no flags shown)
- Example: ms --robot-search="query" (no flags shown)
- Example: ms --robot-build-status (no flags shown)
- Example: ms --robot-cass-status (no flags shown)
- Example: ms list (flags: --robot)
- Example: ms search (flags: --robot)
- Example: ms show (flags: --robot)
- Example: ms load (flags: --robot)
- Example: ms suggest (flags: --robot)
- Example: ms build (flags: --robot, --status)
- Example: ms stats (flags: --robot)
- Example: ms doctor (flags: --robot)
- Example: ms sync (flags: --robot)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Output Schemas:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 121 line(s).
- Counts: 13 struct(s), 1 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 61 field(s).
- Struct: RobotResponse
- Struct: StatusResponse
- Struct: RegistryStatus
- Struct: SuggestResponse
- Struct: SuggestionItem
- Struct: SuggestionExplain
- Struct: SuggestionExplanation
- Struct: SuggestionSignalExplain
- Struct: SignalScore
- Struct: RrfBreakdown
- Struct: BuildStatusResponse
- Struct: RequirementsResponse
- Struct: BuildSessionDetail
- Enum: RobotStatus
- Field/key: status
- Field/key: timestamp
- Field/key: version
- Field/key: data
- Field/key: warnings
- Field/key: registry
- Field/key: search_index
- Field/key: cass_integration
- Field/key: active_builds
- Field/key: config
- Field/key: total_skills
- Field/key: indexed_skills
- Field/key: local_skills
- Field/key: upstream_skills
- Field/key: modified_skills
- Field/key: last_index_update
- Field/key: context
- Field/key: suggestions
- Field/key: swarm_plan
- Field/key: explain
- Field/key: skill_id
- Field/key: name
- Field/key: score
- Field/key: reason
- Field/key: disclosure_level
- Field/key: token_estimate
- Field/key: pack_budget
- Field/key: packed_token_estimate
- Field/key: slice_count
- Field/key: dependencies
- Field/key: layer
- Field/key: conflicts
- Field/key: requirements
- Field/key: explanation
- Field/key: enabled
- Field/key: signals
- Field/key: matched_triggers
- Field/key: signal_scores
- Field/key: rrf_components
- Field/key: signal_type
- Field/key: value
- Field/key: weight
- Field/key: signal
- Field/key: contribution
- Field/key: bm25_rank
- Field/key: vector_rank
- Field/key: rrf_score
- Field/key: active_sessions
- Field/key: recent_completed
- Field/key: queued_patterns
- Field/key: queued_uncertainties
- Field/key: environment
- Field/key: session_id
- Field/key: skill_name
- Field/key: state
- Field/key: iteration
- Field/key: patterns_used
- Field/key: patterns_available
- Field/key: started_at
- Field/key: last_activity
- Field/key: checkpoint_path
- Block defines core structures or algorithms referenced by surrounding text.

**Error Response Format:**

[Code block omitted: JSON example payload/schema.]
- Block length: 12 line(s).
- Keys extracted: 8.
- Key: status
- Key: error
- Key: code
- Key: message
- Key: timestamp
- Key: version
- Key: data
- Key: warnings
- Example illustrates machine-readable output contract.

**Integration Examples:**

[Code block omitted: ms CLI command examples (2 lines).]
- Unique ms commands: 2.
- Example: ms build (flags: --robot, --from-cass, --auto, --max-iterations)
- Example: ms --robot-health (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 4.6 Doctor Command

The `doctor` command performs comprehensive health checks on the ms installation, following best practices from xf and other Rust CLI tools.

[Code block omitted: ms CLI command examples (4 lines).]
- Unique ms commands: 1.
- Example: ms doctor (no flags shown)
- Example: ms doctor (flags: --fix)
- Example: ms doctor (flags: --robot)
- Example: ms doctor (flags: --check)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Check Categories:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 38 line(s).
- Counts: 2 struct(s), 2 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 10 field(s).
- Struct: DoctorReport
- Struct: CheckResult
- Enum: CheckCategory
- Enum: HealthStatus
- Field/key: checks
- Field/key: overall_status
- Field/key: auto_fixable
- Field/key: check_id
- Field/key: category
- Field/key: status
- Field/key: message
- Field/key: details
- Field/key: fix_available
- Field/key: fix_command
- Block defines core structures or algorithms referenced by surrounding text.

**Checks Performed:**

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 94 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Output Example:**

[Code block omitted: ms CLI command examples (1 lines).]
- Unique ms commands: 1.
- Example: ms doctor (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 4.7 Shell Integration

Shell integration provides aliases, completions, and environment setup.

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 1.
- Example: ms init (no flags shown)
- Example: ms init (no flags shown)
- Example: ms init (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Generated Shell Functions:**

[Code block omitted: ms CLI command examples (2 lines).]
- Unique ms commands: 2.
- Example: ms load (flags: --level)
- Example: ms build (flags: --from-cass, --robot, --limit)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Shell Completions:**

[Code block omitted: shell command examples (117 lines).]
- Unique tools referenced: 82.
- Tool invoked: _ms()
- Tool invoked: local
- Tool invoked: commands=(
- Tool invoked: 'search:Search
- Tool invoked: 'list:List
- Tool invoked: 'show:Show
- Tool invoked: 'alias:Manage
- Tool invoked: 'requirements:Check
- Tool invoked: 'edit:Edit
- Tool invoked: 'fmt:Format
- Tool invoked: 'diff:Diff
- Tool invoked: 'prune:Prune
- Tool invoked: 'load:Load
- Tool invoked: 'suggest:Get
- Tool invoked: 'build:Build
- Tool invoked: 'bundle:Manage
- Tool invoked: 'sync:Synchronize
- Tool invoked: 'doctor:Health
- Tool invoked: 'stats:Usage
- Tool invoked: 'config:Configuration
- Tool invoked: 'upgrade:Check
- Tool invoked: )
- Tool invoked: global_opts=(
- Tool invoked: '--robot[Output
- Tool invoked: '--help[Show
- Tool invoked: '--version[Show
- Tool invoked: '--verbose[Verbose
- Tool invoked: '--quiet[Suppress
- Tool invoked: _arguments
- Tool invoked: $global_opts
- Tool invoked: '1:command:->command'
- Tool invoked: '*::arg:->args'
- Tool invoked: case
- Tool invoked: command)
- Tool invoked: _describe
- Tool invoked: ;;
- Tool invoked: args)
- Tool invoked: search)
- Tool invoked: '--limit[Max
- Tool invoked: '--tags[Filter
- Tool invoked: '--type[Filter
- Tool invoked: '--include-deprecated[Include
- Tool invoked: alias)
- Tool invoked: '1:action:(list
- Tool invoked: '*:skill:_ms_skills'
- Tool invoked: requirements)
- Tool invoked: '--project[Project
- Tool invoked: suggest)
- Tool invoked: '--cwd[Working
- Tool invoked: '--file[Current
- Tool invoked: '--query[Explicit
- Tool invoked: '--pack[Token
- Tool invoked: '--mode[Pack
- Tool invoked: '--max-per-group[Max
- Tool invoked: '--explain[Include
- Tool invoked: '--for-ntm[Swarm-aware
- Tool invoked: '--agents[Agent
- Tool invoked: '--budget[Token
- Tool invoked: '--objective[Swarm
- Tool invoked: load)
- Tool invoked: '--level[Disclosure
- Tool invoked: '--format[Output
- Tool invoked: edit)
- Tool invoked: '--allow-lossy[Allow
- Tool invoked: fmt)
- Tool invoked: '--check[Check
- Tool invoked: diff)
- Tool invoked: '--semantic[Spec-level
- Tool invoked: prune)
- Tool invoked: '--scope[Prune
- Tool invoked: '--approve[Verbatim
- Tool invoked: build)
- Tool invoked: '--from-cass[Mine
- Tool invoked: '--from-sessions[Specific
- Tool invoked: '--name[Skill
- Tool invoked: '--auto[Non-interactive
- Tool invoked: '--iterations[Max
- Tool invoked: esac
- Tool invoked: }
- Tool invoked: _ms_skills()
- Tool invoked: skills=(${(f)"$(ms
- Tool invoked: compdef
- Commands illustrate typical workflows and integrations.

### 4.8 MCP Server Mode

Beyond CLI, ms provides a **Model Context Protocol (MCP) server** for native agent
tool-use integration. This eliminates subprocess overhead, PATH issues, JSON parsing
brittleness, and platform differences.

**Why MCP matters:** CLI + JSON parsing works but is brittle. MCP is the native
interface for agent tool calling. Every modern agent (Claude Code, Codex CLI, Cursor)
can consume ms via MCP with dramatically less friction.

**Server Commands:**

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 1.
- Example: ms mcp (no flags shown)
- Example: ms mcp (flags: --tcp)
- Example: ms mcp (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**MCP Tool Definitions:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 49 line(s).
- Counts: 6 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 12 field(s).
- Struct: MsSearch
- Struct: MsSuggest
- Struct: MsLoad
- Struct: MsEvidence
- Struct: MsBuildStatus
- Struct: MsPack
- Field/key: query
- Field/key: filters
- Field/key: limit
- Field/key: context
- Field/key: budget_tokens
- Field/key: skill_id
- Field/key: pack_budget
- Field/key: level
- Field/key: rule_id
- Field/key: expand_context
- Field/key: skill_ids
- Field/key: mode
- Block defines core structures or algorithms referenced by surrounding text.

**Server Architecture:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 26 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 4 field(s).
- Struct: McpServer
- Impl block: McpServer
- Function/method: serve_stdio
- Function/method: serve_tcp
- Field/key: registry
- Field/key: cache
- Field/key: protocol
- Field/key: tokio
- Block defines core structures or algorithms referenced by surrounding text.

**Benefits over CLI:**

| Aspect | CLI Mode | MCP Mode |
|--------|----------|----------|
| Latency | ~50-100ms subprocess | ~1-5ms in-process |
| Caching | Per-invocation | Shared across requests |
| Streaming | Not supported | Partial results supported |
| Error handling | Exit codes + stderr | Structured error responses |
| Type safety | JSON schema drift risk | Schema-validated tools |

**Claude Code Integration:**

[Code block omitted: JSON example payload/schema.]
- Block length: 8 line(s).
- Keys extracted: 4.
- Key: ms
- Key: command
- Key: args
- Key: env
- Example illustrates machine-readable output contract.

---

## 5. CASS Integration Deep Dive

### 5.1 The Mining Pipeline

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 68 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Session Segmentation (Phase-Aware Mining):**
- Segment sessions into phases: recon → hypothesis → change → validation → regression fix → wrap-up.
- Use tool-call boundaries and language markers (“let’s try”, “run tests”, etc.) to classify phase.
- Extract phase-specific patterns to avoid overgeneralizing recon tactics into wrap-up rules.

### 5.2 Pattern Types

[Code block omitted: Rust example code (types/logic).]
- Block length: 68 line(s).
- Counts: 1 struct(s), 1 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 26 field(s).
- Struct: ExtractedPattern
- Enum: PatternType
- Field/key: commands
- Field/key: frequency
- Field/key: contexts
- Field/key: language
- Field/key: code

**Pattern IR (Typed Intermediate Representation):**
- Introduce typed IR before synthesis: `CommandRecipe`, `DiagnosticDecisionTree`,
  `Invariant`, `Pitfall`, `PromptMacro`, `RefactorPlaybook`, `ChecklistItem`.
- Normalize commands, filepaths, tool names, and error signatures for dedupe.
- IR drives confidence scoring, clustering, and deterministic synthesis.
- Field/key: purpose
- Field/key: text
- Field/key: variants
- Field/key: steps
- Field/key: decision_points
- Field/key: rule
- Field/key: severity
- Field/key: rationale
- Field/key: error_signature
- Field/key: resolution
- Field/key: prevention
- Field/key: prompt
- Field/key: context
- Field/key: effectiveness_score
- Field/key: bad_practice
- Field/key: risk
- Field/key: safer_alternative
- Field/key: id
- Field/key: pattern_type
- Field/key: evidence
- Field/key: confidence
- Block defines core structures or algorithms referenced by surrounding text.

### 5.3 CASS Client Implementation

[Code block omitted: Rust example code (types/logic).]
- Block length: 64 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 6 fn(s), 5 field(s).
- Struct: CassClient
- Struct: FingerprintCache
- Impl block: CassClient
- Impl block: FingerprintCache
- Function/method: search
- Function/method: get_session
- Function/method: incremental_sessions
- Function/method: capabilities
- Function/method: is_new_or_changed
- Function/method: update
- Field/key: cass_bin
- Field/key: data_dir
- Field/key: fingerprint_cache
- Field/key: serde_json
- Field/key: db
- Block defines core structures or algorithms referenced by surrounding text.

### 5.4 Interactive Build Session Flow

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 62 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 5.5 The Guided Iterative Mode (Hours-Long Autonomous Skill Generation)

This is a **killer feature**: ms can run autonomously for hours, systematically mining your session history to produce a comprehensive skill library tailored to YOUR approach.

**The Problem It Solves:**
- Manual skill creation is tedious and incomplete
- You've solved thousands of problems but captured none of them
- Your personal patterns and preferences aren't documented anywhere
- Starting from scratch means rediscovering solutions you already found

**The Vision:**

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 37 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Shared State Machine (Guided vs Autonomous):**
- Guided mode and autonomous mode share the same underlying state machine.
- Autonomous = guided mode with zero user input; guided = autonomous with checkpoints.
- One recovery path reduces drift and improves reliability.

**Steady-State Detection:**

From your planning-workflow skill, we adopt the "iterate until steady state" pattern:

[Code block omitted: Rust example code (types/logic).]
- Block length: 99 line(s).
- Counts: 1 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 11 field(s).
- Struct: SteadyStateDetector
- Enum: SteadyStateResult
- Impl block: SteadyStateDetector
- Function/method: is_steady
- Function/method: canonical_embedding
- Field/key: min_iterations
- Field/key: similarity_threshold
- Field/key: max_token_delta
- Field/key: max_quality_delta
- Field/key: min_evidence_coverage
- Field/key: max_no_improvement_iters
- Field/key: max_wall_clock_per_skill
- Field/key: reason
- Field/key: current
- Field/key: required
- Field/key: SteadyStateResult
- Block defines core structures or algorithms referenced by surrounding text.

**Autonomous Quality Rubric:**

The guided mode self-critiques each draft against this rubric:

| Criterion | Weight | Check |
|-----------|--------|-------|
| **Token Density** | 25% | Information per token exceeds threshold |
| **Actionability** | 25% | Contains concrete commands/code, not just advice |
| **Structure** | 20% | Has CRITICAL RULES, examples, troubleshooting |
| **Specificity** | 15% | References YOUR patterns, not generic wisdom |
| **Completeness** | 15% | Covers topic without obvious gaps |

**Interactive Checkpoints:**

Even in autonomous mode, ms pauses for user input at key moments:

[Code block omitted: Rust example code (types/logic).]
- Block length: 12 line(s).
- Counts: 0 struct(s), 1 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 0 field(s).
- Enum: CheckpointTrigger
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Interface:**

[Code block omitted: ms CLI command examples (5 lines).]
- Unique ms commands: 1.
- Example: ms build (flags: --guided, --duration)
- Example: ms build (flags: --guided, --focus, --duration)
- Example: ms build (flags: --guided, --resume)
- Example: ms build (flags: --guided, --autonomous, --duration)
- Example: ms build (flags: --guided, --dry-run, --duration)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 5.6 Specific-to-General Transformation Algorithm

This is the core intellectual innovation: extracting universal patterns ("inner truths") from specific instances.
The same pipeline is applied to counter-examples to produce "Avoid / When NOT to use" rules.

**The Transformation Pipeline:**

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 52 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Optional LLM-Assisted Refinement (Pluggable):**
- If configured, a local model critiques the candidate generalization for overreach,
  ambiguous scope, or missing counter-examples.
- Critique summaries are stored with the uncertainty item so humans can adjudicate.
- If no model is available, the pipeline remains heuristic-only.

**The Algorithm:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 132 line(s).
- Counts: 2 struct(s), 0 enum(s), 1 trait(s), 1 impl(s), 6 fn(s), 23 field(s).
- Struct: SpecificToGeneralTransformer
- Struct: RefinementCritique
- Trait: GeneralizationRefiner
- Impl block: SpecificToGeneralTransformer
- Function/method: critique
- Function/method: transform
- Function/method: extract_structure
- Function/method: find_similar_instances
- Function/method: extract_common_elements
- Function/method: queue_uncertainty
- Field/key: cass
- Field/key: embedder
- Field/key: uncertainty_queue
- Field/key: refiner
- Field/key: min_instances
- Field/key: confidence_threshold
- Field/key: summary
- Field/key: flags_overgeneralization
- Field/key: principle
- Field/key: examples
- Field/key: applicability
- Field/key: confidence
- Field/key: source_instances
- Field/key: abstracted_description
- Field/key: context_conditions
- Field/key: instance
- Field/key: validation
- Field/key: cluster
- Field/key: critique
- Field/key: id
- Field/key: pattern_candidate
- Field/key: status
- Field/key: created_at
- Block defines core structures or algorithms referenced by surrounding text.

**Generalization Confidence Scoring:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 65 line(s).
- Counts: 2 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 11 field(s).
- Struct: GeneralizationValidation
- Struct: CounterExample
- Enum: CounterExampleReason
- Impl block: GeneralizationValidation
- Function/method: compute
- Field/key: coverage
- Field/key: predictive_power
- Field/key: coherence
- Field/key: specificity
- Field/key: confidence
- Field/key: counterexamples
- Field/key: instance_id
- Field/key: failure_reason
- Field/key: missing_precondition
- Field/key: suggests_refinement
- Field/key: CounterExampleReason
- Block defines core structures or algorithms referenced by surrounding text.

### 5.7 Skill Deduplication and Personalization

**No Redundancy Across Skills:**

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 47 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Implementation:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 43 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 10 field(s).
- Struct: SkillDeduplicator
- Impl block: SkillDeduplicator
- Function/method: check_overlap
- Function/method: recommend_action
- Field/key: embedder
- Field/key: registry
- Field/key: semantic_threshold
- Field/key: uniqueness_threshold
- Field/key: existing_skill
- Field/key: semantic_similarity
- Field/key: structural_overlap
- Field/key: recommended_action
- Field/key: suggested_scopes
- Field/key: OverlapAction
- Block defines core structures or algorithms referenced by surrounding text.

**Personalization ("Tailored to YOUR Approach"):**

[Code block omitted: Rust example code (types/logic).]
- Block length: 50 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 9 field(s).
- Struct: PersonalizationEngine
- Struct: StyleProfile
- Impl block: PersonalizationEngine
- Function/method: build_from_sessions
- Function/method: personalize
- Field/key: style_profile
- Field/key: tool_preferences
- Field/key: naming_conventions
- Field/key: prompt_patterns
- Field/key: indentation
- Field/key: comment_style
- Field/key: error_handling
- Field/key: test_style
- Field/key: verbosity
- Block defines core structures or algorithms referenced by surrounding text.

### 5.8 Tech Stack Detection and Specialization

Different tech stacks require different skills. ms auto-detects your project's stack:

[Code block omitted: Rust example code (types/logic).]
- Block length: 78 line(s).
- Counts: 1 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 4 field(s).
- Struct: TechStackDetector
- Enum: TechStack
- Impl block: TechStackDetector
- Function/method: detect
- Function/method: suggest_for_stack
- Field/key: indicators
- Field/key: secondary
- Field/key: confidence
- Field/key: TechStack
- Block defines core structures or algorithms referenced by surrounding text.

**Toolchain Detection and Drift:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 55 line(s).
- Counts: 3 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 10 field(s).
- Struct: ProjectToolchain
- Struct: ToolchainDetector
- Struct: ToolchainMismatch
- Impl block: ToolchainDetector
- Function/method: detect
- Function/method: detect_toolchain_mismatches
- Field/key: node
- Field/key: rust
- Field/key: go
- Field/key: nextjs
- Field/key: react
- Field/key: tool
- Field/key: skill_range
- Field/key: project_version
- Field/key: skill
- Field/key: toolchain
- Block defines core structures or algorithms referenced by surrounding text.

**Stack-Specific Mining:**

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 1.
- Example: ms build (flags: --from-cass, --stack)
- Example: ms build (flags: --guided, --stack, --duration)
- Example: ms build (flags: --from-cass, --stack)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 5.9 The Meta Skill Concept

The **meta skill** is a special skill that guides AI agents in using `ms` itself. This creates a recursive self-improvement loop where agents use skills to build better skills.

#### The Core Insight

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 19 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

#### The Meta Skill Content

[Code block omitted: example block (lang='markdown').]
- Block length: 16 line(s).
- Block contains illustrative content referenced by the surrounding text.
# What topics have enough sessions for skill extraction?
ms coverage --min-sessions 5

# Find pattern clusters in session history
ms analyze --cluster --min-cluster-size 3

# What skills already exist?
ms list --format=coverage
[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.
# Guided interactive build (recommended)
ms build --guided --topic "UI/UX fixes"

# Single-shot extraction from recent sessions
ms build --from-cass "error handling" --since "7 days" --output draft.md

# Hours-long autonomous generation
ms build --guided --duration 4h --checkpoint-interval 30m
[Code block omitted: example block (lang='n/a').]
- Block length: 8 line(s).
- Block contains illustrative content referenced by the surrounding text.
# Add to your skill registry
ms add ./draft-skill/

# Update skill index
ms index --refresh

# Verify skill works
ms suggest "scenario that should trigger this skill"
[Code block omitted: example block (lang='n/a').]
- Block length: 7 line(s).
- Block contains illustrative content referenced by the surrounding text.
Specific Session Example           General Pattern
─────────────────────────────────────────────────────────
"Fixed aria-hidden on SVG" ────► "Decorative elements need aria-hidden"
"Added motion-reduce class" ────► "All animations need reduced-motion support"
"Changed transition-all" ────► "Use specific transition properties"
[Code block omitted: example block (lang='n/a').]
- Block length: 2 line(s).
- Block contains illustrative content referenced by the surrounding text.
# What topics have sessions but no skills?
ms coverage --show-gaps

# What skill categories are underrepresented?
ms stats --by-category

# Suggest next skill to build based on session frequency
ms next --suggest-build
[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.
# 1. I've done many UI/UX fix sessions recently
ms analyze --topic "UI fixes" --days 30
# Output: Found 23 sessions with 156 extractable patterns

# 2. Start guided build
ms build --guided --topic "UI/UX fixes" --stack nextjs-react

# 3. Interactive session begins...
# - ms presents pattern clusters
# - I approve/reject/refine each
# - Draft skill emerges

# 4. Validate and integrate
ms overlap ./draft-skill/  # Check for duplicates
ms validate ./draft-skill/ # Best practices check
ms add ./draft-skill/      # Add to registry
[Code block omitted: empty or placeholder block.]

#### Meta Skill Generation Algorithm

[Code block omitted: Rust example code (types/logic).]
- Block length: 60 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 14 field(s).
- Struct: MetaSkillGenerator
- Struct: MetaSkillMetrics
- Impl block: MetaSkillGenerator
- Function/method: analyze_meta_usage
- Function/method: self_improve
- Field/key: cass
- Field/key: ms_registry
- Field/key: meta_skill_version
- Field/key: total_uses
- Field/key: success_rate
- Field/key: common_errors
- Field/key: improvement_opportunities
- Field/key: content
- Field/key: improvements_made
- Field/key: confidence
- Field/key: skills_generated
- Field/key: avg_quality_score
- Field/key: guided_completion_rate
- Field/key: avg_time_to_skill
- Block defines core structures or algorithms referenced by surrounding text.

#### The Self-Improvement Loop

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 24 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**CLI Commands for Meta Skill:**

[Code block omitted: ms CLI command examples (5 lines).]
- Unique ms commands: 1.
- Example: ms meta (no flags shown)
- Example: ms meta (flags: --days)
- Example: ms meta (flags: --dry-run)
- Example: ms meta (flags: --apply)
- Example: ms meta (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 5.10 Long-Running Autonomous Generation with Checkpointing

The user's vision emphasizes hours-long autonomous skill generation sessions. This requires robust checkpointing, recovery, and progress tracking.

#### The Long-Running Session Problem

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 17 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

#### Checkpoint Architecture

[Code block omitted: Rust example code (types/logic).]
- Block length: 141 line(s).
- Counts: 3 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 41 field(s).
- Struct: CheckpointManager
- Struct: GenerationCheckpoint
- Struct: SkillInProgress
- Enum: GenerationPhase
- Impl block: CheckpointManager
- Function/method: save
- Function/method: load_latest
- Function/method: resume
- Field/key: checkpoint_dir
- Field/key: checkpoint_interval
- Field/key: max_checkpoints
- Field/key: id
- Field/key: build_id
- Field/key: sequence
- Field/key: created_at
- Field/key: phase
- Field/key: active_skills
- Field/key: completed_skills
- Field/key: pattern_pool
- Field/key: cass_state
- Field/key: metrics
- Field/key: feedback_history
- Field/key: processed_session_hashes
- Field/key: config_snapshot
- Field/key: algorithm_version
- Field/key: random_seed
- Field/key: queries_completed
- Field/key: queries_remaining
- Field/key: patterns_found
- Field/key: clusters_formed
- Field/key: current_cluster
- Field/key: skills_started
- Field/key: skills_completed
- Field/key: current_skill
- Field/key: iteration
- Field/key: last_delta
- Field/key: steady_state_approach
- Field/key: total_skills
- Field/key: total_duration
- Field/key: name
- Field/key: tech_stack
- Field/key: patterns_used
- Field/key: current_draft
- Field/key: quality_score
- Field/key: feedback
- Field/key: std
- Field/key: checkpoint
- Field/key: start_time
- Field/key: iterations_since_resume
- Block defines core structures or algorithms referenced by surrounding text.

#### Autonomous Generation Orchestrator

[Code block omitted: Rust example code (types/logic).]
- Block length: 157 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 3 fn(s), 22 field(s).
- Struct: AutonomousOrchestrator
- Struct: AutonomousConfig
- Impl block: Default
- Impl block: AutonomousOrchestrator
- Function/method: default
- Function/method: run
- Function/method: report_progress
- Field/key: cass
- Field/key: transformer
- Field/key: checkpoint_mgr
- Field/key: deduplicator
- Field/key: quality_scorer
- Field/key: config
- Field/key: max_duration
- Field/key: checkpoint_interval
- Field/key: progress_interval
- Field/key: max_iterations_per_skill
- Field/key: min_quality_threshold
- Field/key: parallel_skills
- Field/key: stall_timeout
- Field/key: topics
- Field/key: resume_build_id
- Field/key: GenerationPhase
- Field/key: duration
- Field/key: skills_generated
- Field/key: skills
- Field/key: patterns_discovered
- Field/key: patterns_used
- Field/key: checkpoints_created
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Commands:**

[Code block omitted: ms CLI command examples (9 lines).]
- Unique ms commands: 1.
- Example: ms build (flags: --autonomous, --topics, --duration)
- Example: ms build (flags: --resume)
- Example: ms build (flags: --resume-latest)
- Example: ms build (flags: --list-checkpoints)
- Example: ms build (flags: --show-checkpoint)
- Example: ms build (flags: --export-checkpoint, --output)
- Example: ms build (flags: --autonomous, --topics, --dry-run)
- Example: ms build (flags: --autonomous, --topics, --checkpoint-interval)
- Example: ms build (flags: --autonomous, --topics, --progress-interval)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Progress Output Example:**

[Code block omitted: example block (lang='n/a').]
- Block length: 38 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 5.11 Session Marking for Skill Mining

Allow users to mark sessions during or after completion as good candidates for skill extraction. This creates explicit training data for skill generation.

#### The Session Marking Problem

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 15 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

#### Marking Data Model

[Code block omitted: Rust example code (types/logic).]
- Block length: 148 line(s).
- Counts: 3 struct(s), 2 enum(s), 0 trait(s), 1 impl(s), 4 fn(s), 20 field(s).
- Struct: SessionMark
- Struct: SessionHighlight
- Struct: SessionMarkStore
- Enum: MarkType
- Enum: HighlightType
- Impl block: SessionMarkStore
- Function/method: mark
- Function/method: get_for_topic
- Function/method: get_exemplary
- Function/method: filter_for_cass_query
- Field/key: session_id
- Field/key: session_path
- Field/key: mark_type
- Field/key: topics
- Field/key: tech_stack
- Field/key: quality_rating
- Field/key: reason
- Field/key: marked_at
- Field/key: marked_by
- Field/key: highlights
- Field/key: id
- Field/key: start
- Field/key: end
- Field/key: highlight_type
- Field/key: confidence
- Field/key: pattern
- Field/key: related_ids
- Field/key: db
- Field/key: opts
- Field/key: TechStackDetector
- Block defines core structures or algorithms referenced by surrounding text.

#### CLI Commands for Session Marking

[Code block omitted: ms CLI command examples (12 lines).]
- Unique ms commands: 2.
- Example: ms mark (flags: --exemplary, --topics)
- Example: ms mark (flags: --useful, --topics, --reason)
- Example: ms mark (flags: --ignore, --reason)
- Example: ms mark (flags: --anti-pattern, --topics, --reason)
- Example: ms mark (flags: --exemplary, --quality, --topics)
- Example: ms mark (flags: --highlight, --reason)
- Example: ms marks (no flags shown)
- Example: ms marks (flags: --exemplary, --topic)
- Example: ms marks (no flags shown)
- Example: ms marks (no flags shown)
- Example: ms marks (no flags shown)
- Example: ms marks (flags: --output)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

#### Integration with Skill Building

[Code block omitted: Rust example code (types/logic).]
- Block length: 35 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 5 field(s).
- Struct: MarkedSessionBuilder
- Impl block: MarkedSessionBuilder
- Function/method: build_from_marked
- Field/key: cass
- Field/key: mark_store
- Field/key: transformer
- Field/key: topic
- Field/key: opts
- Block defines core structures or algorithms referenced by surrounding text.

**Example Workflow:**

[Code block omitted: shell command examples (17 lines).]
- Unique tools referenced: 8.
- Tool invoked: $
- Tool invoked: --reason
- Tool invoked: --quality
- Tool invoked: Exemplary
- Tool invoked: ★★★★★
- Tool invoked: ★★★★☆
- Tool invoked: Ignored
- Tool invoked: xyz789
- Commands illustrate typical workflows and integrations.

Anti-pattern markings are treated as counter-examples and flow into a dedicated
"Avoid / When NOT to use" section during draft generation.

### 5.12 Evidence and Provenance Graph

Evidence links are first-class: every rule in a generated skill should be traceable back
to concrete session evidence. ms builds a lightweight provenance graph that connects:

[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.

This makes skills auditable, merge-safe, and self-correcting.

**Provenance Compression (Pointer + Fetch):**
- Level 0: hash pointers + message ranges (cheap, default).
- Level 1: minimal redacted excerpt (N lines) for quick review.
- Level 2: expandable context fetched on demand via CASS.
- Keeps bundles light while remaining auditable.

**Provenance Graph Model:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 37 line(s).
- Counts: 5 struct(s), 1 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 15 field(s).
- Struct: ProvenanceGraph
- Struct: ProvNode
- Struct: ProvEdge
- Struct: EvidenceTimeline
- Struct: TimelineItem
- Enum: ProvNodeType
- Field/key: nodes
- Field/key: edges
- Field/key: id
- Field/key: node_type
- Field/key: label
- Field/key: from
- Field/key: to
- Field/key: weight
- Field/key: reason
- Field/key: rule_id
- Field/key: items
- Field/key: session_id
- Field/key: occurred_at
- Field/key: excerpt_path
- Field/key: confidence
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Examples:**

[Code block omitted: ms CLI command examples (5 lines).]
- Unique ms commands: 1.
- Example: ms evidence (no flags shown)
- Example: ms evidence (flags: --rule)
- Example: ms evidence (flags: --graph, --format)
- Example: ms evidence (flags: --timeline)
- Example: ms evidence (flags: --open)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Actionable Evidence Navigation:**

Provenance is only valuable if humans can quickly validate and refine rules.
ms provides direct jump-to-source workflows that call CASS to expand context.

[Code block omitted: Rust example code (types/logic).]
- Block length: 64 line(s).
- Counts: 3 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 13 field(s).
- Struct: EvidenceNavigator
- Struct: ExpandedEvidence
- Struct: ExpandedEvidenceItem
- Impl block: EvidenceNavigator
- Function/method: expand_evidence
- Function/method: cache_evidence
- Field/key: cass_client
- Field/key: evidence_cache
- Field/key: skill_id
- Field/key: rule_id
- Field/key: context_lines
- Field/key: session_id
- Field/key: message_range
- Field/key: context_before
- Field/key: matched_content
- Field/key: context_after
- Field/key: session_metadata
- Field/key: items
- Field/key: std
- Block defines core structures or algorithms referenced by surrounding text.

**Jump-to-Source CLI:**

[Code block omitted: ms CLI command examples (5 lines).]
- Unique ms commands: 1.
- Example: ms evidence (flags: --rule, --expand)
- Example: ms evidence (flags: --rule, --open-editor)
- Example: ms evidence (flags: --rule, --cass-info)
- Example: ms evidence (flags: --validate)
- Example: ms evidence (flags: --refresh-cache)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 5.13 Redaction and Privacy Guard

All CASS transcripts pass through a redaction pipeline before pattern extraction.
This prevents secrets, tokens, and PII from ever entering generated skills,
evidence excerpts, or provenance graphs.

**Redaction Report Model:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 39 line(s).
- Counts: 3 struct(s), 2 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 12 field(s).
- Struct: RedactionReport
- Struct: RedactionFinding
- Struct: RedactionLocation
- Enum: RedactionKind
- Enum: RedactionRisk
- Field/key: session_id
- Field/key: findings
- Field/key: redacted_tokens
- Field/key: risk_level
- Field/key: created_at
- Field/key: kind
- Field/key: matched_pattern
- Field/key: snippet_hash
- Field/key: location
- Field/key: message_index
- Field/key: byte_start
- Field/key: byte_end
- Block defines core structures or algorithms referenced by surrounding text.

**Redactor Interface:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 14 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 3 field(s).
- Struct: Redactor
- Impl block: Redactor
- Function/method: redact
- Field/key: rules
- Field/key: allowlist
- Field/key: min_entropy
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Examples:**

[Code block omitted: ms CLI command examples (2 lines).]
- Unique ms commands: 2.
- Example: ms doctor (flags: --check)
- Example: ms build (flags: --from-cass, --redaction-report)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Taint Tracking Through Mining Pipeline:**

Beyond binary redaction, ms tracks **taint labels** through the entire extraction →
clustering → synthesis pipeline. This ensures unsafe provenance never leaks into
high-leverage artifacts (prompts, rules, scripts).

**Structured Secret Types + Reassembly Resistance:**
- Classify secrets by type (API keys, tokens, emails, hostnames, filepaths, customer data).
- Assign stable secret IDs and prevent multiple partial excerpts from reconstructing
  the same secret across different evidence/rules.
- Enforce stricter policies for high-risk secret types and aggregate taint to descendants.

[Code block omitted: Rust example code (types/logic).]
- Block length: 69 line(s).
- Counts: 3 struct(s), 1 enum(s), 0 trait(s), 2 impl(s), 4 fn(s), 5 field(s).
- Struct: TaintSet
- Struct: TaintedSnippet
- Struct: TaintTracker
- Enum: TaintSource
- Impl block: TaintSet
- Impl block: TaintTracker
- Function/method: is_safe_for_prompt
- Function/method: is_safe_for_evidence
- Function/method: classify_message
- Function/method: propagate
- Field/key: sources
- Field/key: propagated_from
- Field/key: content
- Field/key: taint
- Field/key: source_location
- Block defines core structures or algorithms referenced by surrounding text.

**Taint Policy Enforcement:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 26 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 1 field(s).
- Struct: TaintPolicy
- Impl block: TaintPolicy
- Function/method: validate_block
- Field/key: BlockType
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Integration:**

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 3.
- Example: ms doctor (flags: --check)
- Example: ms evidence (flags: --show-taint)
- Example: ms build (flags: --from-cass, --strict-taint)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 5.14 Anti-Pattern Mining and Counter-Examples

Great skills include what *not* to do. ms extracts anti-patterns from failure
signals, marked anti-pattern sessions, and explicit “wrong” fixes in transcripts.
These are presented as a dedicated "Avoid / When NOT to use" section and sliced
as `Pitfall` blocks for token packing.

**Symmetric Counterexample Pipeline (First-Class):**
- Treat counterexamples as a full pipeline: extraction → clustering → synthesis → packing.
- Link each anti-pattern to the positive rule it constrains (conditionalization).
- Use counterexamples to tighten scope: “Do X *unless* Y, then do Z.”

**Anti-Pattern Extraction Sources:**
- Session marks with `MarkType::AntiPattern`
- Failure outcomes from the effectiveness loop
- Phrases indicating incorrect or insecure approaches

**Draft Integration (example):**

[Code block omitted: example block (lang='n/a').]
- Block length: 5 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 5.15 Active-Learning Uncertainty Queue

When generalization confidence is too low, ms does not discard the pattern. Instead,
it queues the candidate for targeted evidence gathering. This turns "maybe" patterns
into high-quality rules with minimal extra effort.

**Precision Loop (Active Learning):**
- For each uncertainty item, generate 3–7 targeted CASS queries (positive, negative, boundary).
- Auto-run when idle or via `ms uncertainties --mine`.
- Stop when confidence threshold is met or no further evidence can be found.

**Uncertainty Queue Flow:**

[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.

**Queue Interface:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 100 line(s).
- Counts: 3 struct(s), 2 enum(s), 0 trait(s), 1 impl(s), 4 fn(s), 18 field(s).
- Struct: UncertaintyItem
- Struct: DecisionBoundary
- Struct: UncertaintyQueue
- Enum: MissingSignal
- Enum: ResolutionCheck
- Impl block: UncertaintyQueue
- Function/method: enqueue
- Function/method: list_pending
- Function/method: resolve
- Function/method: check_resolution
- Field/key: id
- Field/key: pattern_candidate
- Field/key: reason
- Field/key: confidence
- Field/key: suggested_queries
- Field/key: status
- Field/key: created_at
- Field/key: decision_boundary
- Field/key: missing_signals
- Field/key: candidate_scope_refinement
- Field/key: positive_instances_needed
- Field/key: counterexample_would_discard
- Field/key: target_confidence
- Field/key: description
- Field/key: db
- Field/key: serde_json
- Field/key: ResolutionCheck
- Field/key: progress
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Examples:**

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 2.
- Example: ms uncertainties (no flags shown)
- Example: ms uncertainties (flags: --mine)
- Example: ms build (flags: --resolve-uncertainties)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 5.16 Session Quality Scoring

Not all sessions are equally useful. ms scores sessions for signal quality and
filters out low-quality transcripts before pattern extraction.

**Session Quality Model:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 34 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 3 field(s).
- Struct: SessionQuality
- Impl block: SessionQuality
- Function/method: compute
- Field/key: session_id
- Field/key: score
- Field/key: signals
- Block defines core structures or algorithms referenced by surrounding text.

**Usage:**
- Default threshold: `cass.min_session_quality`
- Use `--min-session-quality` to override per build
- Marked sessions (exemplary) get a quality bonus

---

### 5.17 Prompt Injection Defense

ms filters prompt-injection content before pattern extraction. Any session messages
that attempt to override system rules or instruct the agent to ignore constraints
are quarantined and excluded by default.

**Forensic Quarantine Playback:**
- Store snippet hash, minimal safe excerpt, triggered rule, and a replay command.
- Replay requires explicit user invocation to expand context from CASS.
- Enables tuning of injection filters without leaking unsafe text into skills.

**Injection Report Model:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 19 line(s).
- Counts: 2 struct(s), 1 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 7 field(s).
- Struct: InjectionReport
- Struct: InjectionFinding
- Enum: InjectionSeverity
- Field/key: session_id
- Field/key: findings
- Field/key: severity
- Field/key: created_at
- Field/key: pattern
- Field/key: message_index
- Field/key: snippet_hash
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Examples:**

[Code block omitted: ms CLI command examples (2 lines).]
- Unique ms commands: 2.
- Example: ms doctor (flags: --check)
- Example: ms build (flags: --from-cass, --no-injection-filter)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

---

### 5.18 Safety Invariant Layer (No Destructive Ops)

ms enforces a hard invariant: destructive filesystem or git operations are never
executed without explicit, verbatim approval. This mirrors the global agent
rules and prevents ms from becoming a footgun.

**Safety Policy Model:**

Safety classification is **effect-based**, not command-string-based. Rather than
pattern-matching on strings like `rm` or `git reset`, we classify by the semantic
effect of what the command does. This is more robust because:
- `rm -rf /` and `find . -delete` have the same effect (file deletion)
- A command with harmless flags (e.g., `rm -i`) is safer than one without
- Novel commands get correct classification based on what they do

**Non-Removable Policy Lenses:**
- Certain policies (e.g., AGENTS.md Rule 1) compile into `Policy` slices with
  `MandatoryPredicate::Always`.
- Packer fails closed if mandatory policy slices are omitted, even under tight budgets.

[Code block omitted: Rust example code (types/logic).]
- Block length: 67 line(s).
- Counts: 2 struct(s), 3 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 9 field(s).
- Struct: SafetyPolicy
- Struct: ApprovalRequest
- Enum: CommandEffect
- Enum: SafetyTier
- Enum: DestructiveOpsPolicy
- Impl block: CommandEffect
- Function/method: to_tier
- Field/key: CommandEffect
- Field/key: destructive_ops
- Field/key: require_verbatim_approval
- Field/key: tombstone_deletes
- Field/key: command
- Field/key: effect
- Field/key: tier
- Field/key: reason
- Field/key: approve_hint
- Block defines core structures or algorithms referenced by surrounding text.

**Behavior:**
- Destructive commands (delete/overwrite/reset) are blocked by default.
- In robot mode, ms returns `approval_required` with the exact approve hint.
- In human mode, ms prompts for the exact verbatim command string.
- In ms-managed directories, deletions become **tombstones** (content-addressed
  markers); actual pruning is only performed when explicitly invoked.

**Robot Approval Example:**

[Code block omitted: JSON example payload/schema.]
- Block length: 12 line(s).
- Keys extracted: 9.
- Key: status
- Key: approval_required
- Key: approve_command
- Key: tier
- Key: reason
- Key: timestamp
- Key: version
- Key: data
- Key: warnings
- Example illustrates machine-readable output contract.

---

## 6. Progressive Disclosure System

### 6.1 Disclosure Levels

[Code block omitted: Rust example code (types/logic).]
- Block length: 31 line(s).
- Counts: 0 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 1 field(s).
- Enum: DisclosureLevel
- Impl block: DisclosureLevel
- Function/method: token_budget
- Field/key: DisclosureLevel
- Block defines core structures or algorithms referenced by surrounding text.

### 6.2 Disclosure Logic

[Code block omitted: Rust example code (types/logic).]
- Block length: 70 line(s).
- Counts: 1 struct(s), 2 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 9 field(s).
- Struct: TokenBudget
- Enum: DisclosurePlan
- Enum: PackMode
- Function/method: disclose
- Function/method: disclose_level
- Function/method: disclose_packed
- Field/key: DisclosurePlan
- Field/key: DisclosureLevel
- Field/key: frontmatter
- Field/key: body
- Field/key: scripts
- Field/key: references
- Field/key: tokens
- Field/key: mode
- Field/key: max_per_group
- Block defines core structures or algorithms referenced by surrounding text.

### 6.3 Context-Aware Disclosure

[Code block omitted: Rust example code (types/logic).]
- Block length: 32 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 5 field(s).
- Function/method: optimal_disclosure
- Field/key: skill
- Field/key: context
- Field/key: mode
- Field/key: max_per_group
- Field/key: DisclosurePlan
- Block defines core structures or algorithms referenced by surrounding text.

**Disclosure Context (partial):**

[Code block omitted: Rust example code (types/logic).]
- Block length: 9 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 7 field(s).
- Struct: DisclosureContext
- Field/key: explicit_level
- Field/key: pack_budget
- Field/key: pack_mode
- Field/key: max_per_group
- Field/key: remaining_tokens
- Field/key: usage_history
- Field/key: request_type
- Block defines core structures or algorithms referenced by surrounding text.

### 6.4 Micro-Slicing and Token Packing

To maximize signal per token, ms pre-slices skills into atomic blocks (rules,
commands, examples, pitfalls). A packer then selects the highest-utility slices
that fit within a token budget.

**Slice Generation Heuristics:**

- One slice per rule, command block, example, checklist, or pitfall (including anti-patterns)
- Preserve section headings by attaching them to the first slice in the section
- Estimate tokens per slice using a fast tokenizer heuristic
- Assign utility score from quality signals + usage frequency + evidence coverage

**Token Packer (Constrained Optimization):**

The packer treats slice selection as a constrained optimization problem, not just
greedy selection. This ensures predictable coverage, safer packs, and stable behavior.

**Constraints:**
- Total tokens ≤ budget
- Dependencies satisfied before dependents
- Coverage quotas (e.g., at least 1 from "critical-rules" group)
- Max per group (avoid over-representing one category)
- Risk tier constraints (always include safety warnings)

**Objective:** Maximize total utility with diminishing returns per group.

**Injection-Time Pack Optimization:**
- Penalize slices already present in the current context (novelty-aware packing).
- Boost slices that cover missing task facets (e.g., pitfalls when editing infra).
- Uses context fingerprints + recent usage history to avoid redundancy.

**Pack Contracts (Minimum Viable Guidance):**
- Define contracts like `DebugContract`, `RefactorContract`, `DeployContract`.
- Contracts specify mandatory slices/groups (repro loop, validation, rollback).
- Packer fails closed if a contract cannot be satisfied within budget.

[Code block omitted: Rust example code (types/logic).]
- Block length: 233 line(s).
- Counts: 4 struct(s), 3 enum(s), 0 trait(s), 1 impl(s), 7 fn(s), 22 field(s).
- Struct: PackConstraints
- Struct: CoverageQuota
- Struct: ConstrainedPacker
- Struct: PackResult
- Enum: MandatorySlice
- Enum: MandatoryPredicate
- Enum: PackError
- Impl block: ConstrainedPacker
- Function/method: pack
- Function/method: matches_mandatory
- Function/method: seed_required_coverage
- Function/method: rank_by_density
- Function/method: try_improve
- Function/method: estimate_packed_tokens_with_constraints
- Function/method: score_slice
- Field/key: budget
- Field/key: max_per_group
- Field/key: required_coverage
- Field/key: excluded_groups
- Field/key: max_improvement_passes
- Field/key: mandatory_slices
- Field/key: fail_on_mandatory_omission
- Field/key: group
- Field/key: min_count
- Field/key: slice_id
- Field/key: required_tokens
- Field/key: available_tokens
- Field/key: slices
- Field/key: total_tokens
- Field/key: coverage_satisfied
- Field/key: MandatorySlice
- Field/key: MandatoryPredicate
- Field/key: index
- Field/key: mode
- Field/key: tokens
- Field/key: PackMode
- Field/key: SliceType
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Example:**

[Code block omitted: ms CLI command examples (1 lines).]
- Unique ms commands: 1.
- Example: ms load (flags: --pack)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 6.5 Conditional Block Predicates

Skills often contain version-specific or environment-specific content. Rather than
maintaining separate skills or relying on the agent to reason about versions,
ms supports **block-level predicates** that strip irrelevant content at load time.

**Markdown Syntax:**

[Code block omitted: example block (lang='markdown').]
- Block length: 3 line(s).
- Block contains illustrative content referenced by the surrounding text.

**Predicate Types:**

| Predicate | Example | Evaluates |
|-----------|---------|-----------|
| `package:<name> <op> <version>` | `package:next >= 16.0.0` | package.json / Cargo.toml version |
| `tool:<name> <op> <version>` | `tool:node >= 18.0.0` | Installed tool version |
| `rust:edition <op> <year>` | `rust:edition == 2021` | Cargo.toml rust edition |
| `env:<var>` | `env:CI` | Environment variable presence |
| `file:<pattern>` | `file:src/middleware.ts` | File/glob existence |

**Operators:** `==`, `!=`, `<`, `<=`, `>`, `>=` (semver-aware for versions)

**Evaluation Flow:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 42 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 4 field(s).
- Function/method: evaluate_predicate
- Function/method: filter_slices_by_predicates
- Field/key: PredicateType
- Field/key: glob
- Field/key: slices
- Field/key: ctx
- Block defines core structures or algorithms referenced by surrounding text.

**Why This Matters:**

The agent *cannot* hallucinate using deprecated patterns because those patterns
are physically absent from its context window. This directly addresses the
version drift problem (e.g., Next.js middleware.ts vs proxy.ts) mentioned in
AGENTS.md without requiring separate skills or complex agent reasoning.

**CLI Example:**

[Code block omitted: ms CLI command examples (2 lines).]
- Unique ms commands: 1.
- Example: ms load (flags: --eval-predicates, --package-version)
- Example: ms load (flags: --dry-run, --show-filtered)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 6.6 Meta-Skills: Composed Slice Bundles

Agents rarely need a single skill—they need **task kits** combining slices from
multiple related skills. Meta-skills are first-class compositions that persist
and evolve.

**Why Meta-Skills:**

| Without Meta-Skills | With Meta-Skills |
|---------------------|------------------|
| `ms load nextjs-ui && ms load a11y && ms load react-patterns` | `ms load frontend-polish` |
| Manual coordination of 4+ skills | Single load, optimal packing |
| Repeated setup per session | Battle-tested bundle |

**Data Model:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 33 line(s).
- Counts: 2 struct(s), 1 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 10 field(s).
- Struct: MetaSkill
- Struct: MetaSkillSliceRef
- Enum: PinStrategy
- Field/key: id
- Field/key: name
- Field/key: description
- Field/key: slices
- Field/key: pin_strategy
- Field/key: validated_at
- Field/key: skill_id
- Field/key: slice_id
- Field/key: content_hash
- Field/key: priority_override
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Commands:**

[Code block omitted: ms CLI command examples (6 lines).]
- Unique ms commands: 2.
- Example: ms meta (no flags shown)
- Example: ms load (flags: --pack)
- Example: ms meta (flags: --add, --remove)
- Example: ms meta (no flags shown)
- Example: ms meta (no flags shown)
- Example: ms meta (flags: --slices)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Resolution and Packing:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 31 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 0 field(s).
- Impl block: MetaSkillLoader
- Function/method: resolve
- Function/method: load_packed
- Block defines core structures or algorithms referenced by surrounding text.

**Use Cases:**

- **NTM integration:** Define meta-skills per bead type (e.g., `ui-polish-bead`, `api-refactor-bead`)
- **Onboarding:** Ship `team-standards` meta-skill bundling all org-required rules
- **Tech stack kits:** `rust-cli-complete`, `nextjs-fullstack`, `go-microservice`

---

## 7. Search & Suggestion Engine

### 7.1 Hybrid Search (Following xf Pattern)

[Code block omitted: Rust example code (types/logic).]
- Block length: 40 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 6 field(s).
- Struct: HybridSearcher
- Impl block: HybridSearcher
- Function/method: search
- Field/key: tantivy_index
- Field/key: embedding_index
- Field/key: rrf_k
- Field/key: query
- Field/key: filters
- Field/key: limit
- Block defines core structures or algorithms referenced by surrounding text.

**Alias + Deprecation Handling:**
- If the query exactly matches a skill alias, ms resolves to the canonical skill id.
- Deprecated skills are filtered out by default (use `--include-deprecated` to show them).

### 7.2 Context-Aware Suggestion

[Code block omitted: Rust example code (types/logic).]
- Block length: 118 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 12 field(s).
- Struct: Suggester
- Impl block: Suggester
- Function/method: suggest
- Function/method: trigger_boost
- Function/method: explain_result
- Field/key: searcher
- Field/key: registry
- Field/key: requirements
- Field/key: skill
- Field/key: signals
- Field/key: final_score
- Field/key: signal
- Field/key: contribution
- Field/key: rrf_components
- Field/key: bm25_rank
- Field/key: vector_rank
- Field/key: rrf_score
- Block defines core structures or algorithms referenced by surrounding text.

When `--for-ntm` is used, `ms suggest` returns `swarm_plan` in robot mode so
each agent can load a complementary slice pack instead of duplicating content.

**Bandit-Weighted Suggestion Signals:**
- Use a contextual bandit over signal-weighting schemes (bm25, embeddings, triggers,
  freshness, project match).
- Reward = acceptance / follow-through / outcome success; train per-project + global prior.
- Replaces static weight tuning with adaptive, self-optimizing retrieval.

**Swarm Role Packs (Leader/Worker/Reviewer):**
- Leader gets decision structure + pitfalls + plan template slices.
- Workers get command recipes + examples; reviewer gets semantic diff + safety invariants.

**Suggestion Context (partial):**

[Code block omitted: Rust example code (types/logic).]
- Block length: 13 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 11 field(s).
- Struct: SuggestionContext
- Field/key: cwd
- Field/key: current_file
- Field/key: recent_commands
- Field/key: query
- Field/key: pack_budget
- Field/key: explain
- Field/key: pack_mode
- Field/key: max_per_group
- Field/key: environment
- Field/key: include_deprecated
- Field/key: swarm
- Block defines core structures or algorithms referenced by surrounding text.

**Requirement-aware suggestions:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 88 line(s).
- Counts: 3 struct(s), 1 enum(s), 0 trait(s), 2 impl(s), 3 fn(s), 8 field(s).
- Struct: EnvironmentSnapshot
- Struct: RequirementStatus
- Struct: RequirementChecker
- Enum: NetworkStatus
- Impl block: RequirementStatus
- Impl block: RequirementChecker
- Function/method: is_satisfied
- Function/method: summary
- Function/method: check
- Field/key: platform
- Field/key: tools
- Field/key: env_vars
- Field/key: network
- Field/key: platform_ok
- Field/key: missing_tools
- Field/key: missing_env
- Field/key: network_ok
- Block defines core structures or algorithms referenced by surrounding text.

**Collective Pack Planning (Swarm / NTM):**

[Code block omitted: Rust example code (types/logic).]
- Block length: 37 line(s).
- Counts: 3 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 11 field(s).
- Struct: SwarmContext
- Struct: SwarmPlan
- Struct: AgentPack
- Enum: PackObjective
- Impl block: Suggester
- Function/method: plan_swarm_packs
- Field/key: agent_count
- Field/key: budget_per_agent
- Field/key: objective
- Field/key: replicate_pitfalls
- Field/key: agents
- Field/key: total_tokens
- Field/key: agent_id
- Field/key: slice_ids
- Field/key: token_estimate
- Field/key: skill
- Field/key: context
- Block defines core structures or algorithms referenced by surrounding text.

### 7.2.1 Context Fingerprints & Suggestion Cooldowns

To prevent `ms suggest` from spamming the same skills repeatedly when context hasn't meaningfully changed, we compute a **context fingerprint** and maintain a cooldown cache.

[Code block omitted: Rust example code (types/logic).]
- Block length: 83 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 6 fn(s), 5 field(s).
- Struct: ContextFingerprint
- Impl block: ContextFingerprint
- Function/method: compute
- Function/method: differs_from
- Function/method: fingerprint_hash
- Function/method: hash_string_list
- Function/method: git_head_short
- Function/method: git_diff_hash
- Field/key: repo_root
- Field/key: git_head
- Field/key: diff_hash
- Field/key: open_files_hash
- Field/key: recent_commands_hash
- Block defines core structures or algorithms referenced by surrounding text.

**Cooldown Cache:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 100 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 6 fn(s), 7 field(s).
- Struct: SuggestionCooldownCache
- Struct: CooldownEntry
- Impl block: SuggestionCooldownCache
- Function/method: load
- Function/method: save
- Function/method: should_suppress
- Function/method: record
- Function/method: evict_oldest
- Function/method: gc
- Field/key: entries
- Field/key: max_entries
- Field/key: skill_ids
- Field/key: suggested_at
- Field/key: fingerprint
- Field/key: std
- Field/key: cooldown
- Block defines core structures or algorithms referenced by surrounding text.

**Integration with Suggester:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 64 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 2 fn(s), 8 field(s).
- Struct: CooldownConfig
- Struct: SuggestionResult
- Impl block: Suggester
- Impl block: Default
- Function/method: suggest_with_cooldown
- Function/method: default
- Field/key: context
- Field/key: cooldown_config
- Field/key: suggestions
- Field/key: suppressed
- Field/key: reason
- Field/key: fingerprint
- Field/key: enabled
- Field/key: duration
- Block defines core structures or algorithms referenced by surrounding text.

**CLI flags:**

[Code block omitted: ms CLI command examples (4 lines).]
- Unique ms commands: 1.
- Example: ms suggest (flags: --no-cooldown)
- Example: ms suggest (flags: --cooldown)
- Example: ms suggest (flags: --show-fingerprint)
- Example: ms suggest (flags: --clear-cache)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

This mechanism prevents suggestion spam in tight loops (e.g., IDE integrations calling `ms suggest` on every keystroke) while still responding to meaningful context changes like new commits, file edits, or command history.

### 7.3 Hash-Based Embeddings (From xf)

[Code block omitted: Rust example code (types/logic).]
- Block length: 41 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: hash_embedding
- Block defines core structures or algorithms referenced by surrounding text.

### 7.3.1 Pluggable Embedding Backends

Hash embeddings are the default (fast, deterministic, zero dependencies). For
higher semantic fidelity, ms supports an optional local ML embedder.

[Code block omitted: Rust example code (types/logic).]
- Block length: 21 line(s).
- Counts: 2 struct(s), 0 enum(s), 1 trait(s), 1 impl(s), 2 fn(s), 1 field(s).
- Struct: HashEmbedder
- Struct: LocalMlEmbedder
- Trait: Embedder
- Impl block: Embedder
- Function/method: embed
- Function/method: dims
- Field/key: dims
- Block defines core structures or algorithms referenced by surrounding text.

**Selection Rules:**
- Default: `HashEmbedder`
- If `embeddings.backend = "local"` and model available → `LocalMlEmbedder`
- Fallback to hash if local model missing

### 7.4 Skill Quality Scoring Algorithm

Quality scoring determines which skills are most worth surfacing to agents. This section details the multi-factor scoring algorithm, including provenance (evidence coverage and confidence).

[Code block omitted: Rust example code (types/logic).]
- Block length: 294 line(s).
- Counts: 4 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 9 fn(s), 26 field(s).
- Struct: QualityScorer
- Struct: QualityWeights
- Struct: QualityScore
- Struct: QualityFactors
- Impl block: Default
- Impl block: QualityScorer
- Function/method: default
- Function/method: score
- Function/method: score_structure
- Function/method: score_content
- Function/method: score_effectiveness
- Function/method: score_provenance
- Function/method: score_safety
- Function/method: score_freshness
- Function/method: score_popularity
- Field/key: weights
- Field/key: usage_tracker
- Field/key: toolchain_detector
- Field/key: project_path
- Field/key: structure_weight
- Field/key: content_weight
- Field/key: effectiveness_weight
- Field/key: provenance_weight
- Field/key: safety_weight
- Field/key: freshness_weight
- Field/key: popularity_weight
- Field/key: overall
- Field/key: factors
- Field/key: issues
- Field/key: suggestions
- Field/key: structure
- Field/key: content
- Field/key: effectiveness
- Field/key: provenance
- Field/key: safety
- Field/key: freshness
- Field/key: popularity
- Field/key: replaced_by
- Field/key: tool
- Field/key: skill_range
- Field/key: project_version
- Block defines core structures or algorithms referenced by surrounding text.

**Quality Issue Types:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 21 line(s).
- Counts: 0 struct(s), 1 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 0 field(s).
- Enum: QualityIssue
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Integration:**

[Code block omitted: ms CLI command examples (7 lines).]
- Unique ms commands: 1.
- Example: ms quality (no flags shown)
- Example: ms quality (flags: --verbose)
- Example: ms quality (flags: --all, --min)
- Example: ms quality (flags: --check, --min)
- Example: ms quality (flags: --stale)
- Example: ms quality (flags: --stale, --project)
- Example: ms quality (flags: --robot)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Quality-Based Filtering:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 18 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 3 field(s).
- Function/method: filter_by_quality
- Field/key: skills
- Field/key: min_score
- Field/key: scorer
- Block defines core structures or algorithms referenced by surrounding text.

---

### 7.5 Skill Pruning & Evolution

As the registry grows, ms must keep skills lean and current without destructive
deletions. Pruning is **proposal-first**: identify candidates, suggest merges
or deprecations, and require explicit confirmation before applying changes.

**Signals:**
- Low recent usage (e.g., <5 uses in 30 days)
- Low quality score (e.g., <0.3)
- High similarity to another skill (e.g., >= 0.8)
- Persistent toolchain mismatch or stale indicators

**Actions (non-destructive by default):**
- Propose merge: "Combine X and Y into Z" with auto-generated draft
- Propose deprecate: mark as deprecated + replacement alias
- Propose split: break overly broad skills into focused children
- Emit BV beads for review and scheduling

**CLI Example:**

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 1.
- Example: ms prune (flags: --scope, --dry-run)
- Example: ms prune (flags: --scope, --emit-beads)
- Example: ms prune (flags: --scope, --apply, --require-confirmation)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

## 8. Bundle & Distribution System

### 8.1 Bundle Format

[Code block omitted: YAML example.]
- Block length: 26 line(s).
- Keys extracted: 17.
- Key: name
- Key: version
- Key: channel
- Key: description
- Key: author
- Key: license
- Key: homepage
- Key: skills
- Key: - id
- Key: path
- Key: dependencies
- Key: - bundle
- Key: checksum
- Key: signatures
- Key: - signer
- Key: key_id
- Key: signature
- Example encodes structured test/spec or config data.

### 8.2 GitHub Integration

[Code block omitted: Rust example code (types/logic).]
- Block length: 36 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 9 field(s).
- Function/method: publish_to_github
- Field/key: bundle
- Field/key: config
- Field/key: description
- Field/key: private
- Field/key: tag_name
- Field/key: name
- Field/key: body
- Field/key: repo_url
- Field/key: release_url
- Block defines core structures or algorithms referenced by surrounding text.

### 8.3 Installation Flow

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 29 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 8.4 Sharing with Local Modification Safety

The sharing system allows one-URL distribution of all your skills while preserving local customizations when upstream changes arrive.

#### The Three-Tier Storage Model

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 27 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

#### Local Modification Data Model

[Code block omitted: Rust example code (types/logic).]
- Block length: 50 line(s).
- Counts: 2 struct(s), 2 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 14 field(s).
- Struct: LocalModification
- Struct: ConflictInfo
- Enum: SkillSyncStatus
- Enum: Resolution
- Field/key: skill_id
- Field/key: upstream_bundle
- Field/key: base_version
- Field/key: patch
- Field/key: created_at
- Field/key: updated_at
- Field/key: reason
- Field/key: current_version
- Field/key: upstream_version
- Field/key: conflicts
- Field/key: section
- Field/key: upstream_change
- Field/key: local_change
- Field/key: suggested_resolution
- Block defines core structures or algorithms referenced by surrounding text.

#### The Sync Engine

[Code block omitted: Rust example code (types/logic).]
- Block length: 112 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 4 fn(s), 14 field(s).
- Struct: SyncEngine
- Impl block: SyncEngine
- Function/method: sync
- Function/method: create_modification
- Function/method: backup_local_mods
- Function/method: reapply_modifications
- Field/key: upstream_dir
- Field/key: local_mods_dir
- Field/key: merged_dir
- Field/key: backup_dir
- Field/key: skill_id
- Field/key: new_content
- Field/key: reason
- Field/key: upstream_bundle
- Field/key: base_version
- Field/key: created_at
- Field/key: updated_at
- Field/key: Utc
- Field/key: current_version
- Field/key: upstream_version
- Block defines core structures or algorithms referenced by surrounding text.

#### One-URL Sharing

Share all your skills (including local modifications) via a single URL:

[Code block omitted: Rust example code (types/logic).]
- Block length: 51 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 10 field(s).
- Function/method: generate_share_url
- Field/key: skills
- Field/key: local_mods
- Field/key: config
- Field/key: version
- Field/key: created_at
- Field/key: local_modifications
- Field/key: upstream_sources
- Field/key: description
- Field/key: private
- Field/key: std
- Block defines core structures or algorithms referenced by surrounding text.

**CLI Commands:**

[Code block omitted: ms CLI command examples (9 lines).]
- Unique ms commands: 2.
- Example: ms share (flags: --gist)
- Example: ms share (flags: --repo)
- Example: ms share (flags: --export)
- Example: ms import (no flags shown)
- Example: ms import (no flags shown)
- Example: ms import (no flags shown)
- Example: ms share (flags: --status)
- Example: ms share (flags: --auto-sync)
- Example: ms share (flags: --dry-run)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

#### Sync Status Dashboard

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 27 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

#### Conflict Resolution Workflow

[Code block omitted: shell command examples (28 lines).]
- Unique tools referenced: 16.
- Tool invoked: $
- Tool invoked: Syncing
- Tool invoked: ✓
- Tool invoked: ⚠
- Tool invoked: Conflict
- Tool invoked: Section:
- Tool invoked: Upstream
- Tool invoked: -
- Tool invoked: +
- Tool invoked: Your
- Tool invoked: Suggested
- Tool invoked: Options:
- Tool invoked: 1.
- Tool invoked: 2.
- Tool invoked: 3.
- Tool invoked: 4.
- Commands illustrate typical workflows and integrations.

#### Automatic Backup Schedule

[Code block omitted: Rust example code (types/logic).]
- Block length: 21 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 4 field(s).
- Struct: BackupConfig
- Impl block: Default
- Function/method: default
- Field/key: retention_count
- Field/key: backup_on_sync
- Field/key: backup_on_modify
- Field/key: scheduled_interval
- Block defines core structures or algorithms referenced by surrounding text.

**Backup Commands:**

[Code block omitted: ms CLI command examples (4 lines).]
- Unique ms commands: 1.
- Example: ms backup (flags: --reason)
- Example: ms backup (no flags shown)
- Example: ms backup (no flags shown)
- Example: ms backup (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 8.5 Multi-Machine Synchronization

Following the xf pattern for distributed archive access across multiple development machines.

#### 8.5.1 Machine Identity

[Code block omitted: Rust example code (types/logic).]
- Block length: 32 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 4 field(s).
- Struct: MachineIdentity
- Impl block: MachineIdentity
- Function/method: generate
- Function/method: load_or_create
- Field/key: machine_id
- Field/key: machine_name
- Field/key: sync_timestamps
- Field/key: std
- Block defines core structures or algorithms referenced by surrounding text.

#### 8.5.2 Sync State Tracking

[Code block omitted: Rust example code (types/logic).]
- Block length: 67 line(s).
- Counts: 3 struct(s), 3 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 13 field(s).
- Struct: SyncState
- Struct: SkillSyncState
- Struct: RemoteConfig
- Enum: SkillSyncStatus
- Enum: RemoteType
- Enum: SyncDirection
- Field/key: skill_states
- Field/key: remotes
- Field/key: last_full_sync
- Field/key: skill_id
- Field/key: local_modified
- Field/key: remote_modified
- Field/key: content_hash
- Field/key: status
- Field/key: name
- Field/key: remote_type
- Field/key: url
- Field/key: auto_sync
- Field/key: direction
- Block defines core structures or algorithms referenced by surrounding text.

#### 8.5.3 Conflict Resolution

[Code block omitted: Rust example code (types/logic).]
- Block length: 103 line(s).
- Counts: 3 struct(s), 3 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 14 field(s).
- Struct: ConflictResolver
- Struct: ConflictInfo
- Struct: SkillVersion
- Enum: ConflictStrategy
- Enum: ConflictType
- Enum: Resolution
- Impl block: ConflictResolver
- Function/method: resolve
- Function/method: attempt_three_way_merge
- Field/key: default_strategy
- Field/key: skill_strategies
- Field/key: skill_id
- Field/key: local_version
- Field/key: remote_version
- Field/key: base_version
- Field/key: conflict_type
- Field/key: content_hash
- Field/key: modified_at
- Field/key: modified_by
- Field/key: version_number
- Field/key: ConflictStrategy
- Field/key: local_suffix
- Field/key: remote_suffix
- Block defines core structures or algorithms referenced by surrounding text.

#### 8.5.4 Sync Engine

[Code block omitted: Rust example code (types/logic).]
- Block length: 119 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 4 fn(s), 14 field(s).
- Struct: SyncEngine
- Struct: SyncReport
- Impl block: SyncEngine
- Impl block: SyncReport
- Function/method: sync
- Function/method: quick_sync
- Function/method: watch_and_sync
- Function/method: summary
- Field/key: machine_identity
- Field/key: sync_state
- Field/key: conflict_resolver
- Field/key: skills_db
- Field/key: SyncChange
- Field/key: Utc
- Field/key: remote
- Field/key: pulled
- Field/key: pushed
- Field/key: resolved
- Field/key: conflicts
- Field/key: deleted
- Field/key: errors
- Field/key: duration
- Block defines core structures or algorithms referenced by surrounding text.

#### 8.5.5 CLI Commands

[Code block omitted: ms CLI command examples (18 lines).]
- Unique ms commands: 4.
- Example: ms remote (flags: --bidirectional)
- Example: ms remote (flags: --push-only)
- Example: ms remote (flags: --pull-only)
- Example: ms remote (no flags shown)
- Example: ms remote (no flags shown)
- Example: ms sync (no flags shown)
- Example: ms sync (no flags shown)
- Example: ms sync (flags: --quick)
- Example: ms sync (flags: --dry-run)
- Example: ms sync (no flags shown)
- Example: ms conflicts (no flags shown)
- Example: ms conflicts (no flags shown)
- Example: ms conflicts (flags: --prefer-local)
- Example: ms conflicts (flags: --prefer-remote)
- Example: ms conflicts (flags: --merge)
- Example: ms sync (no flags shown)
- Example: ms machine (no flags shown)
- Example: ms machine (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

#### 8.5.6 Robot Mode for Multi-Machine

[Code block omitted: ms CLI command examples (4 lines).]
- Unique ms commands: 4.
- Example: ms --robot-sync-status (no flags shown)
- Example: ms --robot-sync (flags: --remote)
- Example: ms --robot-conflicts (no flags shown)
- Example: ms --robot-resolve (flags: --skill, --strategy)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

#### 8.5.7 Sync Configuration

[Code block omitted: TOML config example.]
- Block length: 38 line(s).
- Counts: 8 section(s), 15 key(s).
- Section: machine
- Section: sync
- Section: remotes.origin
- Section: remotes.origin.auth
- Section: remotes.backup
- Section: remotes.work
- Section: skill_overrides."personal-workflow"
- Section: skill_overrides."team-standards"
- Key: name
- Key: default_conflict_strategy
- Key: auto_sync_on_change
- Key: auto_sync_interval_minutes
- Key: sync_on_startup
- Key: sync_skills
- Key: sync_bundles
- Key: sync_config
- Key: url
- Key: type
- Key: direction
- Key: auto_sync
- Key: method
- Key: sync
- Key: conflict_strategy
- Example shows configuration defaults and feature toggles.

---

## 9. Auto-Update System (Following xf Pattern)

### 9.1 Update Check

[Code block omitted: Rust example code (types/logic).]
- Block length: 88 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 11 field(s).
- Struct: Updater
- Struct: UpdateInfo
- Impl block: Updater
- Function/method: check
- Function/method: install
- Field/key: current_version
- Field/key: github_repo
- Field/key: binary_name
- Field/key: version
- Field/key: download_url
- Field/key: release_notes
- Field/key: checksum_url
- Field/key: signature_url
- Field/key: info
- Field/key: security
- Field/key: std
- Block defines core structures or algorithms referenced by surrounding text.

### 9.2 Release Workflow

[Code block omitted: YAML example.]
- Block length: 48 line(s).
- Keys extracted: 21.
- Key: name
- Key: on
- Key: push
- Key: tags
- Key: jobs
- Key: build
- Key: strategy
- Key: matrix
- Key: include
- Key: - os
- Key: target
- Key: artifact
- Key: runs-on
- Key: steps
- Key: - uses
- Key: - name
- Key: uses
- Key: with
- Key: targets
- Key: run
- Key: files
- Example encodes structured test/spec or config data.

---

## 10. Configuration System

### 10.1 Config File Structure

[Code block omitted: TOML config example.]
- Block length: 171 line(s).
- Counts: 19 section(s), 67 key(s).
- Section: general
- Section: disclosure
- Section: dependencies
- Section: layers
- Section: toolchain
- Section: paths
- Section: paths.layers
- Section: search
- Section: embeddings
- Section: cass
- Section: generalization
- Section: uncertainty
- Section: prune
- Section: privacy
- Section: safety
- Section: security
- Section: build
- Section: github
- Section: display
- Key: default_disclosure
- Key: min_quality_score
- Key: max_suggestions
- Key: default_pack_budget
- Key: default_pack_mode
- Key: default_max_per_group
- Key: auto_load
- Key: default_mode
- Key: default_level
- Key: max_depth
- Key: order
- Key: conflict_strategy
- Key: merge_strategy
- Key: section_preference
- Key: emit_conflicts
- Key: detect_toolchain
- Key: project_roots
- Key: max_major_drift
- Key: skill_paths
- Key: base
- Key: org
- Key: project
- Key: user
- Key: exclude_patterns
- Key: default_limit
- Key: rrf_k
- Key: embedding_dims
- Key: backend
- Key: model_path
- Key: binary
- Key: default_session_limit
- Key: min_pattern_confidence
- Key: min_session_quality
- Key: incremental_scan
- Key: engine
- Key: llm_critique
- Key: llm_timeout_ms
- Key: enabled
- Key: min_confidence
- Key: max_pending
- Key: window_days
- Key: min_uses
- Key: merge_similarity
- Key: require_confirmation
- Key: redaction_enabled
- Key: redaction_min_entropy
- Key: redaction_patterns
- Key: "(?i)api[_-]?key\\s*[:
- Key: "(?i)secret\\s*[:
- Key: redaction_allowlist
- Key: prompt_injection_enabled
- Key: prompt_injection_patterns
- Key: quarantine_dir
- Key: destructive_ops
- Key: require_verbatim_approval
- Key: tombstone_deletes
- Key: verify_bundles
- Key: verify_updates
- Key: trusted_keys
- Key: auto_save_interval
- Key: max_iterations
- Key: include_anti_patterns
- Key: default_visibility
- Key: update_check_hours
- Key: theme
- Key: use_icons
- Key: color_scheme
- Example shows configuration defaults and feature toggles.

### 10.2 Project-Local Config

[Code block omitted: TOML config example.]
- Block length: 13 line(s).
- Counts: 2 section(s), 3 key(s).
- Section: project
- Section: triggers
- Key: skill_paths
- Key: "rust-async-patterns"
- Key: "rust-testing"
- Example shows configuration defaults and feature toggles.

---

## 11. Implementation Phases

### Phase 1: Foundation

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 17 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### Phase 2: Search

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 14 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### Phase 3: Disclosure & Suggestions

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 17 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### Phase 4: CASS Integration

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 16 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### Phase 5: Bundles & Distribution

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 15 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### Phase 6: Polish & Auto-Update

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 17 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Reordered Phasing (Hard Invariants First):**
1. Spec-only editing + compilation + semantic diff
2. Index + skillpack artifacts + fast suggest/load
3. Provenance compression + taint/reassembly resistance
4. Mining pipeline + Pattern IR
5. Swarm orchestration + bandit scoring
6. TUI polish + bundles + auto-update

---

## 12. Dependencies (Cargo.toml)

[Code block omitted: TOML config example.]
- Block length: 59 line(s).
- Counts: 5 section(s), 41 key(s).
- Section: package
- Section: bin
- Section: dependencies
- Section: dev-dependencies
- Section: profile.release
- Key: name
- Key: version
- Key: edition
- Key: rust-version
- Key: authors
- Key: description
- Key: license
- Key: repository
- Key: path
- Key: clap
- Key: tokio
- Key: serde
- Key: serde_json
- Key: serde_yaml
- Key: toml
- Key: rusqlite
- Key: tantivy
- Key: gix
- Key: reqwest
- Key: anyhow
- Key: thiserror
- Key: tracing
- Key: tracing-subscriber
- Key: crossterm
- Key: ratatui
- Key: indicatif
- Key: chrono
- Key: directories
- Key: glob
- Key: regex
- Key: sha2
- Key: semver
- Key: uuid
- Key: pulldown-cmark
- Key: tempfile
- Key: assert_cmd
- Key: predicates
- Key: lto
- Key: codegen-units
- Key: strip
- Key: panic
- Example shows configuration defaults and feature toggles.

---

## 13. Key Design Decisions

### 13.1 Why Hash Embeddings Instead of ML Models

| ML Embeddings | Hash Embeddings |
|---------------|-----------------|
| Requires model download (100MB+) | Zero dependencies |
| GPU/CPU inference overhead | Pure CPU, instant |
| Version lock-in | Always reproducible |
| Network dependency for updates | Fully offline |
| Black box | Transparent algorithm |

The hash embedding approach from xf provides 80-90% of ML embedding quality for skill matching, with none of the operational complexity.
For teams that need higher semantic fidelity, ms supports an **optional local ML embedder**
that is still offline and fully opt-in.

### 13.2 Why SQLite + Git Dual Persistence

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 17 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Two-Phase Commit for Consistency**

To avoid partial writes (SQLite updated but Git not, or vice versa), ms wraps every
write in a two-phase commit (2PC) protocol with a durable write-ahead record.

[Code block omitted: example block (lang='n/a').]
- Block length: 10 line(s).
- Block contains illustrative content referenced by the surrounding text.

This makes dual persistence crash-safe and idempotent.

### 13.3 Why Interactive Build Over Fully Automated

The iterative, interactive build process is essential because:

1. **Quality requires judgment** - Not all extracted patterns are good
2. **Context is king** - Human knows what's actually useful
3. **Refinement improves output** - Each iteration focuses the skill
4. **Learning opportunity** - User sees what patterns exist in their sessions

Fully automated mode (`--auto`) is available for pipelines, but interactive is the default for quality.

---

## 14. Future Extensions

### 14.1 Planned Features

| Feature | Description | Priority |
|---------|-------------|----------|
| **Skill composition** | Combine multiple skills into workflows | P1 |
| **Team sharing** | Enterprise-grade skill distribution | P1 |
| **Skill analytics** | Usage patterns, effectiveness metrics | P2 |
| **IDE plugins** | VS Code, JetBrains integration | P2 |
| **Multi-agent coordination** | Skill assignment to agent swarms | P2 |
| **Semantic versioning** | Track skill changes, migrations | P3 |
| **Skill testing** | Validate skills against example scenarios | P3 |
| **Skill bounties** | Prioritize requests with credits/bounties | P3 |

### 14.2 Integration Points

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 24 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

---

## 15. Success Metrics

### 15.1 Technical Metrics

| Metric | Target |
|--------|--------|
| Indexing speed | 1000 skills/second |
| Search latency | <50ms p99 |
| Memory usage | <100MB idle |
| Binary size | <20MB stripped |
| Build session start | <2 seconds |

### 15.2 User Experience Metrics

| Metric | Target |
|--------|--------|
| Skill suggestion relevance | >80% useful |
| Build session completion rate | >60% |
| Bundle installation success | >95% |
| Time to first useful skill | <5 minutes |

---

## 16. Appendix: THE EXACT PROMPTS

### 16.1 Pattern Extraction Prompt

[Code block omitted: example block (lang='n/a').]
- Block length: 17 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 16.2 Draft Generation Prompt

[Code block omitted: example block (lang='n/a').]
- Block length: 17 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 16.3 Refinement Prompt

[Code block omitted: example block (lang='n/a').]
- Block length: 13 line(s).
- Block contains illustrative content referenced by the surrounding text.

---

## 17. Getting Started

[Code block omitted: ms CLI command examples (5 lines).]
- Unique ms commands: 5.
- Example: ms init (flags: --global)
- Example: ms index (flags: --path)
- Example: ms build (flags: --name)
- Example: ms search (no flags shown)
- Example: ms suggest (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

---

## 18. Testing Strategy

### 18.1 Testing Philosophy

Following Rust best practices with comprehensive coverage across unit, integration, and property-based tests.

[Code block omitted: Rust example code (types/logic).]
- Block length: 13 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 0 field(s).
- Block defines core structures or algorithms referenced by surrounding text.

### 18.2 Unit Tests

[Code block omitted: Rust example code (types/logic).]
- Block length: 138 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 11 fn(s), 4 field(s).
- Function/method: test_fnv1a_deterministic
- Function/method: test_embedding_dimensions
- Function/method: test_embedding_normalization
- Function/method: test_similar_texts_similar_embeddings
- Function/method: test_skill_name_validation
- Function/method: test_frontmatter_parsing
- Function/method: test_malformed_frontmatter
- Function/method: test_missing_required_fields
- Function/method: test_index_and_search
- Function/method: test_rrf_fusion
- Function/method: test_structure_score
- Field/key: name
- Field/key: description
- Field/key: id
- Field/key: content
- Block defines core structures or algorithms referenced by surrounding text.
example code
[Code block omitted: example block (lang='n/a').]
- Block length: 19 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 18.3 Integration Tests

[Code block omitted: Rust example code (types/logic).]
- Block length: 73 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 5 fn(s), 4 field(s).
- Function/method: test_init_creates_config
- Function/method: test_index_skills_directory
- Function/method: test_search_returns_results
- Function/method: test_robot_mode_json_output
- Function/method: test_suggest_with_cass_integration
- Field/key: Command
- Field/key: std
- Field/key: name
- Field/key: description
- Block defines core structures or algorithms referenced by surrounding text.

### 18.4 Property-Based Tests

[Code block omitted: Rust example code (types/logic).]
- Block length: 54 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 5 fn(s), 2 field(s).
- Function/method: test_skill_id_generation_unique
- Function/method: test_embedding_always_normalized
- Function/method: test_search_never_panics
- Function/method: test_rrf_order_independent
- Function/method: test_yaml_roundtrip
- Field/key: name
- Field/key: description
- Block defines core structures or algorithms referenced by surrounding text.

### 18.5 Snapshot Tests

[Code block omitted: Rust example code (types/logic).]
- Block length: 30 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 5 fn(s), 0 field(s).
- Function/method: test_skill_disclosure_minimal
- Function/method: test_skill_disclosure_full
- Function/method: test_robot_status_output
- Function/method: test_doctor_report_format
- Function/method: test_search_results_format
- Block defines core structures or algorithms referenced by surrounding text.

### 18.6 Benchmark Tests

[Code block omitted: Rust example code (types/logic).]
- Block length: 65 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 4 fn(s), 1 field(s).
- Function/method: bench_hash_embedding
- Function/method: bench_search
- Function/method: bench_rrf_fusion
- Function/method: bench_skill_parsing
- Field/key: BenchmarkId
- Block defines core structures or algorithms referenced by surrounding text.

### 18.7 Test Fixtures and Helpers

[Code block omitted: Rust example code (types/logic).]
- Block length: 69 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 5 fn(s), 13 field(s).
- Struct: TestFixture
- Impl block: TestFixture
- Impl block: Drop
- Function/method: new
- Function/method: with_indexed_skills
- Function/method: with_mock_cass
- Function/method: config_dir
- Function/method: drop
- Field/key: temp_dir
- Field/key: config_dir
- Field/key: skills_dir
- Field/key: db
- Field/key: index
- Field/key: std
- Field/key: id
- Field/key: name
- Field/key: description
- Field/key: agent
- Field/key: project
- Field/key: messages
- Field/key: MockCass
- Block defines core structures or algorithms referenced by surrounding text.

### 18.8 CI Integration

[Code block omitted: YAML example.]
- Block length: 67 line(s).
- Keys extracted: 25.
- Key: name
- Key: on
- Key: push
- Key: branches
- Key: pull_request
- Key: env
- Key: CARGO_TERM_COLOR
- Key: RUST_BACKTRACE
- Key: jobs
- Key: test
- Key: runs-on
- Key: steps
- Key: - uses
- Key: - name
- Key: uses
- Key: with
- Key: components
- Key: run
- Key: coverage
- Key: files
- Key: fail_ci_if_error
- Key: property-tests
- Key: PROPTEST_CASES
- Key: benchmarks
- Key: snapshots
- Example encodes structured test/spec or config data.

### 18.9 Skill Tests

Skills can include executable tests to validate correctness. Tests are stored
under `tests/` and run via `ms test`.

**Test Format (YAML):**

[Code block omitted: YAML example.]
- Block length: 8 line(s).
- Keys extracted: 7.
- Key: name
- Key: skill
- Key: steps
- Key: - load_skill
- Key: - run
- Key: - assert
- Key: contains
- Example encodes structured test/spec or config data.

**Runner Contract:**
- `load_skill` injects the selected disclosure
- `run` executes a command or script
- `assert` checks stdout/stderr patterns or file outputs

**CLI:**

[Code block omitted: ms CLI command examples (2 lines).]
- Unique ms commands: 1.
- Example: ms test (no flags shown)
- Example: ms test (flags: --all, --report)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

**Extended Test Types:**

Beyond basic schema/script tests, ms supports **retrieval tests** and **packing tests**
to enable regression testing of search quality and token efficiency.

[Code block omitted: YAML example.]
- Block length: 18 line(s).
- Keys extracted: 14.
- Key: name
- Key: skill
- Key: type
- Key: tests
- Key: - context
- Key: cwd
- Key: files
- Key: keywords
- Key: query
- Key: expect
- Key: top_k
- Key: score_min
- Key: diff
- Key: suggested
- Example encodes structured test/spec or config data.

[Code block omitted: YAML example.]
- Block length: 20 line(s).
- Keys extracted: 10.
- Key: name
- Key: skill
- Key: type
- Key: tests
- Key: - budget
- Key: expect_contains
- Key: expect_excludes
- Key: expect_coverage_groups
- Key: expect_min_utility
- Key: expect_max_slices
- Example encodes structured test/spec or config data.

**Test Harness Implementation:**

[Code block omitted: Rust example code (types/logic).]
- Block length: 41 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 9 field(s).
- Struct: SkillTestHarness
- Impl block: SkillTestHarness
- Function/method: run_retrieval_test
- Function/method: run_packing_test
- Field/key: registry
- Field/key: searcher
- Field/key: packer
- Field/key: passed
- Field/key: RetrievalExpect
- Field/key: actual_rank
- Field/key: actual_score
- Field/key: packed_slice_ids
- Field/key: total_tokens
- Block defines core structures or algorithms referenced by surrounding text.

**CI Integration:**

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 1.
- Example: ms test (flags: --all, --report)
- Example: ms test (flags: --type)
- Example: ms test (flags: --type, --budget-range)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

---

### 18.10 Skill Simulation Sandbox

Simulate a skill end-to-end in a controlled workspace before publishing. This
catches broken commands, missing assumptions, and brittle steps without touching
real projects.

**Behavior:**
- Create a temporary workspace with mock files/tools
- Execute skill steps (or mapped test steps) in order
- Capture stdout/stderr and compare against expected assertions
- Emit a simulation report and optional example transcript

**CLI:**

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 1.
- Example: ms simulate (no flags shown)
- Example: ms simulate (flags: --project)
- Example: ms simulate (flags: --report)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

---

## 19. Skill Templates Library

### 19.1 Template System Overview

Pre-built templates for common skill patterns, enabling rapid skill creation with best practices baked in.

[Code block omitted: Rust example code (types/logic).]
- Block length: 52 line(s).
- Counts: 5 struct(s), 2 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 20 field(s).
- Struct: TemplateLibrary
- Struct: SkillTemplate
- Struct: TemplateStructure
- Struct: TemplateSection
- Struct: Placeholder
- Enum: TemplateCategory
- Enum: ContentType
- Field/key: templates
- Field/key: custom_templates_dir
- Field/key: id
- Field/key: name
- Field/key: description
- Field/key: category
- Field/key: structure
- Field/key: placeholders
- Field/key: examples
- Field/key: best_for
- Field/key: sections
- Field/key: optional_sections
- Field/key: resources
- Field/key: heading_level
- Field/key: content_type
- Field/key: placeholder
- Field/key: example
- Field/key: default
- Field/key: validation
- Field/key: required
- Block defines core structures or algorithms referenced by surrounding text.

### 19.2 Built-in Templates

#### 19.2.1 Workflow Template

[Code block omitted: example block (lang='markdown').]
- Block length: 12 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{step_1_code}}
[Code block omitted: example block (lang='n/a').]
- Block length: 3 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{decision_point}} ?
├── YES → {{yes_action}}
└── NO → {{no_action}}
[Code block omitted: example block (lang='n/a').]
- Block length: 10 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### 19.2.2 Checklist Template

[Code block omitted: example block (lang='markdown').]
- Block length: 31 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### 19.2.3 Debugging Template

[Code block omitted: example block (lang='markdown').]
- Block length: 8 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{diagnostic_command}}
[Code block omitted: example block (lang='n/a').]
- Block length: 6 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{fix_code}}
[Code block omitted: example block (lang='n/a').]
- Block length: 6 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{symptom}}
├── Check: {{check_1}}
│   ├── PASS → {{next_check}}
│   └── FAIL → {{cause_1}} → {{fix_1}}
└── Check: {{check_2}}
    └── FAIL → {{cause_2}} → {{fix_2}}
[Code block omitted: example block (lang='n/a').]
- Block length: 2 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### 19.2.4 Integration Template

[Code block omitted: example block (lang='markdown').]
- Block length: 10 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{setup_commands}}
[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{config_example}}
[Code block omitted: example block (lang='n/a').]
- Block length: 2 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{operation_1_command}}
[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{operation_2_command}}
[Code block omitted: example block (lang='n/a').]
- Block length: 11 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### 19.2.5 Pattern Template

[Code block omitted: example block (lang='markdown').]
- Block length: 13 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{pattern_structure}}
[Code block omitted: example block (lang='n/a').]
- Block length: 2 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{basic_implementation}}
[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{advanced_implementation}}
[Code block omitted: example block (lang='n/a').]
- Block length: 3 line(s).
- Block contains illustrative content referenced by the surrounding text.
{{variation_1_code}}
[Code block omitted: example block (lang='n/a').]
- Block length: 7 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 19.3 Template CLI Commands

[Code block omitted: ms CLI command examples (7 lines).]
- Unique ms commands: 1.
- Example: ms template (no flags shown)
- Example: ms template (no flags shown)
- Example: ms template (flags: --name)
- Example: ms template (no flags shown)
- Example: ms template (flags: --from-skill, --name)
- Example: ms template (no flags shown)
- Example: ms template (flags: --output)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 19.4 Template Instantiation Engine

[Code block omitted: Rust example code (types/logic).]
- Block length: 95 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 4 fn(s), 9 field(s).
- Struct: TemplateEngine
- Impl block: TemplateEngine
- Function/method: new
- Function/method: instantiate
- Function/method: interactive_instantiate
- Function/method: code_block_helper
- Field/key: templates
- Field/key: handlebars
- Field/key: template_id
- Field/key: values
- Field/key: name
- Field/key: pattern
- Field/key: h
- Field/key: _
- Field/key: out
- Block defines core structures or algorithms referenced by surrounding text.

### 19.5 Template Discovery from Sessions

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 57 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 19.6 Template Validation

[Code block omitted: Rust example code (types/logic).]
- Block length: 45 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 1 field(s).
- Struct: TemplateValidator
- Impl block: TemplateValidator
- Function/method: validate
- Field/key: rules
- Block defines core structures or algorithms referenced by surrounding text.

---

## 20. Agent Mail Integration for Multi-Agent Skill Coordination

### 20.1 Overview

The `ms` CLI integrates with the Agent Mail MCP server to enable multi-agent skill coordination. When multiple agents work on the same project, they need to:

1. **Share discovered patterns** in real-time
2. **Coordinate skill generation** to avoid duplication
3. **Request skills** from other agents who may have relevant expertise
4. **Notify** when new skills are ready for use

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 25 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 20.2 Agent Mail Client Integration

[Code block omitted: Rust example code (types/logic).]
- Block length: 152 line(s).
- Counts: 2 struct(s), 1 enum(s), 0 trait(s), 2 impl(s), 9 fn(s), 17 field(s).
- Struct: AgentMailClient
- Struct: SkillRequestBounty
- Enum: SkillRequestUrgency
- Impl block: AgentMailClient
- Impl block: SkillRequestUrgency
- Function/method: register_skill_builder
- Function/method: announce_build_start
- Function/method: announce_bounty
- Function/method: announce_skill_ready
- Function/method: request_skill
- Function/method: check_skill_in_progress
- Function/method: reserve_patterns
- Function/method: topics_overlap
- Function/method: to_importance
- Field/key: project_key
- Field/key: agent_name
- Field/key: mcp_endpoint
- Field/key: topics
- Field/key: topic
- Field/key: estimated_duration
- Field/key: bounty
- Field/key: skill
- Field/key: quality_score
- Field/key: urgency
- Field/key: builder
- Field/key: started_at
- Field/key: pattern_ids
- Field/key: ttl
- Field/key: amount
- Field/key: currency
- Field/key: Self
- Block defines core structures or algorithms referenced by surrounding text.

**Reservation-Aware Editing (Fallback):**
- If Agent Mail CLI is unavailable, ms provides a local reservation mechanism with
  the same semantics (path/glob, TTL, exclusive/shared).
- When Agent Mail is available, ms bridges to it transparently.

### 20.3 Coordination Protocol

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 26 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 20.4 CLI Commands with Agent Mail

[Code block omitted: ms CLI command examples (8 lines).]
- Unique ms commands: 5.
- Example: ms build (flags: --check-duplicates)
- Example: ms build (flags: --guided, --topic, --coordinate)
- Example: ms build (flags: --guided, --topic, --no-coordinate)
- Example: ms request (flags: --urgency)
- Example: ms request (flags: --urgency, --bounty)
- Example: ms inbox (flags: --requests)
- Example: ms respond (flags: --skill)
- Example: ms subscribe (flags: --timeout)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

### 20.5 Pattern Sharing Between Agents

[Code block omitted: Rust example code (types/logic).]
- Block length: 62 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 7 field(s).
- Struct: PatternSharer
- Impl block: PatternSharer
- Function/method: share_relevant_patterns
- Function/method: receive_patterns
- Field/key: mail_client
- Field/key: local_patterns
- Field/key: recipient
- Field/key: topic
- Field/key: source_agent
- Field/key: patterns
- Field/key: created_at
- Block defines core structures or algorithms referenced by surrounding text.

### 20.6 Multi-Agent Skill Swarm

When building skills at scale with multiple agents (via NTM), coordinate using this pattern:

[Code block omitted: Rust example code (types/logic).]
- Block length: 47 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 2 field(s).
- Struct: SkillSwarm
- Impl block: SkillSwarm
- Function/method: distribute_topics
- Function/method: find_best_agent
- Field/key: agents
- Field/key: topic_allocator
- Block defines core structures or algorithms referenced by surrounding text.

---

## 21. Interactive Build TUI Experience

### 21.1 TUI Layout

The interactive build experience uses a rich terminal UI for guided skill generation:

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 52 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 21.2 TUI Components

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 135 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 21.3 TUI Navigation and Actions

[Code block omitted: Rust example code (types/logic).]
- Block length: 95 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 6 fn(s), 10 field(s).
- Struct: BuildDialogs
- Impl block: BuildTui
- Impl block: BuildDialogs
- Function/method: handle_key
- Function/method: toggle_current_pattern
- Function/method: accept_pattern
- Function/method: edit_pattern_dialog
- Function/method: search_dialog
- Function/method: quit_confirm_dialog
- Field/key: KeyCode
- Field/key: TuiAction
- Field/key: title
- Field/key: fields
- Field/key: EditField
- Field/key: placeholder
- Field/key: filters
- Field/key: SearchFilter
- Field/key: message
- Field/key: options
- Block defines core structures or algorithms referenced by surrounding text.

### 21.4 Real-Time Draft Generation

[Code block omitted: Rust example code (types/logic).]
- Block length: 48 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 9 field(s).
- Struct: LiveDraftGenerator
- Impl block: LiveDraftGenerator
- Function/method: regenerate_preview
- Function/method: estimate_quality
- Field/key: transformer
- Field/key: debounce
- Field/key: last_generation
- Field/key: selected_patterns
- Field/key: current_draft
- Field/key: content
- Field/key: token_count
- Field/key: quality_estimate
- Field/key: diff_from_current
- Block defines core structures or algorithms referenced by surrounding text.

---

## 22. Skill Effectiveness Feedback Loop

### 22.1 Overview

Track whether skills actually help agents accomplish their tasks. This data improves skill quality scores and informs future skill generation.
When multiple variants exist, ms can run A/B experiments to select the most effective version.

**Slice-Level Experiments:**
- Run experiments on individual slices (rule wording, example choice, checklist variant),
  keeping the rest of the skill constant for higher statistical power.
- Uses the same experiment infrastructure but references slice IDs instead of whole skills.

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 24 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 22.2 Usage Tracking

[Code block omitted: Rust example code (types/logic).]
- Block length: 251 line(s).
- Counts: 7 struct(s), 5 enum(s), 0 trait(s), 1 impl(s), 5 fn(s), 43 field(s).
- Struct: EffectivenessTracker
- Struct: SkillExperiment
- Struct: ExperimentVariant
- Struct: SkillUsageEvent
- Struct: RuleOutcome
- Struct: SkillFeedback
- Struct: RuleStat
- Enum: AllocationStrategy
- Enum: ExperimentStatus
- Enum: DiscoveryMethod
- Enum: SessionOutcome
- Enum: FailureReason
- Impl block: EffectivenessTracker
- Function/method: record_skill_load
- Function/method: analyze_session_outcome
- Function/method: infer_outcome
- Function/method: infer_rule_outcomes
- Function/method: get_rule_stats
- Field/key: db
- Field/key: cass
- Field/key: id
- Field/key: skill_id
- Field/key: variants
- Field/key: allocation
- Field/key: started_at
- Field/key: status
- Field/key: variant_id
- Field/key: version
- Field/key: description
- Field/key: session_id
- Field/key: loaded_at
- Field/key: disclosure_level
- Field/key: discovery_method
- Field/key: experiment_id
- Field/key: outcome
- Field/key: feedback
- Field/key: rule_id
- Field/key: followed
- Field/key: duration
- Field/key: quality_signals
- Field/key: completed_aspects
- Field/key: failed_aspects
- Field/key: reason
- Field/key: at_step
- Field/key: progress_percent
- Field/key: rating
- Field/key: positives
- Field/key: improvements
- Field/key: helpful_sections
- Field/key: confusing_sections
- Field/key: level
- Field/key: method
- Field/key: experiment
- Field/key: serde_json
- Field/key: None
- Field/key: usage_event_id
- Field/key: session
- Field/key: skill
- Field/key: SessionOutcome
- Field/key: total
- Field/key: success
- Block defines core structures or algorithms referenced by surrounding text.

### 22.3 Feedback Collection

[Code block omitted: Rust example code (types/logic).]
- Block length: 63 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 6 field(s).
- Struct: FeedbackCollector
- Impl block: FeedbackCollector
- Function/method: collect_feedback_interactive
- Function/method: infer_feedback
- Field/key: tracker
- Field/key: positives
- Field/key: improvements
- Field/key: helpful_sections
- Field/key: confusing_sections
- Field/key: rating
- Block defines core structures or algorithms referenced by surrounding text.

### 22.4 Quality Score Updates

[Code block omitted: Rust example code (types/logic).]
- Block length: 79 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 7 field(s).
- Struct: QualityUpdater
- Impl block: QualityUpdater
- Function/method: update_quality
- Function/method: generate_improvements
- Field/key: scorer
- Field/key: tracker
- Field/key: db
- Field/key: suggestion_type
- Field/key: priority
- Field/key: evidence
- Field/key: section
- Block defines core structures or algorithms referenced by surrounding text.

### 22.4.1 A/B Skill Experiments

When multiple versions of a skill exist (e.g., different wording, structure, or
examples), ms can run A/B experiments to empirically determine the more effective
variant. Results feed back into quality scoring and can automatically promote the
winning version.

[Code block omitted: Rust example code (types/logic).]
- Block length: 56 line(s).
- Counts: 3 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 15 field(s).
- Struct: ExperimentRunner
- Struct: ExperimentResult
- Struct: VariantStats
- Impl block: ExperimentRunner
- Function/method: create_experiment
- Function/method: assign_variant
- Function/method: evaluate
- Field/key: tracker
- Field/key: db
- Field/key: skill_id
- Field/key: variants
- Field/key: allocation
- Field/key: id
- Field/key: started_at
- Field/key: status
- Field/key: serde_json
- Field/key: winner_variant
- Field/key: confidence
- Field/key: stats
- Field/key: uses
- Field/key: success_rate
- Field/key: avg_rating
- Block defines core structures or algorithms referenced by surrounding text.

### 22.5 CLI Commands for Effectiveness

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 29 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

---

## 23. Cross-Project Learning and Coverage Analysis

### 23.1 Overview

Learn from sessions across multiple projects to build more comprehensive skills and identify coverage gaps.

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 32 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 23.2 Cross-Project Pattern Extraction

[Code block omitted: Rust example code (types/logic).]
- Block length: 169 line(s).
- Counts: 4 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 14 field(s).
- Struct: CrossProjectAnalyzer
- Struct: ProjectInfo
- Struct: UniversalPattern
- Struct: ProjectPattern
- Impl block: CrossProjectAnalyzer
- Function/method: find_universal_patterns
- Function/method: find_tech_specific_patterns
- Function/method: normalize_pattern
- Field/key: cass
- Field/key: projects
- Field/key: path
- Field/key: name
- Field/key: tech_stack
- Field/key: session_count
- Field/key: project
- Field/key: original
- Field/key: normalized_pattern
- Field/key: confidence
- Field/key: pattern
- Field/key: project_count
- Field/key: Regex
- Field/key: occurrences
- Block defines core structures or algorithms referenced by surrounding text.

### 23.3 Coverage Gap Analysis

[Code block omitted: Rust example code (types/logic).]
- Block length: 218 line(s).
- Counts: 5 struct(s), 3 enum(s), 0 trait(s), 1 impl(s), 6 fn(s), 23 field(s).
- Struct: CoverageAnalyzer
- Struct: KnowledgeGraph
- Struct: GraphNode
- Struct: GraphEdge
- Struct: CoverageGap
- Enum: NodeType
- Enum: EdgeRelation
- Enum: SkillSuggestion
- Impl block: CoverageAnalyzer
- Function/method: find_gaps
- Function/method: batch_compute_coverage
- Function/method: get_or_build_skill_index
- Function/method: suggest_next_skill
- Function/method: calculate_gap_priority
- Function/method: build_graph
- Field/key: cass
- Field/key: skill_registry
- Field/key: search
- Field/key: nodes
- Field/key: edges
- Field/key: id
- Field/key: node_type
- Field/key: label
- Field/key: tags
- Field/key: from
- Field/key: to
- Field/key: relation
- Field/key: weight
- Field/key: topic
- Field/key: session_count
- Field/key: pattern_count
- Field/key: best_matching_skill
- Field/key: priority
- Field/key: topics
- Field/key: rationale
- Field/key: example_patterns
- Field/key: suggested_tech_stacks
- Field/key: coverage_score
- Block defines core structures or algorithms referenced by surrounding text.

### 23.4 CLI Commands for Coverage

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 38 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

---

## 24. Error Recovery and Resilience

### 24.1 Overview

Robust error handling for long-running autonomous skill generation, including network failures, LLM errors, and system interruptions.

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 15 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 24.2 Error Taxonomy and Retryability Classification

All errors in `ms` are classified by their retryability to prevent wasteful retry attempts and surface permanent failures immediately.

[Code block omitted: Rust example code (types/logic).]
- Block length: 107 line(s).
- Counts: 0 struct(s), 2 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 4 field(s).
- Enum: MsError
- Enum: RetryDecision
- Impl block: MsError
- Function/method: retry_policy
- Function/method: exit_code
- Function/method: hint
- Field/key: provider
- Field/key: retry_after
- Field/key: MsError
- Field/key: RetryDecision
- Block defines core structures or algorithms referenced by surrounding text.

### 24.3 Retry System

[Code block omitted: Rust example code (types/logic).]
- Block length: 65 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 2 fn(s), 10 field(s).
- Struct: RetryConfig
- Struct: RetryExecutor
- Impl block: Default
- Impl block: RetryExecutor
- Function/method: default
- Function/method: execute
- Field/key: max_retries
- Field/key: initial_delay
- Field/key: max_delay
- Field/key: backoff_multiplier
- Field/key: jitter
- Field/key: config
- Field/key: F
- Field/key: E
- Field/key: Duration
- Field/key: tokio
- Block defines core structures or algorithms referenced by surrounding text.

### 24.3 Rate Limit Handler

[Code block omitted: Rust example code (types/logic).]
- Block length: 126 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 5 fn(s), 12 field(s).
- Struct: RateLimitHandler
- Struct: RateLimitState
- Impl block: RateLimitHandler
- Function/method: should_wait
- Function/method: update_from_headers
- Function/method: parse_reset_timestamp
- Function/method: parse_retry_after
- Function/method: execute_with_limits
- Field/key: limits
- Field/key: queue
- Field/key: provider
- Field/key: requests_remaining
- Field/key: tokens_remaining
- Field/key: reset_at
- Field/key: backoff_until
- Field/key: chrono
- Field/key: operation
- Field/key: F
- Field/key: Fut
- Field/key: tokio
- Block defines core structures or algorithms referenced by surrounding text.

### 24.4 Checkpoint Recovery

[Code block omitted: Rust example code (types/logic).]
- Block length: 136 line(s).
- Counts: 3 struct(s), 2 enum(s), 0 trait(s), 1 impl(s), 4 fn(s), 10 field(s).
- Struct: CheckpointRecovery
- Struct: RecoverableSession
- Struct: RecoveryOption
- Enum: RecoveryAction
- Enum: DataLoss
- Impl block: CheckpointRecovery
- Function/method: find_recoverable
- Function/method: is_recoverable
- Function/method: analyze_recovery_options
- Function/method: recover
- Field/key: checkpoint_dir
- Field/key: recovery_options
- Field/key: name
- Field/key: description
- Field/key: action
- Field/key: data_loss
- Field/key: RecoveryAction
- Field/key: starting_phase
- Field/key: preserved_patterns
- Field/key: checkpoint
- Block defines core structures or algorithms referenced by surrounding text.

### 24.5 Graceful Degradation

[Code block omitted: Rust example code (types/logic).]
- Block length: 119 line(s).
- Counts: 3 struct(s), 0 enum(s), 0 trait(s), 3 impl(s), 3 fn(s), 16 field(s).
- Struct: GracefulDegradation
- Struct: HealthEndpoints
- Struct: HealthStatus
- Impl block: Default
- Impl block: GracefulDegradation
- Impl block: Future
- Function/method: default
- Function/method: execute_with_fallback
- Function/method: health_check
- Field/key: cass_available
- Field/key: network_available
- Field/key: cache
- Field/key: health_endpoints
- Field/key: cass
- Field/key: llm_providers
- Field/key: network_probe
- Field/key: primary
- Field/key: fallback
- Field/key: cache_key
- Field/key: T
- Field/key: serde_json
- Field/key: CassClient
- Field/key: reqwest
- Field/key: cache_size
- Field/key: degraded_mode
- Block defines core structures or algorithms referenced by surrounding text.

### 24.6 CLI Commands for Recovery

[Code block omitted: ms CLI command examples (6 lines).]
- Unique ms commands: 1.
- Example: ms build (flags: --check-recoverable)
- Example: ms build (flags: --resume)
- Example: ms build (flags: --resume, --recovery-option)
- Example: ms build (flags: --fresh, --topic)
- Example: ms build (flags: --show-checkpoint)
- Example: ms build (flags: --prune-checkpoints, --older-than)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

---

## 25. Skill Versioning and Migration System

### 25.1 Overview

Track skill versions semantically and provide migration paths when skills evolve.

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 22 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 25.2 Version Data Model

[Code block omitted: Rust example code (types/logic).]
- Block length: 103 line(s).
- Counts: 5 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 18 field(s).
- Struct: SkillVersion
- Struct: BreakingChange
- Struct: VersionHistory
- Struct: Migration
- Struct: MigrationStep
- Enum: MigrationAction
- Impl block: VersionHistory
- Function/method: migration_path
- Function/method: generate_migration_steps
- Field/key: version
- Field/key: changelog
- Field/key: breaking_changes
- Field/key: migration_from
- Field/key: created_at
- Field/key: author
- Field/key: description
- Field/key: migration_hint
- Field/key: affected_sections
- Field/key: skill_id
- Field/key: versions
- Field/key: current
- Field/key: from
- Field/key: to
- Field/key: steps
- Field/key: action
- Field/key: hint
- Field/key: automatic
- Block defines core structures or algorithms referenced by surrounding text.

### 25.3 Version Tracking

[Code block omitted: SQL schema snippet.]
- Block length: 24 line(s).
- Counts: 2 table(s), 2 index(es), 0 trigger(s).
- Table: skill_versions
  - Column: skill_id
  - Column: version
  - Column: changelog
  - Column: breaking_changes_json
  - Column: migration_from
  - Column: created_at
  - Column: author
  - Column: content_hash
- Table: installed_skills
  - Column: skill_id
  - Column: installed_version
  - Column: pinned_version
  - Column: installed_at
  - Column: source
- Index: idx_skill_versions_skill
- Index: idx_installed_source

[Code block omitted: Rust example code (types/logic).]
- Block length: 176 line(s).
- Counts: 1 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 8 fn(s), 19 field(s).
- Struct: VersionManager
- Enum: BumpType
- Impl block: VersionManager
- Function/method: create_version
- Function/method: get_latest_available
- Function/method: get_installed
- Function/method: needs_update
- Function/method: pin
- Function/method: unpin
- Function/method: detect_bump_type
- Function/method: detect_breaking_changes
- Field/key: db
- Field/key: git
- Field/key: skill_id
- Field/key: bump_type
- Field/key: changelog
- Field/key: breaking_changes
- Field/key: BumpType
- Field/key: version
- Field/key: migration_from
- Field/key: created_at
- Field/key: author
- Field/key: serde_json
- Field/key: installed_version
- Field/key: pinned_version
- Field/key: installed_at
- Field/key: source
- Field/key: description
- Field/key: migration_hint
- Field/key: affected_sections
- Block defines core structures or algorithms referenced by surrounding text.

### 25.4 Migration Runner

[Code block omitted: Rust example code (types/logic).]
- Block length: 93 line(s).
- Counts: 4 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 14 field(s).
- Struct: MigrationRunner
- Struct: MigrationPlan
- Struct: MigrationResult
- Struct: ManualStep
- Impl block: MigrationRunner
- Function/method: check_migration_needed
- Function/method: run_migration
- Function/method: execute_step
- Field/key: version_manager
- Field/key: from
- Field/key: to
- Field/key: migrations
- Field/key: manual_steps
- Field/key: success
- Field/key: completed_steps
- Field/key: failed_step
- Field/key: manual_steps_remaining
- Field/key: description
- Field/key: hint
- Field/key: MigrationAction
- Field/key: total_steps
- Field/key: automatic_steps
- Block defines core structures or algorithms referenced by surrounding text.

### 25.5 CLI Commands for Versioning

[Code block omitted: ms CLI command examples (9 lines).]
- Unique ms commands: 1.
- Example: ms version (no flags shown)
- Example: ms version (flags: --minor, --message)
- Example: ms version (flags: --major)
- Example: ms version (no flags shown)
- Example: ms version (no flags shown)
- Example: ms version (flags: --dry-run)
- Example: ms version (no flags shown)
- Example: ms version (no flags shown)
- Example: ms version (no flags shown)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

---

## 26. Real-World Pattern Mining: CASS Insights

This section documents actual patterns discovered by mining CASS sessions. These represent the "inner truths" that `ms build` should extract and transform into skills.

### 26.1 Discovered Skill Candidates

#### Pattern 1: UI Polish Checklist (from brenner_bot sessions)

**Source Sessions:** `/home/ubuntu/.claude/projects/-data-projects-brenner-bot/agent-a9a6d6d.jsonl`

**Recurring Categories:**
[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 20 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Report Format (from sessions):**
[Code block omitted: example block (lang='n/a').]
- Block length: 3 line(s).
- Block contains illustrative content referenced by the surrounding text.

**Inner Truth → Skill:**
[Code block omitted: YAML example.]
- Block length: 3 line(s).
- Keys extracted: 3.
- Key: name
- Key: description
- Key: tags
- Example encodes structured test/spec or config data.

---

#### Pattern 2: Iterative Convergence (from automated_plan_reviser_pro)

**Source Sessions:** `/home/ubuntu/.claude/projects/-data-projects-automated-plan-reviser-pro/`

**The Convergence Pattern:**
> "Specifications improve through multiple iterations like numerical optimization converging to steady state"

**Round Progression Heuristics:**
[Code block omitted: Rust example code (types/logic).]
- Block length: 51 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 5 field(s).
- Struct: ConvergenceProfile
- Impl block: Default
- Function/method: default
- Field/key: round_expectations
- Field/key: rounds
- Field/key: label
- Field/key: expected_changes
- Field/key: focus_areas
- Block defines core structures or algorithms referenced by surrounding text.

**Steady-State Detection:**
[Code block omitted: Rust example code (types/logic).]
- Block length: 25 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 6 field(s).
- Function/method: detect_steady_state
- Field/key: round_outputs
- Field/key: threshold
- Field/key: SteadyStateResult
- Field/key: final_round
- Field/key: current_delta
- Field/key: estimated_rounds_remaining
- Block defines core structures or algorithms referenced by surrounding text.

---

#### Pattern 3: Brenner Principles Extraction (from brenner_bot)

**Methodology Pattern:**
Sessions reveal extraction of "AppliedPrinciples" from specific instances:

[Code block omitted: Rust example code (types/logic).]
- Block length: 28 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 6 field(s).
- Struct: AppliedPrinciple
- Function/method: extract_principles
- Field/key: name
- Field/key: explanation
- Field/key: source_line
- Field/key: confidence
- Field/key: session_content
- Field/key: principle_keywords
- Block defines core structures or algorithms referenced by surrounding text.

**Inner Truth:** Domain expertise can be encoded as keyword → principle mappings, then extracted from sessions automatically.

---

#### Pattern 4: Accessibility Standards (multi-project)

**Recurring Pattern Across Sessions:**
[Code block omitted: example block (lang='typescript').]
- Block length: 12 line(s).
- Block contains illustrative content referenced by the surrounding text.

---

### 26.2 Pattern-to-Skill Transformation Examples

#### Example: UI Polish → Generated Skill

**Input (aggregated from 15+ sessions):**
- 47 instances of touch-manipulation additions
- 23 instances of active:scale-* additions
- 18 instances of focus-visible corrections
- 12 instances of aria-label additions
- 8 instances of useReducedMotion additions

**Generated Skill Draft:**

[Code block omitted: example block (lang='markdown').]
- Block length: 62 line(s).
- Block contains illustrative content referenced by the surrounding text.

---

### 26.3 Cluster Analysis Insights

The CASS searches revealed natural clustering:

| Cluster | Sessions | Key Terms | Potential Skill |
|---------|----------|-----------|-----------------|
| UI Polish | 15+ | touch-manipulation, focus-visible, aria | `ui-polish-nextjs` |
| Accessibility | 12+ | reduced-motion, aria-label, a11y | `react-accessibility` |
| Iterative Refinement | 8+ | rounds, convergence, steady-state | `iterative-spec-refinement` |
| Code Review | 10+ | fresh eyes, systematic, checklist | `code-review-methodology` |
| Error Handling | 7+ | try-catch, Result, error boundary | `error-handling-patterns` |

---

### 26.4 CASS Query Patterns for Skill Mining

**Effective queries discovered:**

[Code block omitted: shell command examples (13 lines).]
- Unique tools referenced: 1.
- Tool invoked: cass
- Commands illustrate typical workflows and integrations.

**Query expansion strategy:**
1. Start with exact phrase: `"inner truth"`
2. Expand to component terms: `inner`, `truth`, `abstract`
3. Add synonyms: `general`, `principles`, `universal`
4. Add domain context: `pattern`, `extract`, `lesson`

---

### 26.5 Inner Truth Extraction Algorithm

Based on session analysis, here's the refined extraction algorithm:

[Code block omitted: Rust example code (types/logic).]
- Block length: 54 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 2 fn(s), 7 field(s).
- Struct: InnerTruthExtractor
- Impl block: Default
- Impl block: InnerTruthExtractor
- Function/method: default
- Function/method: extract
- Field/key: generalization_markers
- Field/key: specificity_markers
- Field/key: min_pattern_occurrences
- Field/key: occurrences
- Field/key: sessions
- Field/key: confidence
- Field/key: suggested_skill_content
- Block defines core structures or algorithms referenced by surrounding text.

---

### 26.6 Future CASS Integration Enhancements

Based on mining experience, these CASS features would improve skill generation:

1. **Semantic clustering API**: `cass cluster --by-topic --limit 10`
2. **Cross-session patterns**: `cass patterns --min-occurrences 3`
3. **Project filtering**: `cass search "query" --workspace /data/projects/brenner_bot`
4. **Time-range filtering**: `cass search "query" --since "2025-12-01"`
5. **Agent filtering**: `cass search "query" --agent claude_code`

---

## 27. Appendix: Raw CASS Mining Results

### A.1 UI Polish Session Excerpts

**Session:** `agent-a9a6d6d.jsonl` (brenner_bot)
**Key Finding:** Comprehensive UI polish report format

[Code block omitted: example block (lang='n/a').]
- Block length: 10 line(s).
- Block contains illustrative content referenced by the surrounding text.

### A.2 Iterative Refinement Session Excerpts

**Session:** automated_plan_reviser_pro exploration
**Key Finding:** Round progression pattern

[Code block omitted: example block (lang='n/a').]
- Block length: 4 line(s).
- Block contains illustrative content referenced by the surrounding text.

### A.3 Accessibility Pattern Excerpts

**Multi-project recurring pattern:**
[Code block omitted: example block (lang='tsx').]
- Block length: 7 line(s).
- Block contains illustrative content referenced by the surrounding text.

---

## Section 28: The Brenner Method for Skill Extraction

*CASS Mining Deep Dive: brenner_bot methodology (P0 bead: meta_skill-4d7)*

### 28.1 Core Insight: Reverse-Engineering Cognitive Architectures

The brenner_bot project provides a methodology for extracting **actionable skills** from CASS sessions. Key insight: **don't summarize—extract the generative grammar**.

> "This is not a summary... It is an attempt to **reverse-engineer the cognitive architecture** that generated those contributions—to find the generative grammar of his thinking."

**Application to meta_skill:** We're not looking for "what happened"—we're looking for **repeatable cognitive moves** that made work successful. These become skills.

### 28.2 The Two Axioms for Skill Extraction

#### Axiom 1: Effective Coding Has a Generative Grammar
Code changes are *generated* by cognitive moves that can be identified and formalized.

#### Axiom 2: Understanding = Ability to Reproduce
A skill is valid only if you can **execute it on new problems**.

### 28.3 The Brenner Loop for Skill Extraction

[Code block omitted: example block (lang='n/a').]
- Block length: 8 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 28.4 Skill Tags (Operator Algebra)

| Tag | Description |
|-----|-------------|
| ProblemSelection | How to pick what to work on |
| HypothesisSlate | Explicit enumeration of approaches |
| ThirdAlternative | Both approaches could be wrong |
| IterativeRefinement | Multi-round improvement |
| RuthlessKill | Abandoning failing approaches |
| Quickie | Pilot experiments to de-risk |
| MaterializationInstinct | "What would I see if true?" |
| InnerTruth | The generalizable principle |

### 28.5 Key Methodological Insights

1. **Seven-Cycle Log Paper Test**: If improvement isn't obvious, skill needs refinement
2. **Multi-Model Triangulation**: Extract from multiple angles, keep convergent patterns
3. **Don't Worry Hypothesis**: Document gaps, don't block on secondary concerns
4. **Exception Quarantine**: Collect failures first, look for patterns before patching

### 28.6 Beads for Further CASS Mining

| Bead | P | Topic |
|------|---|-------|
| meta_skill-4d7 | P0 | Inner Truth/Abstract Principles ✓ |
| meta_skill-hzg | P1 | APR Iterative Refinement |
| meta_skill-897 | P1 | Optimization Patterns |
| meta_skill-z2r | P1 | Performance Profiling |
| meta_skill-dag | P2 | Error Handling |
| meta_skill-f8s | P2 | CI/CD Automation |

### 28.7 Interactive TUI Wizard: `ms mine --guided`

The Brenner extraction loop becomes operable through an interactive TUI that guides users from "some sessions" to "skill + tests" in one flow.

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 15 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

#### CLI Interface

[Code block omitted: ms CLI command examples (3 lines).]
- Unique ms commands: 1.
- Example: ms mine (flags: --guided)
- Example: ms mine (flags: --guided, --query)
- Example: ms mine (flags: --guided, --resume)
- Examples collectively cover init/index, search/suggest/load, build, maintenance, testing, and integrations.

#### TUI Screens

**Screen 1: Session Selection**
[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 16 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Screen 2: Cognitive Move Extraction**
[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 19 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Screen 3: Third-Alternative Guard**
[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 18 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Screen 4: Skill Formalization (Live Editor)**
[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 26 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Screen 5: Materialization Test**
[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 21 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

#### Wizard Output Artifacts

On completion, the wizard produces:

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 6 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

#### Implementation

[Code block omitted: Rust example code (types/logic).]
- Block length: 66 line(s).
- Counts: 1 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 9 field(s).
- Struct: BrennerWizard
- Enum: WizardState
- Impl block: BrennerWizard
- Function/method: run
- Function/method: resume
- Field/key: state
- Field/key: sessions
- Field/key: moves
- Field/key: skill_draft
- Field/key: test_results
- Field/key: WizardState
- Field/key: WizardAction
- Field/key: skill_path
- Field/key: manifest_path
- Block defines core structures or algorithms referenced by surrounding text.

---

## Section 29: APR Iterative Refinement Patterns

*CASS Mining Deep Dive: automated_plan_reviser_pro methodology (P1 bead: meta_skill-hzg)*

### 29.1 The Numerical Optimizer Analogy

The APR project reveals a powerful insight: **iterative specification refinement follows the same dynamics as numerical optimization**.

> "It very much reminds me of a numerical optimizer gradually converging on a steady state after wild swings in the initial iterations."

**Application to meta_skill:** When building skills through CASS mining, expect early iterations to produce wild swings (major restructures, foundational changes). Later iterations converge on stable formulations. Don't judge early work—judge the convergence trajectory.

### 29.2 The Convergence Pattern

Refinement progresses through predictable phases:

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 9 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

| Phase | Rounds | Focus |
|-------|--------|-------|
| **Major Fixes** | 1-3 | Security gaps, architectural flaws, fundamental issues |
| **Architecture** | 4-7 | Interface improvements, component boundaries |
| **Refinement** | 8-12 | Edge cases, optimizations, nuanced handling |
| **Polishing** | 13+ | Final abstractions, converging on steady state |

**Key insight:** In early rounds, reviewers focus on "putting out fires." Once major issues are addressed, they can apply "considerable intellectual energies on nuanced particulars."

### 29.3 Convergence Analytics Algorithm

APR implements a quantitative convergence detector using three weighted signals:

[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.

| Signal | Weight | What It Measures |
|--------|--------|------------------|
| **Output Size Trend** | 35% | Are responses getting shorter? Early rounds produce lengthy analyses; convergence shows as more focused, briefer feedback |
| **Change Velocity** | 35% | Is the rate of change slowing? Measured by comparing delta sizes between consecutive rounds |
| **Content Similarity** | 30% | Are successive rounds becoming more similar? Uses word-level overlap to detect stabilization |

**Interpretation:**
- **Score ≥ 0.75**: High confidence of convergence. The specification is stabilizing.
- **Score 0.50-0.74**: Moderate convergence. Significant work remains but progress is visible.
- **Score < 0.50**: Low convergence. Still in early iteration phase with major changes likely.

**Application to meta_skill:** When refining skills through CASS mining, track these metrics:
1. Are extracted patterns getting shorter/tighter?
2. Is the rate of changes to skill definitions slowing?
3. Are multi-model extractions converging on similar formulations?

### 29.4 Grounded Abstraction Principle

> "Every few rounds, including the implementation document keeps abstract specifications grounded in concrete reality."

**Pattern:** Every 3-4 rounds of abstract refinement, ground the work in concrete implementation:

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 19 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

**Application to meta_skill:** When extracting skills from CASS sessions, periodically test them:
- Can the skill actually be loaded and executed?
- Does the skill produce expected outputs?
- Do agents understand and apply the skill correctly?

### 29.5 Reliability Features for Long Operations

APR implements several reliability patterns for expensive operations:

#### Pre-Flight Validation
Check all preconditions before starting expensive work:
[Code block omitted: example block (lang='n/a').]
- Block length: 5 line(s).
- Block contains illustrative content referenced by the surrounding text.

**Application to meta_skill:** Before running expensive CASS operations:
- Verify index is up-to-date
- Check disk space for embeddings
- Validate query parameters
- Confirm output paths writable

#### Auto-Retry with Exponential Backoff
[Code block omitted: example block (lang='n/a').]
- Block length: 4 line(s).
- Block contains illustrative content referenced by the surrounding text.

**Application to meta_skill:** Retry transient failures (network, rate limits) with increasing delays.

#### Session Locking
Prevent concurrent operations that could cause corruption:
- File-based locks with timestamp
- Automatic stale lock cleanup
- Clear error messages on lock conflict

### 29.6 Dual Interface Pattern

APR serves two audiences with the same codebase:

| Audience | Interface | Features |
|----------|-----------|----------|
| **Humans** | Beautiful TUI | gum styling, interactive wizards, progress indicators, notifications |
| **Machines** | Robot Mode JSON | Structured output, semantic error codes, pre-flight validation |

[Code block omitted: Rust example code (types/logic).]
- Block length: 13 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 2 field(s).
- Struct: OutputMode
- Field/key: human
- Field/key: robot
- Block defines core structures or algorithms referenced by surrounding text.

**Semantic Error Codes:**
- `ok` - Success
- `not_configured` - No configuration found
- `not_found` - Resource doesn't exist
- `validation_failed` - Preconditions not met
- `dependency_missing` - Required dependency unavailable

### 29.7 Audit Trail Principle

Every operation creates artifacts:
- Output files saved to versioned directories
- Git integration for history
- Logs for debugging
- Metrics for analysis

**Application to meta_skill:**
- Every CASS mining session produces artifacts in `.ms_cache/`
- Extracted skills tracked in Git
- Operation logs preserved for debugging
- Convergence metrics stored for analysis

### 29.8 Design Principles Summary

| Principle | Description |
|-----------|-------------|
| **Iterative Convergence** | Like numerical optimization—expect wild swings early, convergence late |
| **Grounded Abstraction** | Periodically ground abstract work in concrete implementation |
| **Audit Trail** | Every operation creates artifacts; history is preserved |
| **Graceful Degradation** | Fallbacks for missing dependencies (gum → ANSI, global → npx) |
| **Dual Interface** | Beautiful for humans, structured for machines |
| **Secure by Default** | No credential storage, checksum verification, atomic operations |

### 29.9 Updated Beads Table

| Bead | P | Topic | Status |
|------|---|-------|--------|
| meta_skill-4d7 | P0 | Inner Truth/Abstract Principles | ✓ Complete |
| meta_skill-hzg | P1 | APR Iterative Refinement | ✓ Complete |
| meta_skill-897 | P1 | Optimization Patterns | Pending |
| meta_skill-z2r | P1 | Performance Profiling | ✓ Complete |
| meta_skill-aku | P1 | Security Vulnerability Assessment | Pending |
| meta_skill-dag | P2 | Error Handling | Pending |
| meta_skill-f8s | P2 | CI/CD Automation | Pending |
| meta_skill-hax | P2 | Caching/Memoization | Pending |
| meta_skill-36x | P2 | Debugging Workflows | Pending |
| meta_skill-avs | P2 | Refactoring Patterns | Pending |
| meta_skill-cbx | P2 | Testing Patterns | Pending |
| meta_skill-6st | P2 | REST API Design | Pending |

## Section 30: Performance Profiling Patterns

*Source: CASS mining of local coding agent sessions - performance analysis workflows*

### 30.1 Introduction

Performance profiling is critical for building efficient CLI tools. This section synthesizes patterns from real-world performance optimization sessions, covering methodology, tooling, benchmarking, and specific optimization techniques.

### 30.2 Performance Analysis Methodology

A comprehensive performance analysis examines multiple dimensions:

#### Hot Path Identification
Focus analysis on the most frequently executed code:
- Query execution and caching
- Vector/similarity search operations
- Full-text index operations
- I/O pipelines (indexing, storage)
- Database operations
- Parser/connector code

#### Inefficiency Pattern Checklist

| Pattern | Description | Detection |
|---------|-------------|-----------|
| **N+1 Queries** | Fetching in loops | Profile shows repeated DB calls |
| **Unnecessary Allocations** | `String::new()`, `Vec::new()` in hot loops | Heap profiling |
| **Repeated Serialization** | serde overhead in loops | CPU profiling |
| **Linear Scans** | O(n) where hash/binary search works | Code review |
| **Lock Contention** | Mutex/RwLock blocking | Contention profiling |
| **Unbounded Collections** | Growing without limits | Memory profiling |
| **Missing Early Termination** | No short-circuiting | Code review |
| **Redundant Computation** | Same calculation repeated | Memoization analysis |
| **String Operations** | Could use interning | Allocation profiling |
| **Iterator Overhead** | Intermediate collections | Inspect `.collect()` calls |
| **Cache Misses** | Poor memory locality | `perf stat` cache metrics |

### 30.3 Algorithm and Data Structure Opportunities

#### Data Structure Upgrades

| Current | Opportunity | Use Case |
|---------|-------------|----------|
| HashMap | Trie | Prefix operations, autocomplete |
| Linear scan | Bloom filter | Fast negative lookups |
| Range queries | Interval tree | Time-range filtering |
| LRU cache | ARC/LFU | Frequency-biased caching |
| Deduplication | Union-find | Graph-based dedup |
| Cumulative ops | Prefix sums | Running totals |
| Sorting | Priority queue | Top-K selection |

#### Top-K Selection Strategies

[Code block omitted: Rust example code (types/logic).]
- Block length: 7 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 0 field(s).
- Block defines core structures or algorithms referenced by surrounding text.

### 30.4 SIMD and Vectorization

#### Memory Layout Considerations

| Layout | Description | SIMD Friendly |
|--------|-------------|---------------|
| **AoS** | Array of Structs: `[{x,y,z}, {x,y,z}]` | ❌ Poor |
| **SoA** | Struct of Arrays: `{xs: [], ys: [], zs: []}` | ✅ Excellent |

[Code block omitted: Rust example code (types/logic).]
- Block length: 6 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 3 field(s).
- Struct: VectorIndex
- Field/key: xs
- Field/key: ys
- Field/key: zs
- Block defines core structures or algorithms referenced by surrounding text.

#### SIMD Dot Product Pattern

[Code block omitted: Rust example code (types/logic).]
- Block length: 16 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: dot_product_simd
- Block defines core structures or algorithms referenced by surrounding text.

#### Quantization (F16 Storage)

[Code block omitted: Rust example code (types/logic).]
- Block length: 9 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 0 field(s).
- Function/method: quantize_vector
- Function/method: dequantize_vector
- Block defines core structures or algorithms referenced by surrounding text.

### 30.5 Criterion Benchmark Patterns

#### Basic Benchmark Structure

[Code block omitted: Rust example code (types/logic).]
- Block length: 13 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: bench_operation
- Block defines core structures or algorithms referenced by surrounding text.

#### Batched Benchmarks (Setup/Teardown Separation)

[Code block omitted: Rust example code (types/logic).]
- Block length: 13 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 1 field(s).
- Field/key: BatchSize
- Block defines core structures or algorithms referenced by surrounding text.

#### Benchmark Groups for Comparison

[Code block omitted: Rust example code (types/logic).]
- Block length: 13 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: bench_scaling
- Block defines core structures or algorithms referenced by surrounding text.

#### Parallel vs Sequential Comparison

[Code block omitted: Rust example code (types/logic).]
- Block length: 22 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: bench_parallelization
- Block defines core structures or algorithms referenced by surrounding text.

### 30.6 Profiling Build Configuration

#### Cargo Profile for Profiling

[Code block omitted: TOML config example.]
- Block length: 7 line(s).
- Counts: 1 section(s), 3 key(s).
- Section: profile.profiling
- Key: inherits
- Key: debug
- Key: strip
- Example shows configuration defaults and feature toggles.

#### Profiling Workflow

[Code block omitted: shell command examples (8 lines).]
- Unique tools referenced: 3.
- Tool invoked: RUSTFLAGS="-C
- Tool invoked: perf
- Tool invoked: cargo
- Commands illustrate typical workflows and integrations.

### 30.7 I/O and Serialization Optimization

#### Memory-Mapped Files

[Code block omitted: Rust example code (types/logic).]
- Block length: 11 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: mmap_read
- Block defines core structures or algorithms referenced by surrounding text.

#### JSON Parsing Optimization

[Code block omitted: Rust example code (types/logic).]
- Block length: 8 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 0 field(s).
- Block defines core structures or algorithms referenced by surrounding text.

### 30.8 Cache Design Patterns

#### LRU Cache with TTL

[Code block omitted: Rust example code (types/logic).]
- Block length: 20 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 2 field(s).
- Struct: TtlCache
- Function/method: get
- Function/method: insert
- Field/key: cache
- Field/key: ttl
- Block defines core structures or algorithms referenced by surrounding text.

#### Fast Hash for Cache Keys

[Code block omitted: Rust example code (types/logic).]
- Block length: 4 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 0 field(s).
- Block defines core structures or algorithms referenced by surrounding text.

### 30.9 Parallel Processing Patterns

#### Rayon Work-Stealing

[Code block omitted: Rust example code (types/logic).]
- Block length: 12 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 1 field(s).
- Field/key: rayon
- Block defines core structures or algorithms referenced by surrounding text.

#### Chunked Processing

[Code block omitted: Rust example code (types/logic).]
- Block length: 10 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 0 field(s).
- Block defines core structures or algorithms referenced by surrounding text.

### 30.10 Application to meta_skill

| Pattern | Application |
|---------|-------------|
| **Hot Path Analysis** | Profile skill extraction, template rendering, search operations |
| **SIMD** | Vectorize embedding comparisons for semantic skill matching |
| **Criterion Benchmarks** | Measure extraction throughput, template performance |
| **Memory-Mapped Files** | Large session file processing |
| **LRU Cache** | Cache parsed sessions, rendered templates |
| **Parallel Processing** | Batch skill extraction across multiple sessions |
| **Profiling Profile** | Enable flamegraph generation for performance debugging |

### 30.11 Benchmark Results Reference

Real-world improvements observed in CASS codebase:

| Optimization | Before | After | Improvement |
|--------------|--------|-------|-------------|
| Sequential → Parallel search | ~63-135ms | ~2.04ms | **30-50x** |
| Scalar → SIMD dot product | baseline | faster | **4-8x** (typical) |
| HashMap → FxHashMap | baseline | faster | **10-30%** (small keys) |
| String → &str where possible | allocating | zero-copy | **significant** |

### 30.12 Profiling Checklist

Before optimization:
- [ ] Establish baseline measurements with criterion
- [ ] Identify actual hot paths (don't guess)
- [ ] Profile under realistic workloads

During optimization:
- [ ] Make one change at a time
- [ ] Measure after each change
- [ ] Verify correctness with tests

After optimization:
- [ ] Document the improvement with benchmarks
- [ ] Add regression tests if performance is critical
- [ ] Consider adding benchmark to CI

---

*Plan version: 1.8.0*
*Created: 2026-01-13*
*Updated: 2026-01-13*
*Author: Claude Opus 4.5*

## Section 31: Optimization Patterns and Methodology

*Source: CASS mining from optimization sessions across multiple codebases*

This section captures systematic optimization methodologies and specific optimization patterns discovered through CASS analysis of real-world performance work.

### 31.1 Optimization Methodology Framework

Before attempting any optimization, follow this disciplined methodology:

#### A) Baseline Establishment

[Code block omitted: shell command examples (6 lines).]
- Unique tools referenced: 2.
- Tool invoked: cargo
- Tool invoked: time
- Commands illustrate typical workflows and integrations.

**Key Principle**: Never optimize without knowing your starting point.

#### B) Profile Before Proposing

[Code block omitted: shell command examples (10 lines).]
- Unique tools referenced: 4.
- Tool invoked: cargo
- Tool invoked: DHAT=1
- Tool invoked: strace
- Tool invoked: perf
- Commands illustrate typical workflows and integrations.

**Anti-pattern**: Optimizing based on intuition rather than profiling data.

#### C) Equivalence Oracle

Define explicit verification criteria before making changes:

[Code block omitted: Rust example code (types/logic).]
- Block length: 27 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 3 field(s).
- Struct: OptimizationOracle
- Impl block: OptimizationOracle
- Function/method: verify
- Field/key: golden_outputs
- Field/key: invariants
- Field/key: tolerance
- Block defines core structures or algorithms referenced by surrounding text.

#### D) Isomorphism Proof Per Change

Every optimization diff must include proof that outputs cannot change:

[Code block omitted: Rust example code (types/logic).]
- Block length: 11 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 0 field(s).
- Block defines core structures or algorithms referenced by surrounding text.

#### E) Opportunity Matrix

Rank optimizations by expected value:

| Opportunity | Impact (1-5) | Confidence (1-5) | Effort (1-5) | Score |
|-------------|--------------|------------------|--------------|-------|
| Replace Vec with SmallVec for N<8 | 3 | 5 | 1 | 15.0 |
| Parallelize with Rayon | 4 | 4 | 2 | 8.0 |
| Switch to FxHashMap | 2 | 5 | 1 | 10.0 |
| Implement SIMD dot product | 4 | 3 | 4 | 3.0 |
| Memory-map large files | 5 | 4 | 3 | 6.7 |

**Formula**: Score = (Impact × Confidence) / Effort

#### F) Minimal Diffs

One performance lever per commit:

[Code block omitted: example block (lang='n/a').]
- Block length: 2 line(s).
- Block contains illustrative content referenced by the surrounding text.

Benefits:
- Easier to measure individual impact
- Easier to bisect regressions
- Easier to revert if problems arise

#### G) Regression Guardrails

Add benchmark thresholds to CI:

[Code block omitted: Rust example code (types/logic).]
- Block length: 14 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: benchmark_critical_path
- Block defines core structures or algorithms referenced by surrounding text.

[Code block omitted: YAML example.]
- Block length: 6 line(s).
- Keys extracted: 2.
- Key: - name
- Key: run
- Example encodes structured test/spec or config data.

### 31.2 Memory Optimization Patterns

#### Zero-Copy Pattern

[Code block omitted: Rust example code (types/logic).]
- Block length: 13 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 1 field(s).
- Function/method: process_data
- Field/key: Cow
- Block defines core structures or algorithms referenced by surrounding text.

#### Buffer Reuse Pattern

[Code block omitted: Rust example code (types/logic).]
- Block length: 24 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 2 field(s).
- Struct: BufferPool
- Impl block: BufferPool
- Function/method: acquire
- Function/method: release
- Field/key: buffers
- Field/key: buffer_size
- Block defines core structures or algorithms referenced by surrounding text.

#### String Interning

[Code block omitted: Rust example code (types/logic).]
- Block length: 18 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 2 field(s).
- Struct: StringInterner
- Impl block: StringInterner
- Function/method: intern
- Field/key: strings
- Field/key: Arc
- Block defines core structures or algorithms referenced by surrounding text.

#### Copy-on-Write (Cow) Pattern

[Code block omitted: Rust example code (types/logic).]
- Block length: 22 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 3 field(s).
- Struct: SkillConfig
- Function/method: default_static
- Function/method: with_name
- Field/key: name
- Field/key: template
- Field/key: tags
- Block defines core structures or algorithms referenced by surrounding text.

#### Structure of Arrays (SoA) vs Array of Structures (AoS)

[Code block omitted: Rust example code (types/logic).]
- Block length: 27 line(s).
- Counts: 5 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 11 field(s).
- Struct: SkillAoS
- Struct: SkillSoA
- Struct: SkillHybrid
- Struct: SkillHot
- Struct: SkillCold
- Field/key: skills
- Field/key: names
- Field/key: descriptions
- Field/key: tags
- Field/key: hot
- Field/key: cold
- Field/key: name
- Field/key: score
- Field/key: description
- Field/key: examples
- Field/key: metadata
- Block defines core structures or algorithms referenced by surrounding text.

### 31.3 Algorithm and Data Structure Optimizations

#### Trie for Prefix Matching

[Code block omitted: Rust example code (types/logic).]
- Block length: 26 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 3 field(s).
- Struct: TrieNode
- Impl block: TrieNode
- Function/method: insert
- Function/method: find_prefix_matches
- Field/key: children
- Field/key: is_end
- Field/key: value
- Block defines core structures or algorithms referenced by surrounding text.

#### Bloom Filter for Membership Testing

[Code block omitted: Rust example code (types/logic).]
- Block length: 26 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 2 field(s).
- Struct: BloomFilter
- Impl block: BloomFilter
- Function/method: insert
- Function/method: may_contain
- Field/key: bits
- Field/key: num_hashes
- Block defines core structures or algorithms referenced by surrounding text.

#### Interval Tree for Range Queries

[Code block omitted: Rust example code (types/logic).]
- Block length: 19 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 6 field(s).
- Struct: IntervalTree
- Struct: IntervalNode
- Function/method: query_overlapping
- Field/key: root
- Field/key: interval
- Field/key: max_end
- Field/key: value
- Field/key: left
- Field/key: right
- Block defines core structures or algorithms referenced by surrounding text.

#### Segment Tree with Lazy Propagation

[Code block omitted: Rust example code (types/logic).]
- Block length: 28 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 3 field(s).
- Struct: SegmentTree
- Impl block: SegmentTree
- Function/method: range_update
- Function/method: range_query
- Function/method: push_down
- Field/key: tree
- Field/key: lazy
- Field/key: n
- Block defines core structures or algorithms referenced by surrounding text.

### 31.4 Advanced Algorithmic Techniques

#### Convex Hull Trick for DP Optimization

[Code block omitted: Rust example code (types/logic).]
- Block length: 33 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 1 field(s).
- Struct: ConvexHullTrick
- Impl block: ConvexHullTrick
- Function/method: add_line
- Function/method: query
- Field/key: lines
- Block defines core structures or algorithms referenced by surrounding text.

#### Matrix Exponentiation for Linear Recurrences

[Code block omitted: Rust example code (types/logic).]
- Block length: 32 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 0 field(s).
- Function/method: matrix_mult
- Function/method: matrix_pow
- Function/method: fibonacci
- Block defines core structures or algorithms referenced by surrounding text.

#### FFT/NTT for Polynomial Multiplication

[Code block omitted: Rust example code (types/logic).]
- Block length: 58 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 0 field(s).
- Function/method: ntt
- Function/method: mod_pow
- Block defines core structures or algorithms referenced by surrounding text.

### 31.5 Lazy Evaluation Patterns

#### Lazy Iterator Chains

[Code block omitted: Rust example code (types/logic).]
- Block length: 17 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: process_skills
- Block defines core structures or algorithms referenced by surrounding text.

#### Lazy Loading with OnceCell

[Code block omitted: Rust example code (types/logic).]
- Block length: 17 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 3 field(s).
- Struct: LazySkillIndex
- Impl block: LazySkillIndex
- Function/method: new
- Function/method: get
- Field/key: path
- Field/key: index
- Field/key: SkillIndex
- Block defines core structures or algorithms referenced by surrounding text.

#### Deferred Computation Pattern

[Code block omitted: Rust example code (types/logic).]
- Block length: 35 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 2 field(s).
- Struct: Deferred
- Function/method: new
- Function/method: get
- Function/method: is_computed
- Field/key: cell
- Field/key: init
- Block defines core structures or algorithms referenced by surrounding text.

### 31.6 Memoization with Invalidation

#### Time-Based Cache Invalidation

[Code block omitted: Rust example code (types/logic).]
- Block length: 23 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 2 field(s).
- Struct: TimedCache
- Function/method: get
- Function/method: insert
- Function/method: evict_expired
- Field/key: entries
- Field/key: ttl
- Block defines core structures or algorithms referenced by surrounding text.

#### Version-Based Invalidation

[Code block omitted: Rust example code (types/logic).]
- Block length: 34 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 5 field(s).
- Struct: VersionedCache
- Struct: FileCache
- Impl block: FileCache
- Function/method: get
- Function/method: invalidate
- Field/key: value
- Field/key: cached_version
- Field/key: F
- Field/key: path
- Field/key: cache
- Block defines core structures or algorithms referenced by surrounding text.

#### Dependency-Based Invalidation

[Code block omitted: Rust example code (types/logic).]
- Block length: 28 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 4 field(s).
- Struct: DependencyCache
- Struct: CacheEntry
- Function/method: invalidate
- Function/method: set_dependency
- Field/key: entries
- Field/key: dependencies
- Field/key: value
- Field/key: valid
- Block defines core structures or algorithms referenced by surrounding text.

### 31.7 I/O Optimization Patterns

#### Scatter-Gather I/O

[Code block omitted: Rust example code (types/logic).]
- Block length: 10 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: write_multiple
- Block defines core structures or algorithms referenced by surrounding text.

#### Buffered I/O with Controlled Flushing

[Code block omitted: Rust example code (types/logic).]
- Block length: 18 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 3 field(s).
- Struct: BatchedWriter
- Function/method: write_item
- Field/key: inner
- Field/key: writes_since_flush
- Field/key: flush_interval
- Block defines core structures or algorithms referenced by surrounding text.

#### Async I/O for Concurrent Operations

[Code block omitted: Rust example code (types/logic).]
- Block length: 15 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: read_all_files
- Block defines core structures or algorithms referenced by surrounding text.

### 31.8 Precomputation Patterns

#### Lookup Tables

[Code block omitted: Rust example code (types/logic).]
- Block length: 24 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 1 field(s).
- Struct: LookupTable
- Impl block: LookupTable
- Function/method: new
- Function/method: to_hex
- Field/key: byte_to_hex
- Block defines core structures or algorithms referenced by surrounding text.

#### Compile-Time Computation

[Code block omitted: Rust example code (types/logic).]
- Block length: 15 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 0 field(s).
- Function/method: compute_factorial
- Function/method: factorial
- Block defines core structures or algorithms referenced by surrounding text.

#### Static Initialization with LazyLock

[Code block omitted: Rust example code (types/logic).]
- Block length: 11 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 1 field(s).
- Function/method: extract_skill
- Field/key: Regex
- Block defines core structures or algorithms referenced by surrounding text.

### 31.9 N+1 Query Elimination

#### Batch Loading Pattern

[Code block omitted: Rust example code (types/logic).]
- Block length: 35 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 2 field(s).
- Struct: SkillRepository
- Impl block: SkillRepository
- Function/method: get_skills_with_tags_bad
- Function/method: get_skills_with_tags_good
- Field/key: db
- Field/key: tags
- Block defines core structures or algorithms referenced by surrounding text.

#### DataLoader Pattern

[Code block omitted: Rust example code (types/logic).]
- Block length: 27 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 3 field(s).
- Struct: DataLoader
- Function/method: load
- Function/method: execute_batch
- Field/key: load_fn
- Field/key: cache
- Field/key: pending
- Block defines core structures or algorithms referenced by surrounding text.

### 31.10 Application to meta_skill

| Pattern | Application |
|---------|-------------|
| **Zero-Copy** | Parse session files without copying string data |
| **Buffer Reuse** | Reuse buffers when processing multiple sessions |
| **String Interning** | Deduplicate skill names and tag names |
| **Trie** | Fast prefix matching for skill/command autocomplete |
| **Bloom Filter** | Quick "no match" checks before expensive extraction |
| **Lazy Loading** | Load skill definitions on demand |
| **Memoization** | Cache extracted skills per session file |
| **Batch Loading** | Load all skills for displayed results in one pass |
| **Precomputation** | Pre-compile regex patterns, build lookup tables |
| **Convex Hull** | Optimize ranking/scoring DP if applicable |

### 31.11 Optimization Decision Flowchart

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 27 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 31.12 Optimization Checklist

Before optimizing:
- [ ] Establish golden outputs / equivalence oracle
- [ ] Profile under realistic workload
- [ ] Identify actual bottleneck (don't guess)
- [ ] Calculate opportunity score: (Impact × Confidence) / Effort

During optimization:
- [ ] One change at a time
- [ ] Document isomorphism proof for each change
- [ ] Verify tests still pass
- [ ] Measure improvement

After optimization:
- [ ] Add regression benchmark
- [ ] Document the optimization for future maintainers
- [ ] Consider if optimization adds complexity worth the gain

## Section 32: Security Vulnerability Assessment Patterns

*Source: CASS mining from security audits and vulnerability assessments across multiple codebases*

This section captures systematic security vulnerability assessment methodologies and specific security patterns discovered through CASS analysis of real-world security work.

### 32.1 Security Audit Methodology Framework

#### Systematic Security Review Process

[Code block omitted: Rust example code (types/logic).]
- Block length: 31 line(s).
- Counts: 1 struct(s), 2 enum(s), 0 trait(s), 0 impl(s), 0 fn(s), 8 field(s).
- Struct: SecurityFinding
- Enum: AuditPhase
- Enum: Severity
- Field/key: title
- Field/key: severity
- Field/key: file_path
- Field/key: line_number
- Field/key: description
- Field/key: proof_of_concept
- Field/key: recommendation
- Field/key: cwe_id
- Block defines core structures or algorithms referenced by surrounding text.

#### Attack Surface Mapping Checklist

[Code block omitted: example block (lang='markdown').]
- Block length: 29 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 32.2 OWASP-Aligned Vulnerability Categories

#### A01: Broken Access Control

[Code block omitted: Rust example code (types/logic).]
- Block length: 30 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 4 field(s).
- Function/method: verify_authorization
- Field/key: user
- Field/key: resource
- Field/key: action
- Field/key: AccessType
- Block defines core structures or algorithms referenced by surrounding text.

#### A02: Cryptographic Failures

[Code block omitted: Rust example code (types/logic).]
- Block length: 58 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 4 fn(s), 5 field(s).
- Function/method: hash_password
- Function/method: encrypt_data
- Function/method: bad_derive_nonce
- Function/method: good_derive_nonce
- Field/key: argon2
- Field/key: key
- Field/key: plaintext
- Field/key: aad
- Field/key: msg
- Block defines core structures or algorithms referenced by surrounding text.

#### A03: Injection

[Code block omitted: Rust example code (types/logic).]
- Block length: 42 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 7 field(s).
- Function/method: get_user_by_email
- Function/method: safe_execute_command
- Function/method: escape_shell_arg
- Field/key: id
- Field/key: email
- Field/key: name
- Field/key: allowed_commands
- Field/key: command
- Field/key: args
- Field/key: Command
- Block defines core structures or algorithms referenced by surrounding text.

#### A04: Insecure Design

[Code block omitted: Rust example code (types/logic).]
- Block length: 48 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 8 field(s).
- Struct: SecureSession
- Impl block: SecureSession
- Function/method: new
- Function/method: validate
- Field/key: id
- Field/key: user_id
- Field/key: created_at
- Field/key: expires_at
- Field/key: last_activity
- Field/key: ip_address
- Field/key: user_agent_hash
- Field/key: log
- Block defines core structures or algorithms referenced by surrounding text.

#### A05: Security Misconfiguration

[Code block omitted: Rust example code (types/logic).]
- Block length: 44 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 7 field(s).
- Struct: SecurityConfig
- Impl block: SecurityConfig
- Function/method: validate
- Function/method: load_secret
- Field/key: cors_origins
- Field/key: rate_limit
- Field/key: tls
- Field/key: secrets
- Field/key: log
- Field/key: std
- Field/key: ConfigError
- Block defines core structures or algorithms referenced by surrounding text.

### 32.3 Input Validation Patterns

#### Path Traversal Prevention

[Code block omitted: Rust example code (types/logic).]
- Block length: 56 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 1 field(s).
- Function/method: validate_path
- Function/method: sanitize_filename
- Field/key: Component
- Block defines core structures or algorithms referenced by surrounding text.

#### XSS Prevention

[Code block omitted: Rust example code (types/logic).]
- Block length: 42 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 1 field(s).
- Function/method: escape_html
- Function/method: csp_header
- Function/method: sanitize_html
- Field/key: ammonia
- Block defines core structures or algorithms referenced by surrounding text.

### 32.4 Authentication Security Patterns

#### JWT Token Management

[Code block omitted: Rust example code (types/logic).]
- Block length: 94 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 17 field(s).
- Struct: AccessTokenClaims
- Struct: TokenPair
- Function/method: generate_access_token
- Function/method: validate_access_token
- Function/method: refresh_tokens
- Field/key: sub
- Field/key: email
- Field/key: tier
- Field/key: iss
- Field/key: aud
- Field/key: iat
- Field/key: exp
- Field/key: user
- Field/key: secret
- Field/key: issuer
- Field/key: audience
- Field/key: token
- Field/key: access_token
- Field/key: refresh_token
- Field/key: refresh_token_id
- Field/key: old_refresh_token
- Field/key: db
- Block defines core structures or algorithms referenced by surrounding text.

#### OAuth Security

[Code block omitted: Rust example code (types/logic).]
- Block length: 46 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 3 field(s).
- Struct: PkceChallenge
- Impl block: PkceChallenge
- Function/method: validate_redirect_url
- Function/method: generate
- Function/method: verify
- Field/key: code_verifier
- Field/key: code_challenge
- Field/key: code_challenge_method
- Block defines core structures or algorithms referenced by surrounding text.

### 32.5 Rate Limiting and DoS Protection

#### IP-Based Rate Limiting

[Code block omitted: Rust example code (types/logic).]
- Block length: 75 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 9 field(s).
- Struct: RateLimiter
- Struct: RateLimitEntry
- Impl block: RateLimiter
- Function/method: check
- Function/method: extract_client_ip
- Field/key: limits
- Field/key: max_requests
- Field/key: window
- Field/key: max_entries
- Field/key: count
- Field/key: window_start
- Field/key: retry_after
- Field/key: request
- Field/key: trusted_proxies
- Block defines core structures or algorithms referenced by surrounding text.

#### ReDoS (Regex Denial of Service) Protection

[Code block omitted: Rust example code (types/logic).]
- Block length: 25 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 2 field(s).
- Struct: SafeRegex
- Impl block: SafeRegex
- Function/method: new
- Function/method: is_match
- Field/key: inner
- Field/key: max_input_len
- Block defines core structures or algorithms referenced by surrounding text.

### 32.6 Secret Management

#### Environment Variable Security

[Code block omitted: Rust example code (types/logic).]
- Block length: 30 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 1 field(s).
- Struct: Secret
- Impl block: Secret
- Function/method: from_env
- Function/method: expose
- Function/method: validate_required_secrets
- Field/key: std
- Block defines core structures or algorithms referenced by surrounding text.

#### API Key Best Practices

[Code block omitted: Rust example code (types/logic).]
- Block length: 30 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 3 field(s).
- Struct: ApiClient
- Impl block: ApiClient
- Function/method: bad_api_call
- Function/method: good_api_call
- Function/method: from_env
- Field/key: reqwest
- Field/key: base_url
- Field/key: secret
- Block defines core structures or algorithms referenced by surrounding text.

### 32.7 Command Execution Security

#### Safe Command Execution Patterns

[Code block omitted: Rust example code (types/logic).]
- Block length: 74 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 8 field(s).
- Struct: CommandExecutor
- Impl block: CommandExecutor
- Function/method: execute
- Function/method: validate_argument
- Function/method: analyze_heredoc
- Field/key: allowed_commands
- Field/key: allowed_cwd
- Field/key: command
- Field/key: args
- Field/key: cwd
- Field/key: Command
- Field/key: severity
- Field/key: description
- Block defines core structures or algorithms referenced by surrounding text.

### 32.8 Security Audit Report Template

[Code block omitted: example block (lang='markdown').]
- Block length: 9 line(s).
- Block contains illustrative content referenced by the surrounding text.
// Vulnerable code snippet
[Code block omitted: example block (lang='n/a').]
- Block length: 3 line(s).
- Block contains illustrative content referenced by the surrounding text.
// Fixed code snippet
[Code block omitted: example block (lang='n/a').]
- Block length: 26 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 32.9 Application to meta_skill

| Security Area | Application |
|---------------|-------------|
| **Input Validation** | Validate skill file paths, template inputs, user queries |
| **Path Traversal** | Protect skill repository access, session file access |
| **Command Injection** | Safe execution of skill commands, template rendering |
| **Secret Management** | API keys for external services, embedding model credentials |
| **Authentication** | User sessions for skill customization (if applicable) |
| **Rate Limiting** | Prevent abuse of skill extraction endpoints |
| **Crypto** | Secure storage of user preferences, session encryption |

### 32.10 Security Checklist

Before deployment:
- [ ] All user inputs validated and sanitized
- [ ] SQL queries parameterized (no string interpolation)
- [ ] Command execution whitelisted and argument-validated
- [ ] Path traversal attacks prevented
- [ ] XSS outputs properly escaped
- [ ] Secrets loaded from environment, not hardcoded
- [ ] Rate limiting implemented on public endpoints
- [ ] Authentication tokens properly validated
- [ ] HTTPS enforced in production
- [ ] Security headers configured (CSP, HSTS, X-Frame-Options)
- [ ] Error messages don't leak sensitive information
- [ ] Logging doesn't include secrets or PII
- [ ] Dependencies scanned for known vulnerabilities

---

## Section 33: Error Handling Patterns and Methodology

*CASS-mined insights on robust error handling in Rust applications*

### 33.1 Error Handling Philosophy

Error handling in Rust differs fundamentally from exceptions in other languages. The key principles:

1. **Errors are values** - `Result<T, E>` makes errors explicit and composable
2. **Fail loudly, recover gracefully** - Errors should be visible but recoverable
3. **Context over raw messages** - Error chains explain *why*, not just *what*
4. **Match error types to boundaries** - Different error types for different layers

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 8 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

### 33.2 The thiserror and anyhow Dichotomy

**thiserror** is for library code - create specific, matchable error types:

[Code block omitted: Rust example code (types/logic).]
- Block length: 28 line(s).
- Counts: 0 struct(s), 1 enum(s), 0 trait(s), 1 impl(s), 1 fn(s), 1 field(s).
- Enum: SkillError
- Impl block: From
- Function/method: from
- Field/key: SkillError
- Block defines core structures or algorithms referenced by surrounding text.

**anyhow** is for application code - rich context chains without ceremony:

[Code block omitted: Rust example code (types/logic).]
- Block length: 30 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: load_skill
- Block defines core structures or algorithms referenced by surrounding text.

### 33.3 Structured CLI Error Types

For CLI applications, create a structured error type that maps to exit codes:

[Code block omitted: Rust example code (types/logic).]
- Block length: 89 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 6 fn(s), 6 field(s).
- Struct: CliError
- Impl block: CliError
- Impl block: Into
- Function/method: usage
- Function/method: not_found
- Function/method: network
- Function/method: timeout
- Function/method: internal
- Function/method: to_json
- Field/key: code
- Field/key: kind
- Field/key: message
- Field/key: hint
- Field/key: retryable
- Field/key: serde_json
- Block defines core structures or algorithms referenced by surrounding text.

### 33.4 Error Taxonomy Patterns

For protocol or API libraries, define a comprehensive error taxonomy:

[Code block omitted: Rust example code (types/logic).]
- Block length: 99 line(s).
- Counts: 1 struct(s), 0 enum(s), 0 trait(s), 2 impl(s), 6 fn(s), 6 field(s).
- Struct: FcpError
- Impl block: FcpError
- Impl block: Into
- Function/method: protocol
- Function/method: auth
- Function/method: rate_limited
- Function/method: is_protocol_error
- Function/method: is_auth_error
- Function/method: is_external_error
- Field/key: code
- Field/key: message
- Field/key: retryable
- Field/key: retry_after_ms
- Field/key: details
- Field/key: ai_recovery_hint
- Block defines core structures or algorithms referenced by surrounding text.

### 33.5 Error Context Chaining

Build rich error chains that explain the full failure path:

[Code block omitted: Rust example code (types/logic).]
- Block length: 38 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 4 field(s).
- Impl block: From
- Function/method: execute_skill
- Function/method: from
- Field/key: TemplateError
- Field/key: SkillError
- Field/key: line
- Field/key: reason
- Block defines core structures or algorithms referenced by surrounding text.

### 33.6 Error Recovery Patterns

Implement retry logic with exponential backoff:

[Code block omitted: Rust example code (types/logic).]
- Block length: 152 line(s).
- Counts: 2 struct(s), 1 enum(s), 0 trait(s), 2 impl(s), 6 fn(s), 15 field(s).
- Struct: RetryConfig
- Struct: CircuitBreaker
- Enum: CircuitState
- Impl block: Default
- Impl block: CircuitBreaker
- Function/method: default
- Function/method: with_retry
- Function/method: new
- Function/method: allow_request
- Function/method: record_success
- Function/method: record_failure
- Field/key: max_retries
- Field/key: initial_delay
- Field/key: max_delay
- Field/key: backoff_factor
- Field/key: jitter
- Field/key: config
- Field/key: F
- Field/key: Fut
- Field/key: E
- Field/key: tracing
- Field/key: failure_threshold
- Field/key: reset_timeout
- Field/key: state
- Field/key: CircuitState
- Field/key: opened_at
- Block defines core structures or algorithms referenced by surrounding text.

### 33.7 Panic vs Result Guidelines

**When to use panic (via `unwrap`, `expect`, `unreachable!`):**

[Code block omitted: Rust example code (types/logic).]
- Block length: 24 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 2 field(s).
- Function/method: pop_from_non_empty_stack
- Function/method: test_parsing
- Field/key: Regex
- Field/key: State
- Block defines core structures or algorithms referenced by surrounding text.

**When to use Result (proper error handling):**

[Code block omitted: Rust example code (types/logic).]
- Block length: 31 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 2 field(s).
- Function/method: parse_config
- Function/method: read_settings
- Function/method: fetch_data
- Field/key: toml
- Field/key: reqwest
- Block defines core structures or algorithms referenced by surrounding text.

### 33.8 Error Boundary Patterns

For systems with multiple error domains, create clear boundaries:

[Code block omitted: Rust example code (types/logic).]
- Block length: 58 line(s).
- Counts: 0 struct(s), 2 enum(s), 0 trait(s), 2 impl(s), 2 fn(s), 4 field(s).
- Enum: LibraryError
- Enum: AppError
- Impl block: From
- Impl block: AppError
- Function/method: from
- Function/method: display
- Field/key: LibraryError
- Field/key: message
- Field/key: hint
- Field/key: AppError
- Block defines core structures or algorithms referenced by surrounding text.

### 33.9 Error Logging Best Practices

[Code block omitted: Rust example code (types/logic).]
- Block length: 36 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 3 field(s).
- Function/method: save_file
- Function/method: handle_error
- Function/method: log_structured_error
- Field/key: std
- Field/key: AppError
- Field/key: tracing
- Block defines core structures or algorithms referenced by surrounding text.

### 33.10 Application to meta_skill

| Error Category | Pattern | Example |
|----------------|---------|---------|
| **Skill Loading** | `thiserror` enum | `SkillError::NotFound`, `SkillError::ParseFailed` |
| **CLI Interface** | `CliError` struct | Exit codes, hints, retryable flags |
| **Template Rendering** | `anyhow` context | Rich failure chains |
| **External APIs** | `FcpError` taxonomy | Error codes, retry hints |
| **Network Operations** | Retry with backoff | `with_retry()` function |
| **Service Stability** | Circuit breaker | Prevent cascading failures |

### 33.11 Error Handling Checklist

Before shipping error handling:
- [ ] All public API functions return `Result` (not `Option` for errors)
- [ ] Error types are appropriate for the layer (library vs application)
- [ ] Error messages are user-friendly (no raw technical jargon)
- [ ] Errors include actionable hints where possible
- [ ] Sensitive information is not leaked in error messages
- [ ] Errors are logged with appropriate severity
- [ ] Retryable errors are clearly marked
- [ ] Exit codes follow Unix conventions
- [ ] Error chains preserve the full context
- [ ] Panics only occur for programming errors, not user input
- [ ] Circuit breakers protect against cascading failures
- [ ] Retry logic includes jitter and backoff

## Section 34: Testing Patterns and Methodology

### 34.1 Testing Philosophy

#### The "NO Mocks" Principle

**Source**: CASS mining of brenner_bot testing landscape analysis

The observed philosophy is: **"NO mocks - test real implementations with real data fixtures."**

[Code block omitted: example block (lang='n/a').]
- Block length: 9 line(s).
- Block contains illustrative content referenced by the surrounding text.

**When to mock**:
1. **Animations**: Mock framer-motion to avoid flaky timing-dependent tests
2. **External APIs**: Stub fetch only when testing HTTP client behavior, not when testing business logic
3. **Time-dependent operations**: Use fakeable clocks, not mocked functions

**When NOT to mock**:
1. File system operations - use `t.TempDir()` (Go) or `mkdtempSync()` (JS)
2. Database operations - use in-memory SQLite or real temp databases
3. Internal functions - if you need to mock internal functions, the design needs refactoring

### 34.2 Test Organization Patterns

#### JavaScript/TypeScript (Vitest/Jest/Bun)

[Code block omitted: example block (lang='typescript').]
- Block length: 49 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### Go Table-Driven Tests

[Code block omitted: example block (lang='go').]
- Block length: 36 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### Test File Naming Conventions

| Language | Pattern | Example |
|----------|---------|---------|
| **TypeScript** | `*.test.ts`, `*.test.tsx` | `copy.test.ts`, `Button.test.tsx` |
| **Go** | `*_test.go` | `evaluator_test.go` |
| **Rust** | `mod tests` in same file, or `/tests/*.rs` | `mod tests { ... }` |
| **Bash** | `*.bats` (BATS framework) | `test_utils.bats` |

### 34.3 Test Fixture Patterns

#### Real Filesystem Fixtures

[Code block omitted: example block (lang='typescript').]
- Block length: 32 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### Go Test Fixtures with t.TempDir()

[Code block omitted: example block (lang='go').]
- Block length: 18 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### Environment Variable Isolation

[Code block omitted: example block (lang='typescript').]
- Block length: 14 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 34.4 Property-Based Testing

#### Rust with proptest

**Source**: CASS mining of destructive_command_guard property tests

[Code block omitted: Rust example code (types/logic).]
- Block length: 55 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 5 fn(s), 0 field(s).
- Impl block: Strategy
- Function/method: command_strategy
- Function/method: normalization_is_idempotent
- Function/method: evaluation_is_deterministic
- Function/method: evaluation_never_panics
- Function/method: handles_large_inputs
- Block defines core structures or algorithms referenced by surrounding text.

#### Key Property Test Categories

| Property | What it tests | Example assertion |
|----------|---------------|-------------------|
| **Idempotence** | `f(f(x)) == f(x)` | Normalization, formatting |
| **Determinism** | Same input → same output | Evaluation, parsing |
| **Safety** | Never panics on any input | Error handling |
| **Bounds** | Handles edge sizes gracefully | Large inputs, empty inputs |
| **Commutativity** | Order doesn't matter | Set operations |
| **Invertibility** | `decode(encode(x)) == x` | Serialization |

### 34.5 Test Coverage Analysis

#### Comprehensive Coverage Report Pattern

**Source**: CASS mining of Go codebase coverage analysis

[Code block omitted: example block (lang='markdown').]
- Block length: 25 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 34.6 Snapshot Testing

#### Vitest/Jest Snapshot Pattern

[Code block omitted: example block (lang='typescript').]
- Block length: 15 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### Managing Snapshot Updates

[Code block omitted: shell command examples (8 lines).]
- Unique tools referenced: 2.
- Tool invoked: bun
- Tool invoked: git
- Commands illustrate typical workflows and integrations.

### 34.7 E2E Testing Patterns

#### Playwright Configuration

**Source**: CASS mining of brenner_bot E2E test infrastructure

[Code block omitted: example block (lang='typescript').]
- Block length: 41 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### E2E Test Structure

[Code block omitted: example block (lang='typescript').]
- Block length: 25 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 34.8 BATS Framework for Shell Testing

**Source**: CASS mining of APR BATS test infrastructure

#### Test Helper Structure

[Code block omitted: shell command examples (29 lines).]
- Unique tools referenced: 15.
- Tool invoked: load
- Tool invoked: setup_test_environment()
- Tool invoked: export
- Tool invoked: mkdir
- Tool invoked: }
- Tool invoked: teardown_test_environment()
- Tool invoked: rm
- Tool invoked: capture_streams()
- Tool invoked: local
- Tool invoked: STDOUT_FILE="$(mktemp)"
- Tool invoked: STDERR_FILE="$(mktemp)"
- Tool invoked: eval
- Tool invoked: EXIT_CODE=$?
- Tool invoked: CAPTURED_STDOUT="$(cat
- Tool invoked: CAPTURED_STDERR="$(cat
- Commands illustrate typical workflows and integrations.

#### Custom Assertions

[Code block omitted: shell command examples (28 lines).]
- Unique tools referenced: 13.
- Tool invoked: assert_stderr_only()
- Tool invoked: assert
- Tool invoked: }
- Tool invoked: assert_stdout_only()
- Tool invoked: assert_valid_json()
- Tool invoked: echo
- Tool invoked: assert_success
- Tool invoked: assert_json_value()
- Tool invoked: local
- Tool invoked: actual=$(echo
- Tool invoked: assert_equal
- Tool invoked: assert_no_ansi()
- Tool invoked: refute_output
- Commands illustrate typical workflows and integrations.

#### Unit Test Example

[Code block omitted: shell command examples (29 lines).]
- Unique tools referenced: 13.
- Tool invoked: setup()
- Tool invoked: load
- Tool invoked: setup_test_environment
- Tool invoked: source
- Tool invoked: }
- Tool invoked: teardown()
- Tool invoked: teardown_test_environment
- Tool invoked: @test
- Tool invoked: run
- Tool invoked: assert_success
- Tool invoked: assert_failure
- Tool invoked: assert_output
- Tool invoked: export
- Commands illustrate typical workflows and integrations.

### 34.9 Real Clipboard Testing

**Source**: CASS mining of jeffreysprompts.com copy command tests

[Code block omitted: example block (lang='typescript').]
- Block length: 66 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 34.10 Test Harness Pattern

**Source**: CASS mining of Go testutil.Harness pattern

[Code block omitted: example block (lang='go').]
- Block length: 66 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 34.11 CI Integration Patterns

#### JUnit XML Output for CI

[Code block omitted: shell command examples (40 lines).]
- Unique tools referenced: 22.
- Tool invoked: set
- Tool invoked: SCRIPT_DIR="$(cd
- Tool invoked: cd
- Tool invoked: preflight_check()
- Tool invoked: echo
- Tool invoked: if
- Tool invoked: exit
- Tool invoked: fi
- Tool invoked: for
- Tool invoked: done
- Tool invoked: }
- Tool invoked: run_tests()
- Tool invoked: local
- Tool invoked: ./libs/bats-core/bin/bats
- Tool invoked: --formatter
- Tool invoked: --output
- Tool invoked: --tap
- Tool invoked: unit/*.bats
- Tool invoked: |
- Tool invoked: return
- Tool invoked: preflight_check
- Tool invoked: run_tests
- Commands illustrate typical workflows and integrations.

#### GitHub Actions Integration

[Code block omitted: YAML example.]
- Block length: 26 line(s).
- Keys extracted: 14.
- Key: name
- Key: on
- Key: jobs
- Key: test
- Key: runs-on
- Key: steps
- Key: - uses
- Key: with
- Key: submodules
- Key: - name
- Key: uses
- Key: run
- Key: if
- Key: path
- Example encodes structured test/spec or config data.

### 34.12 Application to meta_skill

| Test Type | Pattern | Example |
|-----------|---------|---------|
| **Unit Tests** | Table-driven, real fixtures | Skill parsing, template rendering |
| **Integration Tests** | Real filesystem, temp dirs | Skill installation, config loading |
| **Property Tests** | proptest invariants | Template normalization, path handling |
| **E2E Tests** | CLI subprocess | Full workflow: search → select → copy |
| **Snapshot Tests** | Output verification | Rendered skill content |

### 34.13 Testing Checklist

Before shipping tests:
- [ ] Tests use real implementations, not mocks (except for animations/network)
- [ ] File operations use temp directories (`t.TempDir()` or `mkdtempSync()`)
- [ ] Environment variables are isolated and restored
- [ ] Tests are deterministic (no reliance on timing, random values, or external state)
- [ ] Property tests cover invariants (idempotence, determinism, safety)
- [ ] Table-driven tests cover edge cases systematically
- [ ] E2E tests run on multiple browsers/platforms if applicable
- [ ] CI produces JUnit/TAP output for test reporting
- [ ] Flaky tests are marked or fixed (use `it.skip` with explanation)
- [ ] Test coverage gaps are documented and prioritized
- [ ] Snapshot tests are reviewed when updated
- [ ] Tests are organized by type (unit/, integration/, e2e/)

---

## Section 35: CI/CD Automation Patterns

**Source**: CASS mining of repo_updater, apr, jeffreysprompts_premium, flywheel_gateway, and destructive_command_guard CI/CD implementations

### 35.1 GitHub Actions Workflow Architecture

#### Workflow File Organization

**Source**: CASS mining of production GitHub Actions setups

| Workflow File | Purpose | Trigger |
|---------------|---------|---------|
| `ci.yml` | Continuous integration (lint, test, build) | push, pull_request, workflow_dispatch |
| `release.yml` | Release automation with artifacts | `tags: ['v*']` |
| `deploy.yml` | Production deployment | `tags: ['v*']` or manual |
| `e2e.yml` | Full E2E test suite | push to main |
| `dependabot.yml` | Automated dependency updates | schedule |

[Code block omitted: YAML example.]
- Block length: 103 line(s).
- Keys extracted: 33.
- Key: name
- Key: on
- Key: push
- Key: branches
- Key: pull_request
- Key: workflow_dispatch
- Key: jobs
- Key: shellcheck
- Key: runs-on
- Key: steps
- Key: - uses
- Key: with
- Key: severity
- Key: scandir
- Key: additional_files
- Key: syntax
- Key: - name
- Key: run
- Key: tests
- Key: needs
- Key: strategy
- Key: matrix
- Key: os
- Key: fail-fast
- Key: if
- Key: continue-on-error
- Key: uses
- Key: path
- Key: retention-days
- Key: install-test
- Key: version-check
- Key: echo "
- Key: echo "Version verified
- Example encodes structured test/spec or config data.

### 35.2 Job Dependencies and Ordering

#### Dependency Graph Patterns

[Code block omitted: ASCII diagram illustrating a workflow/system layout.]
- Block length: 5 line(s).
- Diagram uses box-and-arrow flow to show major steps, loops, or dependencies.
- Intended to give a visual map of the process described in nearby text.

[Code block omitted: YAML example.]
- Block length: 18 line(s).
- Keys extracted: 8.
- Key: jobs
- Key: lint
- Key: runs-on
- Key: test
- Key: needs
- Key: build
- Key: deploy
- Key: if
- Example encodes structured test/spec or config data.

#### Conditional Execution

[Code block omitted: YAML example.]
- Block length: 15 line(s).
- Keys extracted: 7.
- Key: jobs
- Key: deploy
- Key: runs-on
- Key: if
- Key: steps
- Key: - name
- Key: run
- Example encodes structured test/spec or config data.

### 35.3 Release Automation

#### Tag-Triggered Releases

**Source**: CASS mining of repo_updater release workflow

[Code block omitted: YAML example.]
- Block length: 69 line(s).
- Keys extracted: 25.
- Key: name
- Key: on
- Key: push
- Key: tags
- Key: permissions
- Key: contents
- Key: jobs
- Key: release
- Key: runs-on
- Key: steps
- Key: - uses
- Key: - name
- Key: id
- Key: run
- Key: echo "
- Key: echo "Version verified
- Key: echo "Generated checksums
- Key: curl -fsSL https
- Key: echo "$(curl -fsSL https
- Key: uses
- Key: with
- Key: body_path
- Key: files
- Key: generate_release_notes
- Key: append_body
- Example encodes structured test/spec or config data.

### 35.4 Version Management Patterns

#### Dual Version Storage

**Source**: CASS mining of repo_updater version management

[Code block omitted: shell command examples (14 lines).]
- Unique tools referenced: 11.
- Tool invoked: 1.2.1
- Tool invoked: VERSION="1.2.1"
- Tool invoked: get_version()
- Tool invoked: local
- Tool invoked: script_dir="$(dirname
- Tool invoked: if
- Tool invoked: cat
- Tool invoked: else
- Tool invoked: echo
- Tool invoked: fi
- Tool invoked: }
- Commands illustrate typical workflows and integrations.

#### Semantic Version Comparison

[Code block omitted: shell command examples (30 lines).]
- Unique tools referenced: 13.
- Tool invoked: version_gt()
- Tool invoked: local
- Tool invoked: IFS='.'
- Tool invoked: for
- Tool invoked: if
- Tool invoked: return
- Tool invoked: elif
- Tool invoked: fi
- Tool invoked: done
- Tool invoked: }
- Tool invoked: check_for_update()
- Tool invoked: latest_version=$(curl
- Tool invoked: echo
- Commands illustrate typical workflows and integrations.

### 35.5 Matrix Testing Strategies

#### Multi-OS Matrix

[Code block omitted: YAML example.]
- Block length: 14 line(s).
- Keys extracted: 12.
- Key: jobs
- Key: test
- Key: strategy
- Key: matrix
- Key: os
- Key: node-version
- Key: fail-fast
- Key: runs-on
- Key: steps
- Key: - uses
- Key: with
- Key: - run
- Example encodes structured test/spec or config data.

#### Browser Matrix for E2E

**Source**: CASS mining of jeffreysprompts_premium E2E workflow

[Code block omitted: YAML example.]
- Block length: 25 line(s).
- Keys extracted: 17.
- Key: jobs
- Key: e2e
- Key: strategy
- Key: matrix
- Key: browser
- Key: include
- Key: - browser
- Key: project
- Key: fail-fast
- Key: runs-on
- Key: steps
- Key: - uses
- Key: - run
- Key: if
- Key: with
- Key: name
- Key: path
- Example encodes structured test/spec or config data.

### 35.6 Container Image Pipelines

#### Multi-Stage Dockerfile with CI

**Source**: CASS mining of flywheel_gateway tenant container pipeline

[Code block omitted: example block (lang='dockerfile').]
- Block length: 13 line(s).
- Block contains illustrative content referenced by the surrounding text.

[Code block omitted: YAML example.]
- Block length: 75 line(s).
- Keys extracted: 39.
- Key: name
- Key: on
- Key: push
- Key: tags
- Key: pull_request
- Key: paths
- Key: env
- Key: REGISTRY
- Key: IMAGE_NAME
- Key: jobs
- Key: build
- Key: runs-on
- Key: permissions
- Key: contents
- Key: packages
- Key: steps
- Key: - uses
- Key: - name
- Key: uses
- Key: if
- Key: with
- Key: registry
- Key: username
- Key: password
- Key: id
- Key: images
- Key: context
- Key: platforms
- Key: labels
- Key: cache-from
- Key: cache-to
- Key: image-ref
- Key: format
- Key: output
- Key: severity
- Key: sarif_file
- Key: image
- Key: output-file
- Key: path
- Example encodes structured test/spec or config data.

### 35.7 Artifact Management

#### Upload and Download Patterns

[Code block omitted: YAML example.]
- Block length: 34 line(s).
- Keys extracted: 17.
- Key: jobs
- Key: build
- Key: runs-on
- Key: steps
- Key: - uses
- Key: - run
- Key: - name
- Key: uses
- Key: with
- Key: name
- Key: path
- Key: retention-days
- Key: if-no-files-found
- Key: test
- Key: needs
- Key: deploy
- Key: run
- Example encodes structured test/spec or config data.

#### Caching Dependencies

[Code block omitted: YAML example.]
- Block length: 34 line(s).
- Keys extracted: 14.
- Key: jobs
- Key: test
- Key: runs-on
- Key: steps
- Key: - uses
- Key: with
- Key: node-version
- Key: cache
- Key: bun-version
- Key: - name
- Key: uses
- Key: path
- Key: key
- Key: restore-keys
- Example encodes structured test/spec or config data.

### 35.8 Automated Dependency Updates

#### Dependabot Configuration

[Code block omitted: YAML example.]
- Block length: 42 line(s).
- Keys extracted: 16.
- Key: version
- Key: updates
- Key: - package-ecosystem
- Key: directory
- Key: schedule
- Key: interval
- Key: day
- Key: open-pull-requests-limit
- Key: groups
- Key: typescript
- Key: patterns
- Key: testing
- Key: ignore
- Key: - dependency-name
- Key: update-types
- Key: actions
- Example encodes structured test/spec or config data.

### 35.9 Pre-Commit Hook Integration

#### Installing Pre-Commit Hooks

**Source**: CASS mining of destructive_command_guard hook patterns

[Code block omitted: YAML example.]
- Block length: 10 line(s).
- Keys extracted: 7.
- Key: jobs
- Key: lint
- Key: runs-on
- Key: steps
- Key: - uses
- Key: - name
- Key: run
- Example encodes structured test/spec or config data.

[Code block omitted: YAML example.]
- Block length: 21 line(s).
- Keys extracted: 9.
- Key: repos
- Key: - repo
- Key: rev
- Key: hooks
- Key: - id
- Key: name
- Key: entry
- Key: language
- Key: types
- Example encodes structured test/spec or config data.

### 35.10 Deployment Workflows

#### Vercel Deployment

**Source**: CASS mining of jeffreysprompts_premium deploy workflow

[Code block omitted: YAML example.]
- Block length: 36 line(s).
- Keys extracted: 21.
- Key: name
- Key: on
- Key: push
- Key: tags
- Key: jobs
- Key: deploy
- Key: runs-on
- Key: steps
- Key: - uses
- Key: - name
- Key: uses
- Key: with
- Key: node-version
- Key: run
- Key: vercel-token
- Key: vercel-org-id
- Key: vercel-project-id
- Key: vercel-args
- Key: env
- Key: DATABASE_URL
- Key: curl -f https
- Example encodes structured test/spec or config data.

### 35.11 Quality Gates

#### Comprehensive Quality Pipeline

[Code block omitted: YAML example.]
- Block length: 36 line(s).
- Keys extracted: 9.
- Key: jobs
- Key: quality
- Key: runs-on
- Key: steps
- Key: - uses
- Key: - name
- Key: uses
- Key: run
- Key: echo "
- Example encodes structured test/spec or config data.

### 35.12 Self-Update Mechanisms

#### CLI Self-Update Pattern

**Source**: CASS mining of apr self-update implementation

[Code block omitted: shell command examples (52 lines).]
- Unique tools referenced: 15.
- Tool invoked: RELEASE_URL="https://github.com/owner/repo/releases/latest/download"
- Tool invoked: update_self()
- Tool invoked: local
- Tool invoked: temp_dir=$(mktemp
- Tool invoked: echo
- Tool invoked: if
- Tool invoked: rm
- Tool invoked: return
- Tool invoked: fi
- Tool invoked: expected_hash=$(cat
- Tool invoked: actual_hash=$(sha256sum
- Tool invoked: install_path=$(which
- Tool invoked: chmod
- Tool invoked: new_version=$("$install_path"
- Tool invoked: }
- Commands illustrate typical workflows and integrations.

### 35.13 Application to meta_skill

| CI/CD Component | Pattern | meta_skill Application |
|-----------------|---------|------------------------|
| **CI Pipeline** | Multi-job with dependencies | lint → test → build |
| **Matrix Testing** | OS × Runtime | ubuntu + macos, Node 20/22 |
| **Release** | Tag-triggered | `v*` creates GitHub Release with checksums |
| **Versioning** | Dual storage | VERSION file + CLI --version |
| **Quality Gates** | Lint, type, format, test, build | All must pass before merge |
| **Caching** | Dependency cache | Bun cache by lockfile hash |
| **Artifacts** | Test results, build output | 14-day retention |

### 35.14 CI/CD Checklist

Before shipping CI/CD:
- [ ] CI runs on push and pull_request to main
- [ ] Jobs have appropriate dependencies (`needs:`)
- [ ] Matrix strategy covers target platforms
- [ ] Tests run in TAP/JUnit format for reporting
- [ ] Artifacts uploaded with sensible retention
- [ ] Dependencies cached by lockfile hash
- [ ] Release workflow validates version consistency
- [ ] Checksums generated and published with releases
- [ ] Quality gates include lint, type check, format, test, build
- [ ] Deployment includes smoke tests/health checks
- [ ] Dependabot configured for dependencies and actions
- [ ] Pre-commit hooks run in CI
- [ ] Bundle size monitoring if applicable
- [ ] Container images scanned for vulnerabilities

---

## Section 36: Caching and Memoization Patterns

**Source**: CASS mining of beads_viewer, xf, coding_agent_session_search, and related optimization sessions

### 36.1 Caching Philosophy

#### When to Cache

| Scenario | Cache Strategy | Example |
|----------|----------------|---------|
| **Computed on demand, used multiple times** | Lazy accessor with memoization | `TriageContext.ActionableIssues()` |
| **Expensive computation, stable inputs** | Hash-keyed persistent cache | Graph metrics, embeddings |
| **Hot path, sub-millisecond latency required** | In-memory with TTL | API responses, search results |
| **Large dataset, memory-limited** | LRU eviction | File caches, database query results |
| **One-time initialization, immutable after** | `OnceLock` / `sync.Once` | Configuration, static indices |

#### Caching Anti-Patterns

[Code block omitted: example block (lang='n/a').]
- Block length: 5 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 36.2 Lazy Initialization Patterns

#### Rust: OnceLock for Static Lazy Values

**Source**: CASS mining of xf VectorIndex cache

[Code block omitted: Rust example code (types/logic).]
- Block length: 17 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 2 field(s).
- Function/method: get_vector_index
- Field/key: VectorIndex
- Field/key: std
- Block defines core structures or algorithms referenced by surrounding text.

**When to use**:
- Configuration that's expensive to compute
- Indices loaded on first access
- Runtime feature flags

#### Go: sync.Once for Thread-Safe Initialization

[Code block omitted: example block (lang='go').]
- Block length: 24 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### TypeScript: Lazy Accessor Pattern

[Code block omitted: example block (lang='typescript').]
- Block length: 23 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 36.3 TriageContext Pattern: Unified Lazy Caching

**Source**: CASS mining of beads_viewer TriageContext implementation

This pattern provides a context object that lazily computes and caches multiple related values, avoiding redundant computation in complex workflows.

#### Go Implementation

[Code block omitted: example block (lang='go').]
- Block length: 113 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### Key Design Points

| Aspect | Implementation | Rationale |
|--------|----------------|-----------|
| **Lazy computation** | Check `computed` flag before work | Avoid redundant expensive calls |
| **Lookup optimization** | Build `Set` from `Slice` on first access | O(1) membership tests |
| **Thread safety** | Optional mutex via constructor | Single-threaded hot paths stay fast |
| **Internal methods** | `*Internal` methods don't acquire locks | Avoid deadlock from nested calls |
| **Cycle detection** | `visiting` map parameter | Handle graph cycles gracefully |
| **Reset capability** | Clear all cached state | Reuse context across operations |

### 36.4 Heap-Based Top-K Collectors

**Source**: CASS mining of beads_viewer topk utility and cass vector search

For selecting the top K items from a stream or large collection, a min-heap is more efficient than full sorting: O(n log k) vs O(n log n).

#### Go Generic Implementation

[Code block omitted: example block (lang='go').]
- Block length: 100 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### Rust BinaryHeap Implementation

[Code block omitted: Rust example code (types/logic).]
- Block length: 68 line(s).
- Counts: 2 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 5 fn(s), 4 field(s).
- Struct: TopKCollector
- Struct: ScoredEntry
- Function/method: partial_cmp
- Function/method: cmp
- Function/method: new
- Function/method: add
- Function/method: results
- Field/key: k
- Field/key: heap
- Field/key: score
- Field/key: item
- Block defines core structures or algorithms referenced by surrounding text.

#### Complexity Comparison

| Operation | sort.Slice | Heap-based Top-K |
|-----------|------------|------------------|
| Time | O(n log n) | O(n log k) |
| Space | O(n) | O(k) |
| Streaming | No (need all items) | Yes (process items as they arrive) |

**Benchmark results** (from beads_viewer): ~15x faster for k=10 on n=10,000 items

### 36.5 LRU Cache with Disk Persistence

**Source**: CASS mining of beads_viewer cache.go and codex LRU discussions

#### Go Implementation

[Code block omitted: example block (lang='go').]
- Block length: 143 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 36.6 In-Memory Cache with TTL

**Source**: CASS mining of beads_viewer GlobalCache pattern

[Code block omitted: example block (lang='go').]
- Block length: 91 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 36.7 SIMD-Optimized Dot Product

**Source**: CASS mining of xf and cass vector search implementations

[Code block omitted: Rust example code (types/logic).]
- Block length: 43 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 3 fn(s), 1 field(s).
- Function/method: dot_product_simd
- Function/method: dot_product
- Function/method: dot_product_scalar
- Field/key: std
- Block defines core structures or algorithms referenced by surrounding text.

### 36.8 Parallel K-NN Search with Thread-Local Heaps

**Source**: CASS mining of cass vector index parallel search

[Code block omitted: Rust example code (types/logic).]
- Block length: 75 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 3 fn(s), 2 field(s).
- Impl block: VectorIndex
- Function/method: search_top_k
- Function/method: search_top_k_sequential
- Function/method: search_top_k_parallel
- Field/key: BinaryHeap
- Field/key: doc_id
- Block defines core structures or algorithms referenced by surrounding text.

### 36.9 Cache-Efficient Data Layout (Struct of Arrays)

**Source**: CASS mining of cass vector index memory layout

[Code block omitted: Rust example code (types/logic).]
- Block length: 34 line(s).
- Counts: 2 struct(s), 1 enum(s), 0 trait(s), 0 impl(s), 2 fn(s), 11 field(s).
- Struct: VectorIndex
- Struct: VectorRow
- Enum: VectorStorage
- Function/method: vector_slab_offset_bytes
- Function/method: align_up
- Field/key: rows
- Field/key: vectors
- Field/key: message_id
- Field/key: created_at_ms
- Field/key: agent_id
- Field/key: workspace_id
- Field/key: source_id
- Field/key: role
- Field/key: chunk_idx
- Field/key: vec_offset
- Field/key: content_hash
- Block defines core structures or algorithms referenced by surrounding text.

**Benefits of SoA Layout**:
| Aspect | Array of Structs (AoS) | Struct of Arrays (SoA) |
|--------|------------------------|------------------------|
| **Cache utilization** | Poor (loads unused fields) | Excellent (loads only needed data) |
| **SIMD friendliness** | Poor (scattered data) | Excellent (contiguous data) |
| **Memory bandwidth** | Wasteful | Efficient |
| **Prefetching** | Unpredictable | Sequential access patterns |

### 36.10 Hash-Based Content Deduplication

**Source**: CASS mining of xf and cass embedding deduplication

[Code block omitted: Rust example code (types/logic).]
- Block length: 31 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 1 impl(s), 2 fn(s), 3 field(s).
- Impl block: EmbeddingCache
- Function/method: content_hash
- Function/method: get_or_compute
- Field/key: content
- Field/key: compute
- Field/key: F
- Block defines core structures or algorithms referenced by surrounding text.

### 36.11 Cache Invalidation Strategies

| Strategy | Use Case | Implementation |
|----------|----------|----------------|
| **TTL-based** | Time-sensitive data | Check `time.Since(computedAt) > ttl` |
| **Hash-based** | Content-derived values | Compare `storedHash != currentHash` |
| **Event-driven** | Reactive systems | Publish invalidation on data change |
| **LRU eviction** | Memory-bounded caches | Remove least-recently-used entries |
| **Version-based** | Schema migrations | Store version, invalidate on mismatch |
| **Manual** | User-triggered | Explicit `cache.Invalidate(key)` |

### 36.12 Application to meta_skill

| Component | Caching Strategy | Pattern |
|-----------|------------------|---------|
| **Skill index** | Lazy initialization | `OnceLock` / `sync.Once` for singleton |
| **Rendered templates** | Content hash deduplication | Hash template + variables |
| **Search results** | LRU with TTL | Top-10 recent queries |
| **Parsed YAML** | TriageContext pattern | Lazy accessor on skill struct |
| **Vector embeddings** | Disk cache with hash | Avoid re-computing unchanged skills |
| **API responses** | In-memory TTL | 5-minute cache for list endpoints |

### 36.13 Caching Checklist

Before implementing caching:
- [ ] Identify the computation bottleneck (profile first)
- [ ] Determine cache key granularity (too broad = cache misses, too narrow = explosion)
- [ ] Choose appropriate TTL for data freshness requirements
- [ ] Implement invalidation strategy (stale data is worse than no caching)
- [ ] Set memory bounds (LRU eviction or max entries)
- [ ] Add hash-based staleness detection for derived values
- [ ] Consider thread safety requirements (single-threaded vs concurrent)
- [ ] Use lazy initialization for expensive one-time setup
- [ ] For hot paths, consider SIMD and cache-line alignment
- [ ] For large datasets, consider parallel processing with thread-local heaps
- [ ] Test cache behavior under memory pressure
- [ ] Add metrics for cache hit rate monitoring

---

## Section 37: Debugging Workflows and Methodologies

**Source**: CASS mining of brenner_bot, coding_agent_session_search, coding_agent_account_manager, mcp_agent_mail, fix_my_documents_backend, and agentic_coding_flywheel_setup debugging sessions

### 37.1 Debugging Philosophy

#### The Systematic Approach

Effective debugging follows a methodical process rather than random experimentation:

| Phase | Action | Outcome |
|-------|--------|---------|
| **1. Reproduce** | Create minimal, reliable reproduction | Consistent failure on demand |
| **2. Isolate** | Narrow scope to smallest unit | Single function or data path |
| **3. Hypothesize** | Form testable theory | "If X, then Y should happen" |
| **4. Verify** | Test hypothesis with evidence | Log output, debugger, or test |
| **5. Fix** | Apply minimal change | Targeted correction |
| **6. Validate** | Confirm fix works | Tests pass, behavior correct |
| **7. Prevent** | Add regression test | Future protection |

#### Debugging Anti-Patterns

[Code block omitted: example block (lang='n/a').]
- Block length: 6 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 37.2 Systematic Code Review for Bug Classes

**Source**: CASS mining of coding_agent_account_manager sync package review

#### Race Condition Hunting

[Code block omitted: example block (lang='go').]
- Block length: 14 line(s).
- Block contains illustrative content referenced by the surrounding text.

**Race Condition Detection Checklist:**
- [ ] Map access from multiple goroutines → needs mutex
- [ ] Pointer/slice assignment without sync → data race
- [ ] Check-then-act without lock → TOCTOU vulnerability
- [ ] Shared mutable state in struct → needs sync primitives

#### Go Race Detector Usage

[Code block omitted: shell command examples (6 lines).]
- Unique tools referenced: 1.
- Tool invoked: go
- Commands illustrate typical workflows and integrations.

**Example race condition fix:**

[Code block omitted: example block (lang='go').]
- Block length: 15 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 37.3 Error Handling Issue Detection

**Source**: CASS mining of coding_agent_account_manager ssh.go review

#### Error Handling Bug Patterns

| Pattern | Issue | Fix |
|---------|-------|-----|
| **Swallowed error** | `if err != nil { /* ignore */ }` | Log or propagate |
| **Missing defer Close** | Resource opened but not closed on error | Add `defer f.Close()` after open |
| **Half-handled error** | Error checked but not all paths covered | Complete error path coverage |
| **Silent fallback** | Error replaced with default without logging | Log original error before fallback |

[Code block omitted: example block (lang='go').]
- Block length: 13 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### Resource Leak Detection

[Code block omitted: example block (lang='go').]
- Block length: 20 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 37.4 Performance Debugging Methodology

**Source**: CASS mining of beads_viewer pkg/ui performance analysis

#### Profiling Hot Paths

**Step 1: Identify the hot path**
[Code block omitted: shell command examples (9 lines).]
- Unique tools referenced: 1.
- Tool invoked: go
- Commands illustrate typical workflows and integrations.

**Step 2: Measure allocation pressure**

| Allocation Source | Count/Frame | Impact |
|-------------------|-------------|--------|
| `Renderer.NewStyle()` | 16 per item | High - 800 allocs at 50 items |
| `fmt.Sprintf()` | 6 per item | Medium - string allocations |
| `append()` to slice | 8-12 per item | Low with pre-allocation |

**Step 3: Apply targeted fixes**

[Code block omitted: example block (lang='go').]
- Block length: 23 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 37.5 N+1 Query Pattern Detection

**Source**: CASS mining of mcp_agent_mail app.py N+1 analysis

#### Identifying N+1 Patterns

[Code block omitted: example block (lang='python').]
- Block length: 14 line(s).
- Block contains illustrative content referenced by the surrounding text.

[Code block omitted: example block (lang='python').]
- Block length: 10 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### N+1 Detection Checklist

- [ ] Loop containing database query → batch outside loop
- [ ] Repeated function calls with single ID → batch with list
- [ ] ORM lazy loading in loop → eager load with joins
- [ ] HTTP request per item → batch API call

### 37.6 Test Failure Debugging

**Source**: CASS mining of coding_agent_session_search cli.rs test debugging

#### Analyzing Test Failures

[Code block omitted: Rust example code (types/logic).]
- Block length: 16 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: truncate_for_markdown
- Block defines core structures or algorithms referenced by surrounding text.

#### Test Debugging Workflow

[Code block omitted: shell command examples (15 lines).]
- Unique tools referenced: 9.
- Tool invoked: cargo
- Tool invoked: fn
- Tool invoked: let
- Tool invoked: eprintln!("Input:
- Tool invoked: eprintln!("Result:
- Tool invoked: eprintln!("Result
- Tool invoked: assert_eq!(result,
- Tool invoked: }
- Tool invoked: RUST_BACKTRACE=1
- Commands illustrate typical workflows and integrations.

### 37.7 Comprehensive Investigation Report Format

**Source**: CASS mining of mcp_agent_mail manifest validation investigation

When debugging complex issues, use a structured report format:

[Code block omitted: example block (lang='markdown').]
- Block length: 9 line(s).
- Block contains illustrative content referenced by the surrounding text.
// Current problematic code
[Code block omitted: example block (lang='n/a').]
- Block length: 1 line(s).
- Block contains illustrative content referenced by the surrounding text.
// Corrected code
[Code block omitted: example block (lang='n/a').]
- Block length: 11 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 37.8 Print Debugging Best Practices

**Source**: CASS mining of coding_agent_session_search CLI normalization debugging

#### Strategic Debug Output

[Code block omitted: Rust example code (types/logic).]
- Block length: 22 line(s).
- Counts: 0 struct(s), 0 enum(s), 0 trait(s), 0 impl(s), 1 fn(s), 0 field(s).
- Function/method: normalize_args
- Block defines core structures or algorithms referenced by surrounding text.

#### Structured Logging for Debugging

[Code block omitted: example block (lang='go').]
- Block length: 22 line(s).
- Block contains illustrative content referenced by the surrounding text.

[Code block omitted: example block (lang='python').]
- Block length: 15 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 37.9 Concurrency Debugging

**Source**: CASS mining of mcp_agent_mail rate limit debugging

#### Detecting Race Conditions in Async Code

[Code block omitted: example block (lang='python').]
- Block length: 27 line(s).
- Block contains illustrative content referenced by the surrounding text.

#### Deadlock Prevention

[Code block omitted: example block (lang='go').]
- Block length: 25 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 37.10 Timeout and Context Deadline Debugging

**Source**: CASS mining of coding_agent_account_manager script test handling

[Code block omitted: example block (lang='go').]
- Block length: 26 line(s).
- Block contains illustrative content referenced by the surrounding text.

### 37.11 Debugging Checklist by Bug Type

#### Crash/Panic Debugging
- [ ] Check for nil pointer dereference
- [ ] Check for out-of-bounds array/slice access
- [ ] Check for division by zero
- [ ] Check for stack overflow (deep recursion)
- [ ] Check for race conditions (use `-race` flag)
- [ ] Enable stack traces (`RUST_BACKTRACE=1` or equivalent)

#### Memory Leak Debugging
- [ ] Check for unclosed file handles
- [ ] Check for unclosed database connections
- [ ] Check for unclosed HTTP response bodies
- [ ] Check for growing maps without cleanup
- [ ] Check for goroutines that never exit
- [ ] Profile with memory profiler

#### Performance Debugging
- [ ] Profile CPU usage to find hot spots
- [ ] Profile memory allocation
- [ ] Check for N+1 query patterns
- [ ] Check for O(n²) algorithms
- [ ] Check for unnecessary allocations in loops
- [ ] Check for synchronous I/O blocking async code

#### Logic Bug Debugging
- [ ] Add assertions for preconditions
- [ ] Log input/output at function boundaries
- [ ] Check boundary conditions (empty, one, many)
- [ ] Check error handling paths
- [ ] Use debugger to step through logic
- [ ] Write failing test first

### 37.12 Application to meta_skill

| Debugging Area | Pattern | meta_skill Application |
|----------------|---------|------------------------|
| **Race conditions** | Run with `-race` flag | Test skill loading concurrency |
| **N+1 queries** | Batch lookups | Load related skills together |
| **Performance** | Profile hot paths | Optimize skill rendering |
| **Test failures** | Structured investigation | Categorize by severity |
| **Error handling** | Check all paths | Ensure cleanup on failure |
| **Logging** | Structured with context | Add skill_id to all logs |

### 37.13 Debugging Workflow Checklist

Before diving into debugging:
- [ ] Can you reproduce the bug reliably?
- [ ] Do you have a minimal reproduction case?
- [ ] Have you checked error logs?
- [ ] Have you checked recent code changes (git bisect)?

During investigation:
- [ ] Have you isolated the failing component?
- [ ] Have you formed a testable hypothesis?
- [ ] Have you verified hypothesis with evidence?
- [ ] Have you checked related code for same issue?

After fixing:
- [ ] Does the fix address root cause (not just symptom)?
- [ ] Have you added a regression test?
- [ ] Have you removed debug code?
- [ ] Have you documented the fix in commit message?
