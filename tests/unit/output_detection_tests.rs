//! Comprehensive unit tests for output detection module.
//!
//! Tests cover all aspects of output mode detection including:
//! - Format-based detection (machine-readable vs human)
//! - Config/robot mode overrides
//! - Agent environment variable detection (15 vars)
//! - CI environment variable detection (12 vars)
//! - IDE environment variable detection (6 vars)
//! - Standard environment variables (NO_COLOR, MS_PLAIN_OUTPUT, MS_FORCE_RICH)
//! - Terminal detection (TTY vs piped)
//! - Precedence logic
//! - OutputModeReport generation
//!
//! All tests use the EnvGuard RAII pattern to ensure environment variable
//! isolation between tests.

use std::env;
use std::ffi::OsString;

use ms::cli::output::OutputFormat;
use ms::output::detection::{
    detected_agent_vars, detected_ci_vars, detected_ide_vars, is_agent_environment,
    is_ci_environment, is_ide_environment, maybe_print_debug_output, should_use_rich_output,
    should_use_rich_with_flags, OutputDecisionReason, OutputDetector, OutputEnvironment,
    OutputModeReport, AGENT_ENV_VARS, CI_ENV_VARS, IDE_ENV_VARS,
};

// =============================================================================
// Test Environment Guard (RAII for env vars)
// =============================================================================

/// RAII guard for temporarily setting environment variables.
/// When dropped, restores all variables to their original state.
struct EnvGuard {
    original_values: Vec<(String, Option<OsString>)>,
}

impl EnvGuard {
    fn new() -> Self {
        Self {
            original_values: Vec::new(),
        }
    }

    fn set(mut self, key: &str, value: &str) -> Self {
        let original = env::var_os(key);
        self.original_values.push((key.to_string(), original));
        // SAFETY: Test-only code
        unsafe { env::set_var(key, value) };
        self
    }

    fn unset(mut self, key: &str) -> Self {
        let original = env::var_os(key);
        self.original_values.push((key.to_string(), original));
        // SAFETY: Test-only code
        unsafe { env::remove_var(key) };
        self
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, original) in self.original_values.iter().rev() {
            // SAFETY: Restoring environment state in test cleanup
            unsafe {
                match original {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            }
        }
    }
}

// =============================================================================
// Format-Based Detection Tests
// =============================================================================

mod format_based {
    use super::*;

    #[test]
    fn test_json_format_always_plain() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Json, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::MachineReadableFormat);
    }

    #[test]
    fn test_jsonl_format_always_plain() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Jsonl, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::MachineReadableFormat);
    }

    #[test]
    fn test_tsv_format_always_plain() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Tsv, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::MachineReadableFormat);
    }

    #[test]
    fn test_plain_format_always_plain() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Plain, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::PlainFormat);
    }

    #[test]
    fn test_human_format_can_be_rich_on_tty() {
        let env = OutputEnvironment::new(false, false, false, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::HumanDefault);
    }

    #[test]
    fn test_format_takes_precedence_over_force_rich_env() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Json, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::MachineReadableFormat);
    }

    #[test]
    fn test_all_machine_readable_formats() {
        let formats = [OutputFormat::Json, OutputFormat::Jsonl, OutputFormat::Tsv];

        for format in formats {
            let env = OutputEnvironment::new(false, false, true, true);
            let detector = OutputDetector::with_env(format, false, env);
            let decision = detector.decide();

            assert!(
                !decision.use_rich,
                "{:?} should produce plain output",
                format
            );
        }
    }

    #[test]
    fn test_robot_flag_converts_to_json_format() {
        // The robot flag causes JSON format to be used
        let format = OutputFormat::from_args(true, None);
        assert_eq!(format, OutputFormat::Json);
    }
}

// =============================================================================
// Config/Robot Mode Detection Tests
// =============================================================================

mod config_based {
    use super::*;

    #[test]
    fn test_robot_mode_overrides_to_plain() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, true, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::RobotMode);
    }

    #[test]
    fn test_robot_mode_beats_force_rich() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, true, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::RobotMode);
    }

    #[test]
    fn test_format_beats_robot_mode() {
        let env = OutputEnvironment::new(false, false, false, true);
        let detector = OutputDetector::with_env(OutputFormat::Json, true, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::MachineReadableFormat);
    }
}

