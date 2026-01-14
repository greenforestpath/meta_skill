//! Pattern mining from CASS sessions
//!
//! Extracts reusable patterns from coding session transcripts.
//! Patterns are the intermediate representation between raw sessions
//! and synthesized skills.

use std::io::Write;

use serde::{Deserialize, Serialize};
use tempfile::Builder;
use tracing::warn;

use crate::error::Result;
use crate::quality::ubs::UbsClient;
use crate::security::SafetyGate;

use super::client::Session;

// =============================================================================
// Pattern Types
// =============================================================================

/// Types of patterns that can be extracted from sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PatternType {
    /// Command sequence pattern (shell commands, CLI invocations)
    CommandPattern {
        commands: Vec<String>,
        frequency: usize,
        contexts: Vec<String>,
    },

    /// Code pattern (snippets, idioms, templates)
    CodePattern {
        language: String,
        code: String,
        purpose: String,
        frequency: usize,
    },

    /// Workflow pattern (multi-step procedures)
    WorkflowPattern {
        steps: Vec<WorkflowStep>,
        triggers: Vec<String>,
        outcomes: Vec<String>,
    },

    /// Decision pattern (conditional logic, branching)
    DecisionPattern {
        condition: String,
        branches: Vec<DecisionBranch>,
        default_action: Option<String>,
    },

    /// Error handling pattern (recovery, diagnostics)
    ErrorPattern {
        error_type: String,
        symptoms: Vec<String>,
        resolution_steps: Vec<String>,
        prevention: Option<String>,
    },

    /// Refactoring pattern (code transformations)
    RefactorPattern {
        before_pattern: String,
        after_pattern: String,
        rationale: String,
        safety_checks: Vec<String>,
    },

    /// Configuration pattern (settings, environment)
    ConfigPattern {
        config_type: String,
        settings: Vec<ConfigSetting>,
        context: String,
    },

    /// Tool usage pattern (specific tool invocations)
    ToolPattern {
        tool_name: String,
        common_args: Vec<String>,
        use_cases: Vec<String>,
    },
}

/// A step in a workflow pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub order: usize,
    pub action: String,
    pub description: String,
    pub optional: bool,
    pub conditions: Vec<String>,
}

/// A branch in a decision pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionBranch {
    pub condition: String,
    pub action: String,
    pub rationale: Option<String>,
}

/// A configuration setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSetting {
    pub key: String,
    pub value: String,
    pub description: Option<String>,
}

// =============================================================================
// Extracted Patterns
// =============================================================================

/// A pattern extracted from one or more sessions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedPattern {
    /// Unique identifier for this pattern
    pub id: String,

    /// The type and data of the pattern
    pub pattern_type: PatternType,

    /// Evidence from sessions supporting this pattern
    pub evidence: Vec<EvidenceRef>,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Number of times this pattern was observed
    pub frequency: usize,

    /// Tags for categorization
    pub tags: Vec<String>,

    /// Human-readable description
    pub description: Option<String>,
}

/// Reference to evidence in a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Session ID where pattern was found
    pub session_id: String,

    /// Message indices where pattern appears
    pub message_indices: Vec<usize>,

    /// Relevance score for this evidence
    pub relevance: f32,

    /// Snippet of the evidence (truncated)
    pub snippet: Option<String>,
}

// =============================================================================
// Pattern IR (Intermediate Representation)
// =============================================================================

/// Typed intermediate representation for patterns
///
/// This provides a normalized form for pattern analysis and transformation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "ir_type", rename_all = "snake_case")]
pub enum PatternIR {
    /// Raw text content
    Text {
        content: String,
        role: TextRole,
    },

    /// Structured command sequence
    CommandSeq {
        commands: Vec<CommandIR>,
        working_dir: Option<String>,
    },

    /// Code block with metadata
    Code {
        language: String,
        content: String,
        file_path: Option<String>,
        line_range: Option<(usize, usize)>,
    },

    /// Tool invocation record
    ToolUse {
        tool_name: String,
        arguments: serde_json::Value,
        result_summary: Option<String>,
    },

    /// Conditional structure
    Conditional {
        condition: Box<PatternIR>,
        then_branch: Box<PatternIR>,
        else_branch: Option<Box<PatternIR>>,
    },

