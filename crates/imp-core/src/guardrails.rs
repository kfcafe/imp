use std::path::Path;

use project_detect::{detect_walk, ProjectKind};
use serde::{Deserialize, Serialize};

/// How strongly guardrail failures influence agent execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum GuardrailLevel {
    /// Run checks and surface failures clearly, but do not block the turn.
    #[default]
    Advisory,
    /// Run checks and treat failures as blocking.
    Enforce,
}

/// Built-in guardrail starter profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GuardrailProfile {
    /// Infer the profile from the current project using `project-detect`.
    Auto,
    /// Language-neutral fallback profile.
    Generic,
    /// Zig starter profile.
    Zig,
    /// Rust starter profile.
    Rust,
    /// TypeScript starter profile.
    #[serde(rename = "typescript")]
    TypeScript,
    /// C / C-family build-system starter profile.
    C,
    /// Go starter profile.
    Go,
    /// Elixir starter profile.
    Elixir,
}

impl GuardrailProfile {
    /// Resolve a detected project kind to the nearest built-in profile.
    #[must_use]
    pub fn from_project_kind(kind: &ProjectKind) -> Self {
        match kind {
            ProjectKind::Zig => Self::Zig,
            ProjectKind::Cargo => Self::Rust,
            ProjectKind::Go => Self::Go,
            ProjectKind::Elixir { .. } => Self::Elixir,
            ProjectKind::Node { .. } => Self::TypeScript,
            ProjectKind::CMake | ProjectKind::Meson | ProjectKind::Make => Self::C,
            _ => Self::Generic,
        }
    }
}

/// Configurable engineering guardrails for agent-time guidance and checks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardrailConfig {
    /// Master switch. `None` means "use the default".
    pub enabled: Option<bool>,
    /// Advisory vs blocking behavior.
    pub level: Option<GuardrailLevel>,
    /// Built-in profile selection.
    pub profile: Option<GuardrailProfile>,
    /// File globs that should trigger guardrail checks after writes.
    pub critical_paths: Option<Vec<String>>,
    /// Commands to run after writes. `None` means use profile defaults.
    pub after_write: Option<Vec<String>>,
}

impl GuardrailConfig {
    /// Returns whether guardrails are enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.enabled.unwrap_or(false)
    }

    /// Returns the effective configured level.
    #[must_use]
    pub fn effective_level(&self) -> GuardrailLevel {
        self.level.unwrap_or_default()
    }

    /// Returns the configured profile before auto-detection.
    #[must_use]
    pub fn configured_profile(&self) -> GuardrailProfile {
        self.profile.unwrap_or(GuardrailProfile::Generic)
    }

    /// Resolve the effective profile for a path.
    #[must_use]
    pub fn resolve_effective_profile(&self, cwd: &Path) -> GuardrailProfile {
        match self.configured_profile() {
            GuardrailProfile::Auto => detect_walk(cwd)
                .map(|(kind, _)| GuardrailProfile::from_project_kind(&kind))
                .unwrap_or(GuardrailProfile::Generic),
            profile => profile,
        }
    }

    /// Merge another guardrail config into this one.
    pub fn merge(&mut self, other: GuardrailConfig) {
        if other.enabled.is_some() {
            self.enabled = other.enabled;
        }
        if other.level.is_some() {
            self.level = other.level;
        }
        if other.profile.is_some() {
            self.profile = other.profile;
        }
        if other.critical_paths.is_some() {
            self.critical_paths = other.critical_paths;
        }
        if other.after_write.is_some() {
            self.after_write = other.after_write;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use tempfile::TempDir;

    #[derive(Debug, Deserialize)]
    struct GuardrailToml {
        guardrails: GuardrailConfig,
    }

    #[test]
    fn guardrail_toml_deserializes() {
        let parsed: GuardrailToml = toml::from_str(
            r#"
[guardrails]
enabled = true
level = "enforce"
profile = "zig"
critical_paths = ["src/**", "lib/**"]
after_write = ["zig fmt --check .", "zig build"]
"#,
        )
        .unwrap();

        assert_eq!(parsed.guardrails.enabled, Some(true));
        assert_eq!(parsed.guardrails.level, Some(GuardrailLevel::Enforce));
        assert_eq!(parsed.guardrails.profile, Some(GuardrailProfile::Zig));
        assert_eq!(
            parsed.guardrails.critical_paths,
            Some(vec!["src/**".into(), "lib/**".into()])
        );
        assert_eq!(
            parsed.guardrails.after_write,
            Some(vec!["zig fmt --check .".into(), "zig build".into()])
        );
    }

    #[test]
    fn guardrail_auto_profile_resolves_zig() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("build.zig"), "").unwrap();

        let config = GuardrailConfig {
            profile: Some(GuardrailProfile::Auto),
            ..Default::default()
        };

        assert_eq!(
            config.resolve_effective_profile(dir.path()),
            GuardrailProfile::Zig
        );
    }

    #[test]
    fn guardrail_auto_profile_resolves_rust_from_subdirectory() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname='x'\nversion='0.1.0'\n",
        )
        .unwrap();
        let nested = dir.path().join("src").join("nested");
        std::fs::create_dir_all(&nested).unwrap();

        let config = GuardrailConfig {
            profile: Some(GuardrailProfile::Auto),
            ..Default::default()
        };

        assert_eq!(
            config.resolve_effective_profile(&nested),
            GuardrailProfile::Rust
        );
    }

    #[test]
    fn guardrail_auto_profile_resolves_go() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module example.com/test\n").unwrap();

        let config = GuardrailConfig {
            profile: Some(GuardrailProfile::Auto),
            ..Default::default()
        };

        assert_eq!(
            config.resolve_effective_profile(dir.path()),
            GuardrailProfile::Go
        );
    }

    #[test]
    fn guardrail_auto_profile_resolves_elixir() {
        let dir = TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("mix.exs"),
            "defmodule Demo.MixProject do end\n",
        )
        .unwrap();

        let config = GuardrailConfig {
            profile: Some(GuardrailProfile::Auto),
            ..Default::default()
        };

        assert_eq!(
            config.resolve_effective_profile(dir.path()),
            GuardrailProfile::Elixir
        );
    }

    #[test]
    fn guardrail_auto_profile_falls_back_to_generic() {
        let dir = TempDir::new().unwrap();
        let config = GuardrailConfig {
            profile: Some(GuardrailProfile::Auto),
            ..Default::default()
        };

        assert_eq!(
            config.resolve_effective_profile(dir.path()),
            GuardrailProfile::Generic
        );
    }

    #[test]
    fn guardrail_merge_only_overrides_present_fields() {
        let mut base = GuardrailConfig {
            enabled: Some(true),
            level: Some(GuardrailLevel::Advisory),
            profile: Some(GuardrailProfile::Rust),
            critical_paths: Some(vec!["src/**".into()]),
            after_write: None,
        };

        let overlay = GuardrailConfig {
            enabled: None,
            level: Some(GuardrailLevel::Enforce),
            profile: None,
            critical_paths: None,
            after_write: Some(vec!["cargo test".into()]),
        };

        base.merge(overlay);

        assert_eq!(base.enabled, Some(true));
        assert_eq!(base.level, Some(GuardrailLevel::Enforce));
        assert_eq!(base.profile, Some(GuardrailProfile::Rust));
        assert_eq!(base.critical_paths, Some(vec!["src/**".into()]));
        assert_eq!(base.after_write, Some(vec!["cargo test".into()]));
    }
}
