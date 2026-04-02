use serde::{Deserialize, Serialize};

/// Ordered editable words that render the top-line identity sentence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PersonaFocus {
    #[default]
    Coding,
    Engineering,
    Software,
    Debugging,
    Research,
    Writing,
    Planning,
    Operations,
    Analysis,
    General,
}

impl PersonaFocus {
    pub const ALL: &'static [Self] = &[
        Self::Coding,
        Self::Engineering,
        Self::Software,
        Self::Debugging,
        Self::Research,
        Self::Writing,
        Self::Planning,
        Self::Operations,
        Self::Analysis,
        Self::General,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Coding => "coding",
            Self::Engineering => "engineering",
            Self::Software => "software",
            Self::Debugging => "debugging",
            Self::Research => "research",
            Self::Writing => "writing",
            Self::Planning => "planning",
            Self::Operations => "operations",
            Self::Analysis => "analysis",
            Self::General => "general",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PersonaRole {
    #[default]
    Agent,
    Assistant,
    Worker,
    Collaborator,
    Partner,
    Reviewer,
    Planner,
}

impl PersonaRole {
    pub const ALL: &'static [Self] = &[
        Self::Agent,
        Self::Assistant,
        Self::Worker,
        Self::Collaborator,
        Self::Partner,
        Self::Reviewer,
        Self::Planner,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Agent => "agent",
            Self::Assistant => "assistant",
            Self::Worker => "worker",
            Self::Collaborator => "collaborator",
            Self::Partner => "partner",
            Self::Reviewer => "reviewer",
            Self::Planner => "planner",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WorkStyleWord {
    #[default]
    Practical,
    Careful,
    Disciplined,
    Methodical,
    Focused,
    Thorough,
    Precise,
    Deliberate,
    Skeptical,
    Patient,
}

impl WorkStyleWord {
    pub const ALL: &'static [Self] = &[
        Self::Practical,
        Self::Careful,
        Self::Disciplined,
        Self::Methodical,
        Self::Focused,
        Self::Thorough,
        Self::Precise,
        Self::Deliberate,
        Self::Skeptical,
        Self::Patient,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Practical => "practical",
            Self::Careful => "careful",
            Self::Disciplined => "disciplined",
            Self::Methodical => "methodical",
            Self::Focused => "focused",
            Self::Thorough => "thorough",
            Self::Precise => "precise",
            Self::Deliberate => "deliberate",
            Self::Skeptical => "skeptical",
            Self::Patient => "patient",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum VoiceWord {
    #[default]
    Concise,
    Clear,
    Direct,
    Calm,
    Thoughtful,
    Collaborative,
    Structured,
    Friendly,
    Terse,
    Warm,
}

impl VoiceWord {
    pub const ALL: &'static [Self] = &[
        Self::Concise,
        Self::Clear,
        Self::Direct,
        Self::Calm,
        Self::Thoughtful,
        Self::Collaborative,
        Self::Structured,
        Self::Friendly,
        Self::Terse,
        Self::Warm,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Concise => "concise",
            Self::Clear => "clear",
            Self::Direct => "direct",
            Self::Calm => "calm",
            Self::Thoughtful => "thoughtful",
            Self::Collaborative => "collaborative",
            Self::Structured => "structured",
            Self::Friendly => "friendly",
            Self::Terse => "terse",
            Self::Warm => "warm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PersonalityBand {
    VeryLow,
    Low,
    #[default]
    Medium,
    High,
    VeryHigh,
}

impl PersonalityBand {
    pub const ALL: &'static [Self] = &[
        Self::VeryLow,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::VeryHigh,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::VeryLow => "very-low",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::VeryHigh => "very-high",
        }
    }