    /// Sequence of IR nodes
    Sequence {
        items: Vec<PatternIR>,
    },

    /// Reference to another pattern
    PatternRef {
        pattern_id: String,
    },
}

/// Role of text in a pattern
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TextRole {
    Instruction,
    Explanation,
    Warning,
    Note,
    Example,
}

/// IR representation of a command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandIR {
    pub executable: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub description: Option<String>,
}

// =============================================================================
// Session Segmentation
// =============================================================================

/// Phase of a coding session
///
/// Sessions typically progress through these phases:
/// 1. Reconnaissance - understanding the problem, reading code
/// 2. Change - making modifications to solve the problem
/// 3. Validation - testing, verifying the changes work
/// 4. WrapUp - committing, cleanup, final summaries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    /// Initial exploration: reading files, searching, understanding
    Reconnaissance,
    /// Active changes: editing, writing, running commands
    Change,
    /// Verification: running tests, checking builds, reviewing
    Validation,
    /// Final steps: commits, cleanup, summaries
    WrapUp,
}

/// A segment of messages belonging to a single phase
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSegment {
    /// The phase this segment belongs to
    pub phase: SessionPhase,
    /// Starting message index (inclusive)
    pub start_idx: usize,
    /// Ending message index (exclusive)
    pub end_idx: usize,
    /// Confidence that this segmentation is correct (0.0 to 1.0)
    pub confidence: f32,
}

/// A session divided into phase segments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentedSession {
    /// The original session ID
    pub session_id: String,
    /// Ordered list of segments
    pub segments: Vec<SessionSegment>,
    /// Total message count
    pub total_messages: usize,
}

impl SegmentedSession {
    /// Get all segments of a particular phase
    pub fn segments_for_phase(&self, phase: SessionPhase) -> Vec<&SessionSegment> {
        self.segments.iter().filter(|s| s.phase == phase).collect()
    }

    /// Get the dominant phase (most messages)
    pub fn dominant_phase(&self) -> Option<SessionPhase> {
        let mut counts = std::collections::HashMap::new();
        for seg in &self.segments {
            let len = seg.end_idx - seg.start_idx;
            *counts.entry(seg.phase).or_insert(0usize) += len;
        }
        counts.into_iter().max_by_key(|(_, c)| *c).map(|(p, _)| p)
    }
}

/// Segment a session into phases based on tool usage and message patterns
pub fn segment_session(session: &Session) -> SegmentedSession {
    let mut segments = Vec::new();
    let mut current_phase = SessionPhase::Reconnaissance;
    let mut phase_start = 0;

    for (idx, msg) in session.messages.iter().enumerate() {
        let detected_phase = classify_message_phase(msg);

        // Phase transition detected
        if detected_phase != current_phase && idx > phase_start {
            // Only record segment if it has messages
            segments.push(SessionSegment {
                phase: current_phase,
                start_idx: phase_start,
                end_idx: idx,
                confidence: compute_segment_confidence(&session.messages[phase_start..idx], current_phase),
            });
            current_phase = detected_phase;
            phase_start = idx;
        }
    }

    // Record final segment
    if phase_start < session.messages.len() {
        segments.push(SessionSegment {
            phase: current_phase,
            start_idx: phase_start,
            end_idx: session.messages.len(),
            confidence: compute_segment_confidence(&session.messages[phase_start..], current_phase),
        });
    }

    // Merge adjacent segments of same phase
    segments = merge_adjacent_segments(segments);

    SegmentedSession {
        session_id: session.id.clone(),
        segments,
        total_messages: session.messages.len(),
    }
}

