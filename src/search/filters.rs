//! Search filters for post-fusion result filtering
//!
//! Filters are applied after RRF fusion to narrow down results based on:
//! - Layer (base, org, project, user)
//! - Tags (any-match)
//! - Minimum quality score
//! - Deprecation status (default excludes deprecated)

use crate::storage::sqlite::SkillRecord;

/// Search filters for narrowing results
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    /// Filter by layer (base, org, project, user)
    pub layer: Option<String>,
    /// Filter by tags (any-match - skill must have at least one matching tag)
    pub tags: Vec<String>,
    /// Minimum quality score (0.0 - 1.0)
    pub min_quality: Option<f32>,
    /// Include deprecated skills (default: false)
    pub include_deprecated: bool,
}

impl SearchFilters {
    /// Create new empty filters
    pub fn new() -> Self {
        Self::default()
    }

    /// Set layer filter
    pub fn with_layer(mut self, layer: impl Into<String>) -> Self {
        self.layer = Some(layer.into());
        self
    }

    /// Set tags filter
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set minimum quality filter
    pub fn with_min_quality(mut self, min_quality: f32) -> Self {
        self.min_quality = Some(min_quality.clamp(0.0, 1.0));
        self
    }

    /// Include deprecated skills
    pub fn include_deprecated(mut self) -> Self {
        self.include_deprecated = true;
        self
    }

    /// Check if any filters are set
    pub fn is_empty(&self) -> bool {
        self.layer.is_none()
            && self.tags.is_empty()
            && self.min_quality.is_none()
            && !self.include_deprecated
    }

    /// Check if a skill record passes all filters
    pub fn matches(&self, skill: &SkillRecord) -> bool {
        // Layer filter
        if let Some(ref layer) = self.layer {
            if skill.source_layer != *layer {
                return false;
            }
        }

        // Tags filter (any-match)
        if !self.tags.is_empty() {
            let skill_tags = parse_tags_from_metadata(&skill.metadata_json);
            if !self.tags.iter().any(|t| skill_tags.contains(t)) {
                return false;
            }
        }

        // Quality filter
        if let Some(min_quality) = self.min_quality {
            if skill.quality_score < min_quality as f64 {
                return false;
            }
        }

        // Deprecation filter (default: exclude deprecated)
        if !self.include_deprecated && skill.is_deprecated {
            return false;
        }

        true
    }
}

/// Filter a list of skill IDs based on a lookup function
pub fn filter_skill_ids<F>(
    skill_ids: &[String],
    filters: &SearchFilters,
    lookup: F,
) -> Vec<String>
where
    F: Fn(&str) -> Option<SkillRecord>,
{
    skill_ids
        .iter()
        .filter(|id| {
            if let Some(skill) = lookup(id) {
                filters.matches(&skill)
            } else {
                false
            }
        })
        .cloned()
        .collect()
}

/// Filter hybrid results maintaining order and scores
pub fn filter_hybrid_results(
    results: Vec<super::hybrid::HybridResult>,
    filters: &SearchFilters,
    lookup: impl Fn(&str) -> Option<SkillRecord>,
) -> Vec<super::hybrid::HybridResult> {
    results
        .into_iter()
        .filter(|r| {
            if let Some(skill) = lookup(&r.skill_id) {
                filters.matches(&skill)
            } else {
                false
            }
        })
        .collect()
}