    pub fn ui_label(&self) -> &'static str {
        match self {
            Self::VeryLow => "very low",
            Self::Low => "low",
            Self::Medium => "balanced",
            Self::High => "high",
            Self::VeryHigh => "very high",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PersonalityOption {
    pub value: &'static str,
    pub label: &'static str,
    pub hint: &'static str,
}

pub const WORK_STYLE_OPTIONS: &[PersonalityOption] = &[
    PersonalityOption {
        value: "practical",
        label: "practical",
        hint: "favor concrete progress over theory",
    },
    PersonalityOption {
        value: "careful",
        label: "careful",
        hint: "avoid careless changes and read closely",
    },
    PersonalityOption {
        value: "disciplined",
        label: "disciplined",
        hint: "stay consistent and procedure-minded",
    },
    PersonalityOption {
        value: "methodical",
        label: "methodical",
        hint: "work step by step",
    },
    PersonalityOption {
        value: "focused",
        label: "focused",
        hint: "minimize drift and stay on task",
    },
    PersonalityOption {
        value: "thorough",
        label: "thorough",
        hint: "inspect details before concluding",
    },
    PersonalityOption {
        value: "precise",
        label: "precise",
        hint: "optimize for exactness",
    },
    PersonalityOption {
        value: "deliberate",
        label: "deliberate",
        hint: "slow down before important choices",
    },
    PersonalityOption {
        value: "skeptical",
        label: "skeptical",
        hint: "challenge assumptions and verify them",
    },
    PersonalityOption {
        value: "patient",
        label: "patient",
        hint: "take the time a problem needs",
    },
];

pub const VOICE_OPTIONS: &[PersonalityOption] = &[
    PersonalityOption {
        value: "concise",
        label: "concise",
        hint: "keep responses compact",
    },
    PersonalityOption {
        value: "clear",
        label: "clear",
        hint: "optimize for easy understanding",
    },
    PersonalityOption {
        value: "direct",
        label: "direct",
        hint: "be straightforward and plainspoken",
    },
    PersonalityOption {
        value: "calm",
        label: "calm",
        hint: "stay steady and unflustered",
    },
    PersonalityOption {
        value: "thoughtful",
        label: "thoughtful",
        hint: "show measured consideration",
    },
    PersonalityOption {
        value: "collaborative",
        label: "collaborative",
        hint: "feel like a teammate",
    },
    PersonalityOption {
        value: "structured",
        label: "structured",
        hint: "organize responses cleanly",
    },
    PersonalityOption {
        value: "friendly",
        label: "friendly",
        hint: "be approachable without fluff",
    },
    PersonalityOption {
        value: "terse",
        label: "terse",
        hint: "be extremely brief",
    },
    PersonalityOption {
        value: "warm",
        label: "warm",
        hint: "be supportive and human",
    },
];

pub const FOCUS_OPTIONS: &[PersonalityOption] = &[
    PersonalityOption {
        value: "coding",
        label: "coding",
        hint: "default toward software implementation work",
    },
    PersonalityOption {
        value: "engineering",
        label: "engineering",
        hint: "broader technical systems work",
    },
    PersonalityOption {
        value: "software",
        label: "software",
        hint: "general software problem solving",
    },
    PersonalityOption {
        value: "debugging",
        label: "debugging",
        hint: "default toward diagnosis and repair",
    },
    PersonalityOption {
        value: "research",
        label: "research",
        hint: "default toward investigation and synthesis",
    },
    PersonalityOption {
        value: "writing",
        label: "writing",
        hint: "default toward prose and editing",
    },
    PersonalityOption {
        value: "planning",
        label: "planning",
        hint: "default toward breakdown and sequencing",
    },
    PersonalityOption {
        value: "operations",
        label: "operations",
        hint: "default toward runbooks and systems ops",
    },
    PersonalityOption {
        value: "analysis",
        label: "analysis",
        hint: "default toward reasoning and evaluation",
    },
    PersonalityOption {
        value: "general",
        label: "general",
        hint: "stay broadly applicable",
    },
];

pub const ROLE_OPTIONS: &[PersonalityOption] = &[
    PersonalityOption {
        value: "agent",
        label: "agent",
        hint: "active and tool-using",
    },
    PersonalityOption {
        value: "assistant",
        label: "assistant",
        hint: "more consultative and supportive",
    },
    PersonalityOption {
        value: "worker",
        label: "worker",
        hint: "task-oriented and execution-focused",
    },
    PersonalityOption {
        value: "collaborator",
        label: "collaborator",
        hint: "peer-like and cooperative",
    },
    PersonalityOption {
        value: "partner",
        label: "partner",
        hint: "close and team-oriented",
    },
    PersonalityOption {
        value: "reviewer",
        label: "reviewer",
        hint: "critical and evaluative",
    },
    PersonalityOption {
        value: "planner",
        label: "planner",
        hint: "organizational and sequencing-focused",
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonalityIdentity {
    #[serde(default = "default_personality_name")]
    pub name: String,
    #[serde(default)]
    pub work_style: WorkStyleWord,
    #[serde(default)]
    pub voice: VoiceWord,
    #[serde(default)]
    pub focus: PersonaFocus,
    #[serde(default)]
    pub role: PersonaRole,
}

impl PersonalityIdentity {
    pub fn render_sentence(&self) -> String {
        format!(
            "You are {}, a {}, {}, {} {}.",
            self.name,
            self.work_style.as_str(),
            self.voice.as_str(),
            self.focus.as_str(),
            self.role.as_str()
        )
    }
}

impl Default for PersonalityIdentity {
    fn default() -> Self {
        Self {
            name: default_personality_name(),
            work_style: WorkStyleWord::default(),
            voice: VoiceWord::default(),
            focus: PersonaFocus::default(),
            role: PersonaRole::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonalitySliders {
    #[serde(default)]
    pub autonomy: PersonalityBand,
    #[serde(default)]
    pub verbosity: PersonalityBand,
    #[serde(default)]
    pub caution: PersonalityBand,
    #[serde(default)]
    pub warmth: PersonalityBand,
    #[serde(default)]
    pub planning_depth: PersonalityBand,
}

impl Default for PersonalitySliders {
    fn default() -> Self {
        Self {
            autonomy: PersonalityBand::High,
            verbosity: PersonalityBand::Low,
            caution: PersonalityBand::High,
            warmth: PersonalityBand::Medium,
            planning_depth: PersonalityBand::Medium,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersonalityProfile {
    #[serde(default)]
    pub identity: PersonalityIdentity,
    #[serde(default)]
    pub sliders: PersonalitySliders,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersonalityProfiles {
    #[serde(default)]
    pub active: Option<String>,
    #[serde(default)]
    pub saved: std::collections::BTreeMap<String, PersonalityProfile>,
}

impl PersonalityProfiles {
    pub fn active_profile(&self) -> Option<&PersonalityProfile> {
        self.active
            .as_ref()
            .and_then(|name| self.saved.get(name.as_str()))
    }

    pub fn set_active(&mut self, name: impl Into<String>) {
        self.active = Some(name.into());
    }

    pub fn clear_active(&mut self) {
        self.active = None;
    }

    pub fn save_profile(&mut self, name: impl Into<String>, profile: PersonalityProfile) -> String {
        let name = sanitize_profile_name(&name.into());
        self.saved.insert(name.clone(), profile);
        self.active = Some(name.clone());
        name
    }

    pub fn delete_profile(&mut self, name: &str) -> bool {
        let removed = self.saved.remove(name).is_some();
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }
        removed
    }

    pub fn rename_profile(&mut self, from: &str, to: impl Into<String>) -> bool {
        let Some(profile) = self.saved.remove(from) else {
            return false;
        };
        let to = sanitize_profile_name(&to.into());
        self.saved.insert(to.clone(), profile);
        if self.active.as_deref() == Some(from) {
            self.active = Some(to);
        }
        true
    }

    pub fn profile_names(&self) -> Vec<String> {
        self.saved.keys().cloned().collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PersonalityConfig {
    #[serde(default)]
    pub profile: PersonalityProfile,
    #[serde(default)]
    pub profiles: PersonalityProfiles,
}

impl PersonalityConfig {
    pub fn merge(&mut self, other: Self) {
        self.profile = other.profile;
        if other.profiles.active.is_some() {
            self.profiles.active = other.profiles.active;
        }
        self.profiles.saved.extend(other.profiles.saved);
    }

    pub fn effective_profile(&self) -> &PersonalityProfile {
        self.profiles.active_profile().unwrap_or(&self.profile)
    }
}

fn sanitize_profile_name(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "profile".to_string();
    }
    trimmed
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

fn default_personality_name() -> String {
    "imp".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_identity_sentence_is_strong_and_compact() {
        let identity = PersonalityIdentity::default();
        assert_eq!(
            identity.render_sentence(),
            "You are imp, a practical, concise, coding agent."
        );
    }

    #[test]
    fn personality_config_merge_overrides_profile_and_extends_saved_profiles() {
        let mut base = PersonalityConfig::default();
        base.profiles.active = Some("builder".into());
        base.profiles.saved.insert(
            "builder".into(),
            PersonalityProfile {
                identity: PersonalityIdentity::default(),
                sliders: PersonalitySliders::default(),
            },
        );

        let mut overlay = PersonalityConfig::default();
        overlay.profile.identity.name = "Nova".into();
        overlay.profiles.active = Some("researcher".into());
        overlay.profiles.saved.insert(
            "researcher".into(),
            PersonalityProfile {
                identity: PersonalityIdentity {
                    name: "Nova".into(),
                    focus: PersonaFocus::Research,
                    role: PersonaRole::Assistant,
                    ..PersonalityIdentity::default()
                },
                sliders: PersonalitySliders::default(),
            },
        );

        base.merge(overlay);

        assert_eq!(base.profile.identity.name, "Nova");
        assert_eq!(base.profiles.active.as_deref(), Some("researcher"));
        assert!(base.profiles.saved.contains_key("builder"));
        assert!(base.profiles.saved.contains_key("researcher"));
    }

    #[test]
    fn profiles_can_save_activate_rename_and_delete() {
        let mut profiles = PersonalityProfiles::default();
        let saved = profiles.save_profile("Builder", PersonalityProfile::default());
        assert_eq!(saved, "Builder");
        assert_eq!(profiles.active.as_deref(), Some("Builder"));
        assert!(profiles.saved.contains_key("Builder"));

        assert!(profiles.rename_profile("Builder", "Reviewer"));
        assert_eq!(profiles.active.as_deref(), Some("Reviewer"));
        assert!(profiles.saved.contains_key("Reviewer"));
        assert!(!profiles.saved.contains_key("Builder"));

        assert!(profiles.delete_profile("Reviewer"));
        assert!(profiles.active.is_none());
        assert!(profiles.saved.is_empty());
    }

    #[test]
    fn config_effective_profile_prefers_active_saved_profile() {
        let mut config = PersonalityConfig::default();
        config.profile.identity.name = "imp".into();
        config.profiles.save_profile(
            "Researcher",
            PersonalityProfile {
                identity: PersonalityIdentity {
                    name: "Nova".into(),
                    focus: PersonaFocus::Research,
                    role: PersonaRole::Assistant,
                    ..PersonalityIdentity::default()
                },
                sliders: PersonalitySliders::default(),
            },
        );

        assert_eq!(config.effective_profile().identity.name, "Nova");
    }

    #[test]
    fn option_lists_and_band_labels_match_defaults() {
        assert!(WORK_STYLE_OPTIONS.iter().any(|o| o.value == "practical"));
        assert!(VOICE_OPTIONS.iter().any(|o| o.value == "concise"));
        assert!(FOCUS_OPTIONS.iter().any(|o| o.value == "coding"));
        assert!(ROLE_OPTIONS.iter().any(|o| o.value == "agent"));
        assert_eq!(PersonalityBand::Medium.ui_label(), "balanced");
    }
}
