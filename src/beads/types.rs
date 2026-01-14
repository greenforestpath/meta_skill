//! Core types for beads (bd) integration.
//!
//! These types mirror the Go types from beads' `internal/types/types.go`.
//! We implement only the essential subset needed for common CRUD operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Issue status in the beads workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueStatus {
    Open,
    InProgress,
    Blocked,
    Deferred,
    Closed,
    Tombstone,
    Pinned,
    Hooked,
}

impl IssueStatus {
    /// Check if the status represents an active (not terminal) state.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Open | Self::InProgress | Self::Blocked | Self::Pinned | Self::Hooked)
    }

    /// Check if the status represents a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed | Self::Tombstone)
    }
}

impl std::fmt::Display for IssueStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Deferred => "deferred",
            Self::Closed => "closed",
            Self::Tombstone => "tombstone",
            Self::Pinned => "pinned",
            Self::Hooked => "hooked",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for IssueStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "in_progress" | "in-progress" | "inprogress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "deferred" => Ok(Self::Deferred),
            "closed" => Ok(Self::Closed),
            "tombstone" => Ok(Self::Tombstone),
            "pinned" => Ok(Self::Pinned),
            "hooked" => Ok(Self::Hooked),
            _ => Err(format!("unknown issue status: {}", s)),
        }
    }
}

/// Issue type classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    Task,
    Bug,
    Feature,
    Epic,
    Chore,
    Message,
    Gate,
    Agent,
    Role,
    Convoy,
    Event,
    Slot,
    Question,
    Docs,
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Task => "task",
            Self::Bug => "bug",
            Self::Feature => "feature",
            Self::Epic => "epic",
            Self::Chore => "chore",
            Self::Message => "message",
            Self::Gate => "gate",
            Self::Agent => "agent",
            Self::Role => "role",
            Self::Convoy => "convoy",
            Self::Event => "event",
            Self::Slot => "slot",
            Self::Question => "question",
            Self::Docs => "docs",
        };
        write!(f, "{}", s)
    }
}

impl std::str::FromStr for IssueType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "task" => Ok(Self::Task),
            "bug" => Ok(Self::Bug),
            "feature" => Ok(Self::Feature),
            "epic" => Ok(Self::Epic),
            "chore" => Ok(Self::Chore),
            "message" => Ok(Self::Message),
            "gate" => Ok(Self::Gate),
            "agent" => Ok(Self::Agent),
            "role" => Ok(Self::Role),
            "convoy" => Ok(Self::Convoy),
            "event" => Ok(Self::Event),
            "slot" => Ok(Self::Slot),
            "question" => Ok(Self::Question),
            "docs" => Ok(Self::Docs),
            _ => Err(format!("unknown issue type: {}", s)),
        }
    }
}

/// Priority level (0 = critical, 4 = backlog).
pub type Priority = u8;

/// A beads issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Issue {
    /// Unique issue ID (e.g., "meta_skill-abc")
    pub id: String,

    /// Issue title
    pub title: String,

    /// Issue description (Markdown)
    #[serde(default)]
    pub description: String,

    /// Current status
    pub status: IssueStatus,

    /// Priority (0-4, lower = higher priority)
    #[serde(default)]
    pub priority: Priority,

    /// Issue type classification
    pub issue_type: IssueType,

    /// Assigned owner (email or username)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// Assigned worker
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,

    /// Labels/tags
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// Notes (additional context)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Creation timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,

    /// Creator identity
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// Last update timestamp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,

    /// Closed timestamp (if closed)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,

    /// Issues that this issue depends on (blockers)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Dependency>,

    /// Issues that depend on this issue (dependents)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependents: Vec<Dependency>,
}

impl Issue {
    /// Check if this issue is ready to work (open and not blocked).
    pub fn is_ready(&self) -> bool {
        self.status == IssueStatus::Open && self.dependencies.is_empty()
    }

    /// Check if this issue is in an active (workable) state.
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }
}

/// A dependency relationship between issues.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// The related issue ID
    pub id: String,

    /// Title of the related issue
    #[serde(default)]
    pub title: String,

    /// Status of the related issue
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssueStatus>,

    /// Type of dependency relationship
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dependency_type: Option<DependencyType>,
}

/// Types of dependency relationships.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyType {
    /// This issue blocks another
    Blocks,
    /// This issue is blocked by another
    BlockedBy,
    /// Parent-child relationship
    Parent,
    Child,
    /// Conditional blocking
    ConditionalBlocks,
    /// Waiting for completion
    WaitsFor,
    /// Tracking relationship
    Tracks,
}

/// Request payload for creating a new issue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateIssueRequest {
    /// Issue title (required)
    pub title: String,

    /// Issue description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Issue type (default: task)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<IssueType>,

    /// Priority (0-4)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,

    /// Labels
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// Parent issue ID (for epics)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
}