/// Classify a message into a session phase based on its content and tool usage
fn classify_message_phase(msg: &super::client::SessionMessage) -> SessionPhase {
    // Check tool calls to determine phase
    for tool in &msg.tool_calls {
        match tool.name.as_str() {
            // Reconnaissance tools
            "Read" | "read" | "Glob" | "glob" | "Grep" | "grep" | "ListDirectory" => {
                return SessionPhase::Reconnaissance;
            }
            // Change tools
            "Edit" | "edit" | "Write" | "write" | "NotebookEdit" => {
                return SessionPhase::Change;
            }
            // Bash commands need deeper analysis
            "Bash" | "bash" => {
                if let Some(cmd) = tool.arguments.get("command").and_then(|v| v.as_str()) {
                    return classify_bash_command(cmd);
                }
            }
            _ => {}
        }
    }

    // Check content for phase indicators
    let content_lower = msg.content.to_lowercase();

    // WrapUp indicators
    if content_lower.contains("commit")
        || content_lower.contains("done")
        || content_lower.contains("complete")
        || content_lower.contains("finished")
        || content_lower.contains("summary")
    {
        return SessionPhase::WrapUp;
    }

    // Validation indicators
    if content_lower.contains("test")
        || content_lower.contains("verify")
        || content_lower.contains("check")
        || content_lower.contains("works")
    {
        return SessionPhase::Validation;
    }

    // Default to reconnaissance for user messages, change for assistant
    if msg.role == "user" {
        SessionPhase::Reconnaissance
    } else {
        SessionPhase::Change
    }
}

/// Classify a bash command into a session phase
fn classify_bash_command(cmd: &str) -> SessionPhase {
    let cmd_lower = cmd.to_lowercase();

    // WrapUp commands
    if cmd_lower.starts_with("git commit")
        || cmd_lower.starts_with("git push")
        || cmd_lower.contains("git tag")
    {
        return SessionPhase::WrapUp;
    }

    // Validation commands
    if cmd_lower.contains("test")
        || cmd_lower.contains("cargo check")
        || cmd_lower.contains("cargo build")
        || cmd_lower.contains("npm run")
        || cmd_lower.contains("pytest")
        || cmd_lower.contains("go test")
        || cmd_lower.starts_with("git status")
        || cmd_lower.starts_with("git diff")
    {
        return SessionPhase::Validation;
    }

    // Reconnaissance commands
    if cmd_lower.starts_with("ls")
        || cmd_lower.starts_with("cat")
        || cmd_lower.starts_with("head")
        || cmd_lower.starts_with("tail")
        || cmd_lower.starts_with("find")
        || cmd_lower.starts_with("grep")
        || cmd_lower.starts_with("rg")
        || cmd_lower.starts_with("git log")
        || cmd_lower.starts_with("git show")
    {
        return SessionPhase::Reconnaissance;
    }

    // Default to Change for other bash commands
    SessionPhase::Change
}

/// Compute confidence score for a segment
fn compute_segment_confidence(
    messages: &[super::client::SessionMessage],
    expected_phase: SessionPhase,
) -> f32 {
    if messages.is_empty() {
        return 0.0;
    }

    let mut matching = 0;
    for msg in messages {
        if classify_message_phase(msg) == expected_phase {
            matching += 1;
        }
    }

    matching as f32 / messages.len() as f32
}

/// Merge adjacent segments that have the same phase
fn merge_adjacent_segments(segments: Vec<SessionSegment>) -> Vec<SessionSegment> {
    if segments.is_empty() {
        return segments;
    }

    let mut merged = Vec::new();
    let mut current = segments[0].clone();

    for seg in segments.into_iter().skip(1) {
        if seg.phase == current.phase && seg.start_idx == current.end_idx {
            // Merge: extend current segment
            current.end_idx = seg.end_idx;
            // Recompute confidence as weighted average
            let current_len = current.end_idx - current.start_idx;
            let seg_len = seg.end_idx - seg.start_idx;
            current.confidence = (current.confidence * (current_len - seg_len) as f32
                + seg.confidence * seg_len as f32)
                / current_len as f32;
        } else {
            merged.push(current);
            current = seg;
        }
    }
    merged.push(current);

    merged
}

// =============================================================================
// Legacy Pattern Type (for backward compatibility)
// =============================================================================

/// Simple pattern struct for basic extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub pattern_type: SimplePatternType,
    pub content: String,
    pub confidence: f32,
}

/// Simple pattern type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimplePatternType {
    /// Command recipe (e.g., "cargo build --release")
    CommandRecipe,
    /// Debugging decision tree
    DiagnosticTree,
    /// Invariant to maintain
    Invariant,
    /// Pitfall to avoid
    Pitfall,
    /// Prompt macro
    PromptMacro,
    /// Refactoring playbook
    RefactorPlaybook,
    /// Checklist item
    Checklist,
}