// =============================================================================
// Environment Variable Detection Tests
// =============================================================================

mod env_vars {
    use super::*;

    #[test]
    fn test_no_color_env_disables_rich() {
        let env = OutputEnvironment::new(true, false, false, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::EnvNoColor);
    }

    #[test]
    fn test_ms_plain_output_env_disables_rich() {
        let env = OutputEnvironment::new(false, true, false, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::EnvPlainOutput);
    }

    #[test]
    fn test_ms_force_rich_env_enables_rich() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::ForcedRich);
    }

    #[test]
    fn test_no_color_beats_force_rich() {
        let env = OutputEnvironment::new(true, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::EnvNoColor);
    }

    #[test]
    fn test_plain_output_beats_force_rich() {
        let env = OutputEnvironment::new(false, true, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::EnvPlainOutput);
    }
}

// =============================================================================
// Agent Environment Variable Detection Tests (15 env vars)
// =============================================================================

mod agent_env_detection {
    use super::*;

    #[test]
    fn test_agent_env_vars_constant_has_15_entries() {
        assert_eq!(
            AGENT_ENV_VARS.len(),
            15,
            "Expected 15 agent environment variables"
        );
    }

    #[test]
    fn test_claude_code_env_detected() {
        let _guard = EnvGuard::new().set("CLAUDE_CODE", "1");
        assert!(is_agent_environment());
        let vars = detected_agent_vars();
        assert!(vars.contains(&"CLAUDE_CODE".to_string()));
    }