impl CreateIssueRequest {
    /// Create a new request with the given title.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            ..Default::default()
        }
    }

    /// Set the issue type.
    pub fn with_type(mut self, issue_type: IssueType) -> Self {
        self.issue_type = Some(issue_type);
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Set the description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a label.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.labels.push(label.into());
        self
    }

    /// Set the parent issue.
    pub fn with_parent(mut self, parent: impl Into<String>) -> Self {
        self.parent = Some(parent.into());
        self
    }
}

/// Request payload for updating an issue.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateIssueRequest {
    /// New status
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssueStatus>,

    /// New title
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    /// New description
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// New priority
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub priority: Option<Priority>,

    /// New assignee
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,

    /// Labels to add
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add_labels: Vec<String>,

    /// Labels to remove
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub remove_labels: Vec<String>,

    /// Notes to append
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl UpdateIssueRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the status.
    pub fn with_status(mut self, status: IssueStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Set the assignee.
    pub fn with_assignee(mut self, assignee: impl Into<String>) -> Self {
        self.assignee = Some(assignee.into());
        self
    }

    /// Set the priority.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Add notes.
    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Filter parameters for listing issues.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkFilter {
    /// Filter by status
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<IssueStatus>,

    /// Filter by type
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue_type: Option<IssueType>,

    /// Filter by assignee
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,

    /// Filter by labels (all must match)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,

    /// Maximum priority to include (0-4)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_priority: Option<Priority>,

    /// Limit results
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_issue_status_roundtrip() {
        let statuses = [
            IssueStatus::Open,
            IssueStatus::InProgress,
            IssueStatus::Blocked,
            IssueStatus::Deferred,
            IssueStatus::Closed,
            IssueStatus::Tombstone,
            IssueStatus::Pinned,
            IssueStatus::Hooked,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: IssueStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, parsed);
        }
    }

    #[test]
    fn test_issue_type_roundtrip() {
        let types = [
            IssueType::Task,
            IssueType::Bug,
            IssueType::Feature,
            IssueType::Epic,
            IssueType::Chore,
            IssueType::Question,
            IssueType::Docs,
        ];

        for issue_type in types {
            let json = serde_json::to_string(&issue_type).unwrap();
            let parsed: IssueType = serde_json::from_str(&json).unwrap();
            assert_eq!(issue_type, parsed);
        }
    }

    #[test]
    fn test_issue_status_from_str() {
        assert_eq!("open".parse::<IssueStatus>().unwrap(), IssueStatus::Open);
        assert_eq!("in_progress".parse::<IssueStatus>().unwrap(), IssueStatus::InProgress);
        assert_eq!("in-progress".parse::<IssueStatus>().unwrap(), IssueStatus::InProgress);
        assert_eq!("closed".parse::<IssueStatus>().unwrap(), IssueStatus::Closed);
    }

    #[test]
    fn test_issue_status_is_active() {
        assert!(IssueStatus::Open.is_active());
        assert!(IssueStatus::InProgress.is_active());
        assert!(IssueStatus::Blocked.is_active());
        assert!(!IssueStatus::Closed.is_active());
        assert!(!IssueStatus::Tombstone.is_active());
    }

    #[test]
    fn test_issue_status_is_terminal() {
        assert!(IssueStatus::Closed.is_terminal());
        assert!(IssueStatus::Tombstone.is_terminal());
        assert!(!IssueStatus::Open.is_terminal());
        assert!(!IssueStatus::InProgress.is_terminal());
    }

    #[test]
    fn test_issue_deserialization() {
        let json = r#"{
            "id": "test-123",
            "title": "Test Issue",
            "description": "A test issue",
            "status": "open",
            "priority": 2,
            "issue_type": "task"
        }"#;

        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.id, "test-123");
        assert_eq!(issue.title, "Test Issue");
        assert_eq!(issue.status, IssueStatus::Open);
        assert_eq!(issue.priority, 2);
        assert_eq!(issue.issue_type, IssueType::Task);
    }

    #[test]
    fn test_create_issue_request_builder() {
        let req = CreateIssueRequest::new("Test Issue")
            .with_type(IssueType::Bug)
            .with_priority(1)
            .with_description("Bug description")
            .with_label("urgent");

        assert_eq!(req.title, "Test Issue");
        assert_eq!(req.issue_type, Some(IssueType::Bug));
        assert_eq!(req.priority, Some(1));
        assert_eq!(req.description, Some("Bug description".to_string()));
        assert!(req.labels.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_update_issue_request_builder() {
        let req = UpdateIssueRequest::new()
            .with_status(IssueStatus::InProgress)
            .with_assignee("agent@example.com")
            .with_notes("Started work");

        assert_eq!(req.status, Some(IssueStatus::InProgress));
        assert_eq!(req.assignee, Some("agent@example.com".to_string()));
        assert_eq!(req.notes, Some("Started work".to_string()));
    }
}