/// Parse tags from metadata JSON
fn parse_tags_from_metadata(metadata_json: &str) -> Vec<String> {
    if let Ok(meta) = serde_json::from_str::<serde_json::Value>(metadata_json) {
        if let Some(tags) = meta.get("tags").and_then(|t| t.as_array()) {
            return tags
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
        }
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_skill(id: &str, layer: &str, quality: f64, deprecated: bool, tags: &[&str]) -> SkillRecord {
        let tags_json = serde_json::json!({ "tags": tags });
        SkillRecord {
            id: id.to_string(),
            name: id.to_string(),
            description: "Test skill".to_string(),
            version: Some("1.0.0".to_string()),
            author: None,
            source_path: "/test".to_string(),
            source_layer: layer.to_string(),
            git_remote: None,
            git_commit: None,
            content_hash: "hash".to_string(),
            body: "content".to_string(),
            metadata_json: tags_json.to_string(),
            assets_json: "{}".to_string(),
            token_count: 100,
            quality_score: quality,
            indexed_at: "2025-01-01T00:00:00Z".to_string(),
            modified_at: "2025-01-01T00:00:00Z".to_string(),
            is_deprecated: deprecated,
            deprecation_reason: None,
        }
    }

    #[test]
    fn test_empty_filters_match_all() {
        let filters = SearchFilters::new();
        let skill = make_skill("test", "project", 0.8, false, &["rust"]);
        assert!(filters.matches(&skill));
    }

    #[test]
    fn test_layer_filter() {
        let filters = SearchFilters::new().with_layer("project");

        let project_skill = make_skill("s1", "project", 0.8, false, &[]);
        let org_skill = make_skill("s2", "org", 0.8, false, &[]);

        assert!(filters.matches(&project_skill));
        assert!(!filters.matches(&org_skill));
    }

    #[test]
    fn test_tags_filter_any_match() {
        let filters = SearchFilters::new().with_tags(vec!["rust".to_string(), "cli".to_string()]);

        let rust_skill = make_skill("s1", "project", 0.8, false, &["rust", "web"]);
        let cli_skill = make_skill("s2", "project", 0.8, false, &["cli"]);
        let python_skill = make_skill("s3", "project", 0.8, false, &["python"]);

        assert!(filters.matches(&rust_skill)); // has "rust"
        assert!(filters.matches(&cli_skill));  // has "cli"
        assert!(!filters.matches(&python_skill)); // no match
    }

    #[test]
    fn test_quality_filter() {
        let filters = SearchFilters::new().with_min_quality(0.7);

        let high_quality = make_skill("s1", "project", 0.9, false, &[]);
        let low_quality = make_skill("s2", "project", 0.5, false, &[]);
        let edge_quality = make_skill("s3", "project", 0.7, false, &[]);

        assert!(filters.matches(&high_quality));
        assert!(!filters.matches(&low_quality));
        assert!(filters.matches(&edge_quality)); // exactly at threshold
    }

    #[test]
    fn test_deprecated_filter_default_excludes() {
        let filters = SearchFilters::new();

        let active_skill = make_skill("s1", "project", 0.8, false, &[]);
        let deprecated_skill = make_skill("s2", "project", 0.8, true, &[]);

        assert!(filters.matches(&active_skill));
        assert!(!filters.matches(&deprecated_skill));
    }

    #[test]
    fn test_deprecated_filter_include() {
        let filters = SearchFilters::new().include_deprecated();

        let deprecated_skill = make_skill("s1", "project", 0.8, true, &[]);
        assert!(filters.matches(&deprecated_skill));
    }

    #[test]
    fn test_combined_filters() {
        let filters = SearchFilters::new()
            .with_layer("project")
            .with_tags(vec!["rust".to_string()])
            .with_min_quality(0.7);

        // Matches all filters
        let good_skill = make_skill("s1", "project", 0.8, false, &["rust"]);
        assert!(filters.matches(&good_skill));

        // Wrong layer
        let wrong_layer = make_skill("s2", "org", 0.8, false, &["rust"]);
        assert!(!filters.matches(&wrong_layer));

        // Wrong tags
        let wrong_tags = make_skill("s3", "project", 0.8, false, &["python"]);
        assert!(!filters.matches(&wrong_tags));

        // Low quality
        let low_quality = make_skill("s4", "project", 0.5, false, &["rust"]);
        assert!(!filters.matches(&low_quality));
    }

    #[test]
    fn test_quality_clamp() {
        // Quality should be clamped to 0.0-1.0
        let filters = SearchFilters::new().with_min_quality(1.5);
        assert_eq!(filters.min_quality, Some(1.0));

        let filters = SearchFilters::new().with_min_quality(-0.5);
        assert_eq!(filters.min_quality, Some(0.0));
    }

    #[test]
    fn test_is_empty() {
        assert!(SearchFilters::new().is_empty());
        assert!(!SearchFilters::new().with_layer("project").is_empty());
        assert!(!SearchFilters::new().with_tags(vec!["rust".to_string()]).is_empty());
        assert!(!SearchFilters::new().with_min_quality(0.5).is_empty());
        // Note: include_deprecated = true also makes it "not empty"
        // because it changes behavior from default
        assert!(!SearchFilters::new().include_deprecated().is_empty());
    }

    #[test]
    fn test_parse_tags_from_metadata() {
        let json = r#"{"tags": ["rust", "cli", "search"]}"#;
        let tags = parse_tags_from_metadata(json);
        assert_eq!(tags, vec!["rust", "cli", "search"]);
    }

    #[test]
    fn test_parse_tags_empty_metadata() {
        let tags = parse_tags_from_metadata("{}");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_parse_tags_invalid_json() {
        let tags = parse_tags_from_metadata("not json");
        assert!(tags.is_empty());
    }

    #[test]
    fn test_filter_skill_ids() {
        let skills = vec![
            make_skill("rust-cli", "project", 0.8, false, &["rust"]),
            make_skill("python-web", "org", 0.9, false, &["python"]),
            make_skill("deprecated", "project", 0.7, true, &["rust"]),
        ];

        let lookup = |id: &str| skills.iter().find(|s| s.id == id).cloned();

        let filters = SearchFilters::new().with_layer("project");
        let ids = vec!["rust-cli".to_string(), "python-web".to_string(), "deprecated".to_string()];

        // Should filter to project layer, excluding deprecated
        let filtered = filter_skill_ids(&ids, &filters, lookup);
        assert_eq!(filtered, vec!["rust-cli"]);
    }
}