// =============================================================================
// Mining Functions
// =============================================================================

/// Extract patterns from a session transcript
pub fn extract_patterns(_session_path: &str) -> Result<Vec<Pattern>> {
    // TODO: Implement pattern extraction
    Ok(vec![])
}

/// Extract patterns from a parsed session
pub fn extract_from_session(session: &Session) -> Result<Vec<ExtractedPattern>> {
    let mut patterns = Vec::new();

    // Extract command patterns from tool calls
    let command_pattern = extract_command_patterns(session);
    if let Some(p) = command_pattern {
        patterns.push(p);
    }

    // Extract code patterns from messages
    let code_patterns = extract_code_patterns(session);
    patterns.extend(code_patterns);

    Ok(patterns)
}

/// Extract command patterns from session tool calls
fn extract_command_patterns(session: &Session) -> Option<ExtractedPattern> {
    let mut commands = Vec::new();
    let mut evidence = Vec::new();

    for msg in &session.messages {
        for tool_call in &msg.tool_calls {
            if tool_call.name == "Bash" || tool_call.name == "bash" {
                if let Some(cmd) = tool_call.arguments.get("command").and_then(|v| v.as_str()) {
                    commands.push(cmd.to_string());
                    evidence.push(EvidenceRef {
                        session_id: session.id.clone(),
                        message_indices: vec![msg.index],
                        relevance: 0.8,
                        snippet: Some(truncate(cmd, 100)),
                    });
                }
            }
        }
    }

    if commands.is_empty() {
        return None;
    }

    let frequency = evidence.len();
    Some(ExtractedPattern {
        id: format!("cmd_{}", &session.id[..8.min(session.id.len())]),
        pattern_type: PatternType::CommandPattern {
            commands,
            frequency,
            contexts: vec![session.metadata.project.clone().unwrap_or_default()],
        },
        evidence,
        confidence: 0.6,
        frequency,
        tags: vec!["auto-extracted".to_string(), "commands".to_string()],
        description: Some("Command sequence extracted from session".to_string()),
    })
}

/// Extract code patterns from session messages
fn extract_code_patterns(session: &Session) -> Vec<ExtractedPattern> {
    let mut patterns = Vec::new();
    let ubs_client = match SafetyGate::from_env() {
        Ok(gate) => UbsClient::new(None).with_safety(gate),
        Err(_) => UbsClient::new(None),
    };

    for msg in &session.messages {
        if msg.role == "assistant" {
            // Look for code blocks in content
            let code_blocks = extract_code_blocks(&msg.content);
            for (lang, code) in code_blocks {
                if code.len() > 50 {
                    if !code_passes_ubs(&ubs_client, &lang, &code) {
                        continue;
                    }
                    // Only significant code blocks
                    patterns.push(ExtractedPattern {
                        id: format!(
                            "code_{}_{}_{}",
                            &session.id[..8.min(session.id.len())],
                            msg.index,
                            patterns.len()
                        ),
                        pattern_type: PatternType::CodePattern {
                            language: lang.clone(),
                            code: code.clone(),
                            purpose: "Extracted code block".to_string(),
                            frequency: 1,
                        },
                        evidence: vec![EvidenceRef {
                            session_id: session.id.clone(),
                            message_indices: vec![msg.index],
                            relevance: 0.7,
                            snippet: Some(truncate(&code, 100)),
                        }],
                        confidence: 0.5,
                        frequency: 1,
                        tags: vec!["auto-extracted".to_string(), lang],
                        description: None,
                    });
                }
            }
        }
    }

    patterns
}

fn code_passes_ubs(client: &UbsClient, language: &str, code: &str) -> bool {
    let ext = extension_for_language(language);
    if ext == "txt" {
        return true;
    }

    let suffix = format!(".{ext}");
    let mut temp: tempfile::NamedTempFile = match Builder::new().prefix("ms-ubs-").suffix(&suffix).tempfile() {
        Ok(file) => file,
        Err(err) => {
            warn!("ubs temp file error: {err}");
            return true;
        }
    };

    if let Err(err) = temp.write_all(code.as_bytes()) {
        warn!("ubs temp write error: {err}");
        return true;
    }
    if let Err(err) = temp.flush() {
        warn!("ubs temp flush error: {err}");
        return true;
    }

    let path = temp.path().to_path_buf();
    match client.check_files(&[path]) {
        Ok(result) => result.is_clean(),
        Err(err) => {
            warn!("ubs check failed: {err}");
            true
        }
    }
}

