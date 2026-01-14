//! Beads (bd) integration module.
//!
//! This module provides programmatic access to the beads issue tracker,
//! following the same patterns as other flywheel tools (CASS, UBS, DCG).
//!
//! # Usage
//!
//! ```rust,ignore
//! use meta_skill::beads::{BeadsClient, CreateIssueRequest, IssueType};
//!
//! let client = BeadsClient::new();
//!
//! // Check availability
//! if client.is_available() {
//!     // List ready issues
//!     let ready = client.ready()?;
//!
//!     // Create a new issue
//!     let issue = client.create(
//!         CreateIssueRequest::new("Fix authentication bug")
//!             .with_type(IssueType::Bug)
//!             .with_priority(1)
//!     )?;
//!
//!     // Update status
//!     client.update_status(&issue.id, IssueStatus::InProgress)?;
//! }
//! ```

mod client;
mod types;

pub use client::BeadsClient;
pub use types::{
    CreateIssueRequest, Dependency, DependencyType, Issue, IssueStatus, IssueType, Priority,
    UpdateIssueRequest, WorkFilter,
};