    #[test]
    fn test_cursor_ai_env_detected() {
        let _guard = EnvGuard::new().set("CURSOR_AI", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_openai_codex_env_detected() {
        let _guard = EnvGuard::new().set("OPENAI_CODEX", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_aider_mode_env_detected() {
        let _guard = EnvGuard::new().set("AIDER_MODE", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_codeium_enabled_env_detected() {
        let _guard = EnvGuard::new().set("CODEIUM_ENABLED", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_windsurf_agent_env_detected() {
        let _guard = EnvGuard::new().set("WINDSURF_AGENT", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_copilot_agent_env_detected() {
        let _guard = EnvGuard::new().set("COPILOT_AGENT", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_copilot_workspace_env_detected() {
        let _guard = EnvGuard::new().set("COPILOT_WORKSPACE", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_generic_agent_mode_env_detected() {
        let _guard = EnvGuard::new().set("AGENT_MODE", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_ide_agent_env_detected() {
        let _guard = EnvGuard::new().set("IDE_AGENT", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_continue_dev_env_detected() {
        let _guard = EnvGuard::new().set("CONTINUE_DEV", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_sourcegraph_cody_env_detected() {
        let _guard = EnvGuard::new().set("SOURCEGRAPH_CODY", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_tabnine_agent_env_detected() {
        let _guard = EnvGuard::new().set("TABNINE_AGENT", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_amazon_q_env_detected() {
        let _guard = EnvGuard::new().set("AMAZON_Q", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_gemini_code_env_detected() {
        let _guard = EnvGuard::new().set("GEMINI_CODE", "1");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_no_agent_env_when_none_set() {
        let _guard = EnvGuard::new()
            .unset("CLAUDE_CODE")
            .unset("CURSOR_AI")
            .unset("OPENAI_CODEX")
            .unset("AIDER_MODE")
            .unset("CODEIUM_ENABLED")
            .unset("WINDSURF_AGENT")
            .unset("COPILOT_AGENT")
            .unset("COPILOT_WORKSPACE")
            .unset("AGENT_MODE")
            .unset("IDE_AGENT")
            .unset("CONTINUE_DEV")
            .unset("SOURCEGRAPH_CODY")
            .unset("TABNINE_AGENT")
            .unset("AMAZON_Q")
            .unset("GEMINI_CODE");

        assert!(!is_agent_environment());
        assert!(detected_agent_vars().is_empty());
    }

    #[test]
    fn test_agent_env_with_various_values() {
        // "1" should work
        let _guard = EnvGuard::new().set("CLAUDE_CODE", "1");
        assert!(is_agent_environment());
        drop(_guard);

        // "true" should work
        let _guard = EnvGuard::new().set("CLAUDE_CODE", "true");
        assert!(is_agent_environment());
        drop(_guard);

        // "yes" should work
        let _guard = EnvGuard::new().set("CLAUDE_CODE", "yes");
        assert!(is_agent_environment());
        drop(_guard);

        // Any non-empty value should work
        let _guard = EnvGuard::new().set("CLAUDE_CODE", "anything");
        assert!(is_agent_environment());
    }

    #[test]
    fn test_empty_agent_env_value_not_detected() {
        let _guard = EnvGuard::new().set("CLAUDE_CODE", "");
        // Empty value should not count
        assert!(!is_agent_environment());
    }

    #[test]
    fn test_multiple_agent_vars_all_detected() {
        let _guard = EnvGuard::new()
            .set("CLAUDE_CODE", "1")
            .set("CURSOR_AI", "1")
            .set("AMAZON_Q", "1");

        assert!(is_agent_environment());
        let vars = detected_agent_vars();
        assert!(vars.len() >= 3);
        assert!(vars.contains(&"CLAUDE_CODE".to_string()));
        assert!(vars.contains(&"CURSOR_AI".to_string()));
        assert!(vars.contains(&"AMAZON_Q".to_string()));
    }
}

// =============================================================================
// CI Environment Variable Detection Tests (12 env vars)
// =============================================================================

mod ci_env_detection {
    use super::*;

    #[test]
    fn test_ci_env_vars_constant_has_12_entries() {
        assert_eq!(CI_ENV_VARS.len(), 12, "Expected 12 CI environment variables");
    }

    #[test]
    fn test_generic_ci_env_detected() {
        let _guard = EnvGuard::new().set("CI", "true");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_github_actions_env_detected() {
        let _guard = EnvGuard::new().set("GITHUB_ACTIONS", "true");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_gitlab_ci_env_detected() {
        let _guard = EnvGuard::new().set("GITLAB_CI", "true");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_jenkins_url_env_detected() {
        let _guard = EnvGuard::new().set("JENKINS_URL", "http://jenkins.example.com");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_travis_env_detected() {
        let _guard = EnvGuard::new().set("TRAVIS", "true");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_circleci_env_detected() {
        let _guard = EnvGuard::new().set("CIRCLECI", "true");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_buildkite_env_detected() {
        let _guard = EnvGuard::new().set("BUILDKITE", "true");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_bitbucket_pipelines_env_detected() {
        let _guard = EnvGuard::new().set("BITBUCKET_PIPELINES", "true");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_azure_pipelines_tf_build_env_detected() {
        let _guard = EnvGuard::new().set("TF_BUILD", "True");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_teamcity_version_env_detected() {
        let _guard = EnvGuard::new().set("TEAMCITY_VERSION", "2023.11.1");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_drone_env_detected() {
        let _guard = EnvGuard::new().set("DRONE", "true");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_woodpecker_env_detected() {
        let _guard = EnvGuard::new().set("WOODPECKER", "true");
        assert!(is_ci_environment());
    }

    #[test]
    fn test_no_ci_env_when_none_set() {
        let _guard = EnvGuard::new()
            .unset("CI")
            .unset("GITHUB_ACTIONS")
            .unset("GITLAB_CI")
            .unset("JENKINS_URL")
            .unset("TRAVIS")
            .unset("CIRCLECI")
            .unset("BUILDKITE")
            .unset("BITBUCKET_PIPELINES")
            .unset("TF_BUILD")
            .unset("TEAMCITY_VERSION")
            .unset("DRONE")
            .unset("WOODPECKER");

        assert!(!is_ci_environment());
        assert!(detected_ci_vars().is_empty());
    }

    #[test]
    fn test_multiple_ci_vars_detected() {
        let _guard = EnvGuard::new()
            .set("CI", "true")
            .set("GITHUB_ACTIONS", "true");

        assert!(is_ci_environment());
        let vars = detected_ci_vars();
        assert!(vars.len() >= 2);
        assert!(vars.contains(&"CI".to_string()));
        assert!(vars.contains(&"GITHUB_ACTIONS".to_string()));
    }
}

// =============================================================================
// IDE Environment Variable Detection Tests (6 env vars)
// =============================================================================

mod ide_env_detection {
    use super::*;

    #[test]
    fn test_ide_env_vars_constant_has_6_entries() {
        assert_eq!(IDE_ENV_VARS.len(), 6, "Expected 6 IDE environment variables");
    }

    #[test]
    fn test_vscode_git_askpass_node_env_detected() {
        let _guard = EnvGuard::new().set("VSCODE_GIT_ASKPASS_NODE", "/path/to/node");
        assert!(is_ide_environment());
    }

    #[test]
    fn test_vscode_injection_env_detected() {
        let _guard = EnvGuard::new().set("VSCODE_INJECTION", "1");
        assert!(is_ide_environment());
    }

    #[test]
    fn test_codespaces_env_detected() {
        let _guard = EnvGuard::new().set("CODESPACES", "true");
        assert!(is_ide_environment());
    }

    #[test]
    fn test_gitpod_workspace_id_env_detected() {
        let _guard = EnvGuard::new().set("GITPOD_WORKSPACE_ID", "abc123xyz");
        assert!(is_ide_environment());
    }

    #[test]
    fn test_replit_db_url_env_detected() {
        let _guard = EnvGuard::new().set("REPLIT_DB_URL", "https://kv.replit.com/...");
        assert!(is_ide_environment());
    }

    #[test]
    fn test_cloud_shell_env_detected() {
        let _guard = EnvGuard::new().set("CLOUD_SHELL", "true");
        assert!(is_ide_environment());
    }

    #[test]
    fn test_no_ide_env_when_none_set() {
        let _guard = EnvGuard::new()
            .unset("VSCODE_GIT_ASKPASS_NODE")
            .unset("VSCODE_INJECTION")
            .unset("CODESPACES")
            .unset("GITPOD_WORKSPACE_ID")
            .unset("REPLIT_DB_URL")
            .unset("CLOUD_SHELL");

        assert!(!is_ide_environment());
        assert!(detected_ide_vars().is_empty());
    }

    #[test]
    fn test_multiple_ide_vars_detected() {
        let _guard = EnvGuard::new()
            .set("CODESPACES", "true")
            .set("VSCODE_INJECTION", "1");

        assert!(is_ide_environment());
        let vars = detected_ide_vars();
        assert!(vars.len() >= 2);
    }
}

// =============================================================================
// Terminal Detection Tests
// =============================================================================

mod terminal_detection {
    use super::*;

    #[test]
    fn test_non_tty_gets_plain() {
        let env = OutputEnvironment::new(false, false, false, false);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::NotTerminal);
    }

    #[test]
    fn test_tty_gets_rich_by_default() {
        let env = OutputEnvironment::new(false, false, false, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::HumanDefault);
    }

    #[test]
    fn test_non_tty_with_force_rich_still_plain() {
        // force_rich only works on TTY
        let env = OutputEnvironment::new(false, false, true, false);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::NotTerminal);
    }
}

// =============================================================================
// Precedence Tests (Critical - Full Chain)
// =============================================================================

mod precedence {
    use super::*;

    #[test]
    fn test_machine_readable_format_beats_everything() {
        // Even with all flags favoring rich, machine-readable format wins
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Json, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::MachineReadableFormat);
    }

    #[test]
    fn test_plain_format_beats_everything_except_machine_readable() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Plain, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::PlainFormat);
    }

    #[test]
    fn test_robot_mode_beats_env_settings() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, true, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::RobotMode);
    }

    #[test]
    fn test_no_color_beats_force_rich_env() {
        let env = OutputEnvironment::new(true, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::EnvNoColor);
    }

    #[test]
    fn test_plain_output_env_beats_force_rich_env() {
        let env = OutputEnvironment::new(false, true, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::EnvPlainOutput);
    }

    #[test]
    fn test_not_terminal_beats_force_rich() {
        let env = OutputEnvironment::new(false, false, true, false);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(!decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::NotTerminal);
    }

    #[test]
    fn test_force_rich_used_only_when_all_else_allows() {
        let env = OutputEnvironment::new(false, false, true, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::ForcedRich);
    }

    #[test]
    fn test_human_default_when_no_overrides() {
        let env = OutputEnvironment::new(false, false, false, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let decision = detector.decide();

        assert!(decision.use_rich);
        assert_eq!(decision.reason, OutputDecisionReason::HumanDefault);
    }

    #[test]
    fn test_full_precedence_chain_documented() {
        // Document the full precedence chain:
        // 1. Machine-readable format (JSON/JSONL/TSV) -> plain
        // 2. Plain format -> plain
        // 3. Robot mode flag -> plain
        // 4. NO_COLOR env -> plain
        // 5. MS_PLAIN_OUTPUT env -> plain
        // 6. Not a terminal -> plain
        // 7. MS_FORCE_RICH env -> rich
        // 8. Default human format on terminal -> rich

        let env = OutputEnvironment::new(false, false, true, true);

        // Level 1: Machine-readable wins
        let d1 = OutputDetector::with_env(OutputFormat::Json, true, env).decide();
        assert_eq!(d1.reason, OutputDecisionReason::MachineReadableFormat);

        // Level 2: Plain format wins (over robot mode)
        let d2 = OutputDetector::with_env(OutputFormat::Plain, true, env).decide();
        assert_eq!(d2.reason, OutputDecisionReason::PlainFormat);

        // Level 3: Robot mode wins (over env)
        let d3 = OutputDetector::with_env(OutputFormat::Human, true, env).decide();
        assert_eq!(d3.reason, OutputDecisionReason::RobotMode);

        // Level 4: NO_COLOR wins
        let env4 = OutputEnvironment::new(true, true, true, true);
        let d4 = OutputDetector::with_env(OutputFormat::Human, false, env4).decide();
        assert_eq!(d4.reason, OutputDecisionReason::EnvNoColor);

        // Level 5: MS_PLAIN_OUTPUT wins
        let env5 = OutputEnvironment::new(false, true, true, true);
        let d5 = OutputDetector::with_env(OutputFormat::Human, false, env5).decide();
        assert_eq!(d5.reason, OutputDecisionReason::EnvPlainOutput);

        // Level 6: Not terminal wins
        let env6 = OutputEnvironment::new(false, false, true, false);
        let d6 = OutputDetector::with_env(OutputFormat::Human, false, env6).decide();
        assert_eq!(d6.reason, OutputDecisionReason::NotTerminal);

        // Level 7: Force rich wins
        let env7 = OutputEnvironment::new(false, false, true, true);
        let d7 = OutputDetector::with_env(OutputFormat::Human, false, env7).decide();
        assert_eq!(d7.reason, OutputDecisionReason::ForcedRich);

        // Level 8: Human default
        let env8 = OutputEnvironment::new(false, false, false, true);
        let d8 = OutputDetector::with_env(OutputFormat::Human, false, env8).decide();
        assert_eq!(d8.reason, OutputDecisionReason::HumanDefault);
    }
}

// =============================================================================
// should_use_rich_with_flags Tests
// =============================================================================

mod flags_api {
    use super::*;

    #[test]
    fn test_force_plain_flag_wins() {
        let result = should_use_rich_with_flags(OutputFormat::Human, false, true, true);
        assert!(!result);
    }

    #[test]
    fn test_force_rich_flag_wins_when_not_plain() {
        let result = should_use_rich_with_flags(OutputFormat::Human, false, false, true);
        assert!(result);
    }

    #[test]
    fn test_flags_fallback_to_detection() {
        // With no flags, should use normal detection
        let result = should_use_rich_with_flags(OutputFormat::Json, false, false, false);
        assert!(!result); // JSON is machine-readable
    }
}

// =============================================================================
// OutputEnvironment Tests
// =============================================================================

mod output_environment {
    use super::*;

    #[test]
    fn test_output_environment_new() {
        let env = OutputEnvironment::new(true, false, true, false);
        assert!(env.no_color);
        assert!(!env.plain_output);
        assert!(env.force_rich);
        assert!(!env.stdout_is_terminal);
    }

    #[test]
    fn test_output_environment_from_env_captures_no_color() {
        let _guard = EnvGuard::new().set("NO_COLOR", "1");
        let env = OutputEnvironment::from_env();
        assert!(env.no_color);
    }

    #[test]
    fn test_output_environment_from_env_captures_plain_output() {
        let _guard = EnvGuard::new().set("MS_PLAIN_OUTPUT", "1");
        let env = OutputEnvironment::from_env();
        assert!(env.plain_output);
    }

    #[test]
    fn test_output_environment_from_env_captures_force_rich() {
        let _guard = EnvGuard::new().set("MS_FORCE_RICH", "1");
        let env = OutputEnvironment::from_env();
        assert!(env.force_rich);
    }
}

// =============================================================================
// OutputDecision Tests
// =============================================================================

mod output_decision {
    use super::*;

    #[test]
    fn test_output_decision_rich_and_plain() {
        let env = OutputEnvironment::new(false, false, false, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        let rich_decision = detector.decide();
        assert!(rich_decision.use_rich);

        let env_plain = OutputEnvironment::new(true, false, false, true);
        let detector_plain = OutputDetector::with_env(OutputFormat::Human, false, env_plain);
        let plain_decision = detector_plain.decide();
        assert!(!plain_decision.use_rich);
    }

    #[test]
    fn test_all_decision_reasons_reachable() {
        // Test that each reason can be triggered
        let reasons = vec![
            (
                OutputFormat::Json,
                false,
                OutputEnvironment::new(false, false, false, true),
                OutputDecisionReason::MachineReadableFormat,
            ),
            (
                OutputFormat::Plain,
                false,
                OutputEnvironment::new(false, false, false, true),
                OutputDecisionReason::PlainFormat,
            ),
            (
                OutputFormat::Human,
                true,
                OutputEnvironment::new(false, false, false, true),
                OutputDecisionReason::RobotMode,
            ),
            (
                OutputFormat::Human,
                false,
                OutputEnvironment::new(true, false, false, true),
                OutputDecisionReason::EnvNoColor,
            ),
            (
                OutputFormat::Human,
                false,
                OutputEnvironment::new(false, true, false, true),
                OutputDecisionReason::EnvPlainOutput,
            ),
            (
                OutputFormat::Human,
                false,
                OutputEnvironment::new(false, false, false, false),
                OutputDecisionReason::NotTerminal,
            ),
            (
                OutputFormat::Human,
                false,
                OutputEnvironment::new(false, false, true, true),
                OutputDecisionReason::ForcedRich,
            ),
            (
                OutputFormat::Human,
                false,
                OutputEnvironment::new(false, false, false, true),
                OutputDecisionReason::HumanDefault,
            ),
        ];

        for (format, robot, env, expected_reason) in reasons {
            let detector = OutputDetector::with_env(format, robot, env);
            let decision = detector.decide();
            assert_eq!(
                decision.reason, expected_reason,
                "Failed for format={:?}, robot={}, env={:?}",
                format, robot, env
            );
        }
    }
}

// =============================================================================
// OutputModeReport Tests
// =============================================================================

mod report {
    use super::*;

    #[test]
    fn test_report_generation() {
        let report = OutputModeReport::generate(OutputFormat::Human, false);

        assert_eq!(report.format, "Human");
        assert!(!report.robot_mode);
        // The decision should be populated
        let _ = report.decision;
    }

    #[test]
    fn test_report_with_robot_mode() {
        let report = OutputModeReport::generate(OutputFormat::Human, true);

        assert_eq!(report.format, "Human");
        assert!(report.robot_mode);
        assert!(!report.decision.use_rich);
        assert_eq!(report.decision.reason, OutputDecisionReason::RobotMode);
    }

    #[test]
    fn test_report_format_text() {
        let report = OutputModeReport::generate(OutputFormat::Human, false);
        let text = report.format_text();

        assert!(text.contains("Output Mode Detection Report"));
        assert!(text.contains("Format:"));
        assert!(text.contains("Robot Mode:"));
        assert!(text.contains("Decision:"));
    }

    #[test]
    fn test_report_captures_env_vars() {
        let _guard = EnvGuard::new().set("NO_COLOR", "1");
        let report = OutputModeReport::generate(OutputFormat::Human, false);

        assert!(report.env.no_color);
    }

    #[test]
    fn test_report_captures_agent_vars() {
        let _guard = EnvGuard::new().set("CLAUDE_CODE", "1");
        let report = OutputModeReport::generate(OutputFormat::Human, false);

        assert!(!report.agent_vars.is_empty());
        assert!(report.agent_vars.contains(&"CLAUDE_CODE".to_string()));
    }

    #[test]
    fn test_report_captures_ci_vars() {
        let _guard = EnvGuard::new().set("GITHUB_ACTIONS", "true");
        let report = OutputModeReport::generate(OutputFormat::Human, false);

        assert!(!report.ci_vars.is_empty());
        assert!(report.ci_vars.contains(&"GITHUB_ACTIONS".to_string()));
    }

    #[test]
    fn test_report_captures_ide_vars() {
        let _guard = EnvGuard::new().set("CODESPACES", "true");
        let report = OutputModeReport::generate(OutputFormat::Human, false);

        assert!(!report.ide_vars.is_empty());
        assert!(report.ide_vars.contains(&"CODESPACES".to_string()));
    }

    #[test]
    fn test_report_captures_terminal_info() {
        let _guard = EnvGuard::new()
            .set("TERM", "xterm-256color")
            .set("COLORTERM", "truecolor")
            .set("COLUMNS", "120");
        let report = OutputModeReport::generate(OutputFormat::Human, false);

        assert_eq!(report.term.as_deref(), Some("xterm-256color"));
        assert_eq!(report.colorterm.as_deref(), Some("truecolor"));
        assert_eq!(report.columns.as_deref(), Some("120"));
    }
}

// =============================================================================
// maybe_print_debug_output Tests
// =============================================================================

mod debug_output {
    use super::*;

    #[test]
    fn test_debug_output_when_env_not_set() {
        let _guard = EnvGuard::new().unset("MS_DEBUG_OUTPUT");
        // Should not panic and should not print anything
        maybe_print_debug_output(OutputFormat::Human, false);
    }

    #[test]
    fn test_debug_output_when_env_set() {
        let _guard = EnvGuard::new().set("MS_DEBUG_OUTPUT", "1");
        // Should not panic (output goes to stderr)
        maybe_print_debug_output(OutputFormat::Human, false);
    }
}

// =============================================================================
// Helper Function Tests
// =============================================================================

mod helpers {
    use super::*;

    #[test]
    fn test_should_use_rich_output_convenience_function() {
        // This uses the from_env() constructor internally, so we test
        // with controlled env vars
        let _guard = EnvGuard::new().set("NO_COLOR", "1");
        let result = should_use_rich_output(OutputFormat::Human, false);
        assert!(!result);
    }

    #[test]
    fn test_should_use_rich_output_with_machine_format() {
        let result = should_use_rich_output(OutputFormat::Json, false);
        assert!(!result);
    }

    #[test]
    fn test_detector_should_use_rich_method() {
        let env = OutputEnvironment::new(false, false, false, true);
        let detector = OutputDetector::with_env(OutputFormat::Human, false, env);
        assert!(detector.should_use_rich());
    }
}

// =============================================================================
// Edge Cases and Error Handling Tests
// =============================================================================

mod edge_cases {
    use super::*;

    #[test]
    fn test_detection_never_panics_with_unusual_env() {
        // Test with various unusual environment values
        let _guard = EnvGuard::new()
            .set("NO_COLOR", "")
            .set("MS_PLAIN_OUTPUT", "false") // Should still count as set
            .set("MS_FORCE_RICH", "0"); // Should still count as set

        // Should not panic
        let _ = OutputEnvironment::from_env();
    }

    #[test]
    fn test_all_output_formats_handled() {
        // Ensure all output formats produce a valid decision
        let formats = vec![
            OutputFormat::Human,
            OutputFormat::Plain,
            OutputFormat::Json,
            OutputFormat::Jsonl,
            OutputFormat::Tsv,
        ];

        for format in formats {
            let env = OutputEnvironment::new(false, false, false, true);
            let detector = OutputDetector::with_env(format, false, env);
            let decision = detector.decide();
            // Should always produce a valid decision
            let _ = decision.use_rich;
            let _ = decision.reason;
        }
    }
}