fn extension_for_language(language: &str) -> &'static str {
    match language.trim().to_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "go" => "go",
        "python" | "py" => "py",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "bash" | "sh" | "shell" => "sh",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        _ => "txt",
    }
}

/// Extract code blocks from markdown content
fn extract_code_blocks(content: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut in_block = false;
    let mut current_lang = String::new();
    let mut current_code = String::new();

    for line in content.lines() {
        if line.starts_with("```") {
            if in_block {
                // End of block
                blocks.push((current_lang.clone(), current_code.trim().to_string()));
                current_lang.clear();
                current_code.clear();
                in_block = false;
            } else {
                // Start of block
                current_lang = line.trim_start_matches('`').trim().to_string();
                in_block = true;
            }
        } else if in_block {
            current_code.push_str(line);
            current_code.push('\n');
        }
    }

    blocks
}

/// Convert extracted pattern to IR
pub fn pattern_to_ir(pattern: &ExtractedPattern) -> PatternIR {
    match &pattern.pattern_type {
        PatternType::CommandPattern { commands, .. } => PatternIR::CommandSeq {
            commands: commands
                .iter()
                .map(|cmd| {
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    CommandIR {
                        executable: parts.first().unwrap_or(&"").to_string(),
                        args: parts.iter().skip(1).map(|s| s.to_string()).collect(),
                        env: vec![],
                        description: None,
                    }
                })
                .collect(),
            working_dir: None,
        },

        PatternType::CodePattern { language, code, .. } => PatternIR::Code {
            language: language.clone(),
            content: code.clone(),
            file_path: None,
            line_range: None,
        },

        PatternType::WorkflowPattern { steps, .. } => PatternIR::Sequence {
            items: steps
                .iter()
                .map(|step| PatternIR::Text {
                    content: format!("{}. {}", step.order, step.action),
                    role: TextRole::Instruction,
                })
                .collect(),
        },

        _ => PatternIR::Text {
            content: pattern.description.clone().unwrap_or_default(),
            role: TextRole::Explanation,
        },
    }
}

/// Truncate string to max length
fn truncate(s: &str, max_len: usize) -> String {
    let mut out = String::new();
    for (idx, ch) in s.chars().enumerate() {
        if idx >= max_len {
            break;
        }
        out.push(ch);
    }
    if s.chars().count() > max_len {
        if max_len >= 3 {
            let trimmed = out.chars().take(max_len.saturating_sub(3)).collect::<String>();
            format!("{trimmed}...")
        } else {
            "...".to_string()
        }
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_code_blocks() {
        let content = r#"Here is some code:

```rust
fn main() {
    println!("Hello");
}
```

And more text.
"#;
        let blocks = extract_code_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].0, "rust");
        assert!(blocks[0].1.contains("fn main()"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
    }

    #[test]
    fn test_pattern_serialization() {
        let pattern = ExtractedPattern {
            id: "test-1".to_string(),
            pattern_type: PatternType::CommandPattern {
                commands: vec!["cargo build".to_string()],
                frequency: 1,
                contexts: vec!["rust".to_string()],
            },
            evidence: vec![],
            confidence: 0.8,
            frequency: 1,
            tags: vec!["test".to_string()],
            description: None,
        };

        let json = serde_json::to_string(&pattern).unwrap();
        assert!(json.contains("command_pattern"));
        assert!(json.contains("cargo build"));
    }

    #[test]
    fn test_pattern_ir_serialization() {
        let ir = PatternIR::Code {
            language: "rust".to_string(),
            content: "fn foo() {}".to_string(),
            file_path: None,
            line_range: None,
        };

        let json = serde_json::to_string(&ir).unwrap();
        assert!(json.contains("code"));
        assert!(json.contains("rust"));
    }
}
