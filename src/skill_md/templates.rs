//! Template strings for SKILL.md generation.
//!
//! Contains customizable templates for different sections of the SKILL.md file.
//! These templates are reserved for future SKILL.md generation features.

/// Header template with placeholders for name and description.
#[allow(dead_code)]
pub const HEADER_TEMPLATE: &str = r#"# {name}

> {description}

> Version: {version}
"#;

/// MCP server section template.
#[allow(dead_code)]
pub const MCP_SECTION_TEMPLATE: &str = r#"## MCP Server
Start MCP server for native tool integration:
```bash
ms mcp serve           # stdio transport (Claude Code)
ms mcp serve --tcp-port 8080  # HTTP transport
```
"#;

/// Context integration section content.
#[allow(dead_code)]
pub const CONTEXT_SECTION: &str = r#"## Context Integration
- Reads `.ms/config.toml` for project-specific settings
- Respects `NO_COLOR` and `FORCE_COLOR` environment variables
- Auto-detects project type from marker files
"#;

/// Basic example commands.
#[allow(dead_code)]
pub const EXAMPLE_COMMANDS: &[(&str, &str)] = &[
    ("Find skills for error handling", "ms search \"rust error handling\""),
    ("Load with full content", "ms load rust-error-patterns --level full"),
    ("Get suggestions for current project", "ms suggest --explain"),
    ("Validate a skill file", "ms lint SKILL.md"),
    ("Run health checks", "ms doctor"),
];

/// Robot mode example template.
#[allow(dead_code)]
pub const ROBOT_MODE_EXAMPLES: &[(&str, &str)] = &[
    ("search", "ms search \"query\" -O json"),
    ("load", "ms load skill-name -O json --level overview"),
    ("suggest", "ms suggest -O json"),
    ("list", "ms list -O json"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_template_has_placeholders() {
        assert!(HEADER_TEMPLATE.contains("{name}"));
        assert!(HEADER_TEMPLATE.contains("{description}"));
        assert!(HEADER_TEMPLATE.contains("{version}"));
    }

    #[test]
    fn test_example_commands_not_empty() {
        assert!(!EXAMPLE_COMMANDS.is_empty());
    }

    #[test]
    fn test_robot_mode_examples_not_empty() {
        assert!(!ROBOT_MODE_EXAMPLES.is_empty());
    }
}
