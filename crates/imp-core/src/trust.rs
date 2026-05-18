use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Provenance {
    pub id: Option<String>,
    pub source: ProvenanceSource,
    pub trust: TrustLabel,
    pub risk: Vec<RiskLabel>,
    pub origin: Option<String>,
    pub artifact_ref: Option<PathBuf>,
    pub derived_from: Vec<DerivedFrom>,
    pub notes: Vec<String>,
}

impl Provenance {
    pub fn new(source: ProvenanceSource) -> Self {
        let trust = source.default_trust_label();
        let risk = source.default_risk_labels();
        Self {
            source,
            trust,
            risk,
            ..Self::default()
        }
    }

    pub fn user_instruction() -> Self {
        Self::new(ProvenanceSource::UserInstruction)
    }

    pub fn workspace_file(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        let mut provenance = Self::new(ProvenanceSource::WorkspaceFile { path: path.clone() });
        provenance.origin = Some(path.display().to_string());
        provenance
    }

    pub fn external_web(url: impl Into<String>) -> Self {
        let url = url.into();
        let mut provenance = Self::new(ProvenanceSource::ExternalWebContent {
            url: Some(url.clone()),
        });
        provenance.origin = Some(url);
        provenance
    }

    pub fn tool_observation(tool_name: impl Into<String>) -> Self {
        Self::new(ProvenanceSource::ToolObservation {
            tool_name: Some(tool_name.into()),
        })
    }

    pub fn verifier_output(gate_id: impl Into<String>) -> Self {
        Self::new(ProvenanceSource::VerifierOutput {
            gate_id: Some(gate_id.into()),
        })
    }

    pub fn durable_memory(key: impl Into<String>) -> Self {
        Self::new(ProvenanceSource::DurableMemory {
            key: Some(key.into()),
        })
    }

    pub fn generated_summary(parents: impl IntoIterator<Item = Provenance>) -> Self {
        let parents: Vec<Provenance> = parents.into_iter().collect();
        let mut provenance = Self::new(ProvenanceSource::GeneratedSummary);
        provenance.derived_from = parents.iter().map(DerivedFrom::from).collect();
        provenance.trust = lowest_authority_trust(parents.iter().map(|parent| parent.trust));
        provenance.risk = merge_risk_labels(
            parents
                .iter()
                .flat_map(|parent| parent.risk.iter().copied())
                .chain([RiskLabel::Generated]),
        );
        provenance
    }

    pub fn mana_record(kind: ManaRecordKind, unit_id: impl Into<String>) -> Self {
        let unit_id = unit_id.into();
        let mut provenance = Self::new(ProvenanceSource::ManaRecord {
            kind,
            unit_id: Some(unit_id.clone()),
        });
        provenance.origin = Some(unit_id);
        provenance
    }

    pub fn with_risk(mut self, risk: RiskLabel) -> Self {
        if !self.risk.contains(&risk) {
            self.risk.push(risk);
        }
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn is_low_trust(&self) -> bool {
        self.trust.is_low_trust() || self.risk.contains(&RiskLabel::LowTrust)
    }
}

impl Default for Provenance {
    fn default() -> Self {
        Self {
            id: None,
            source: ProvenanceSource::Unknown,
            trust: TrustLabel::Unknown,
            risk: vec![RiskLabel::LowTrust],
            origin: None,
            artifact_ref: None,
            derived_from: Vec::new(),
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TrustedContext<T> {
    pub value: T,
    pub provenance: Provenance,
}

impl<T> TrustedContext<T> {
    pub fn new(value: T, provenance: Provenance) -> Self {
        Self { value, provenance }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> TrustedContext<U> {
        TrustedContext {
            value: f(self.value),
            provenance: self.provenance,
        }
    }
}

impl<T: Default> Default for TrustedContext<T> {
    fn default() -> Self {
        Self {
            value: T::default(),
            provenance: Provenance::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustLabel {
    UserInstruction,
    ProjectTrusted,
    ToolObserved,
    ExternalUntrusted,
    DurableMemory,
    GeneratedSummary,
    VerifierOutput,
    ManaLedger,
    Unknown,
}

impl TrustLabel {
    pub fn is_low_trust(self) -> bool {
        matches!(
            self,
            Self::ExternalUntrusted | Self::GeneratedSummary | Self::Unknown
        )
    }

    fn authority_rank(self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::ExternalUntrusted => 1,
            Self::GeneratedSummary => 2,
            Self::ToolObserved => 3,
            Self::DurableMemory => 4,
            Self::ProjectTrusted => 5,
            Self::VerifierOutput => 6,
            Self::ManaLedger => 7,
            Self::UserInstruction => 8,
        }
    }
}

impl Default for TrustLabel {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", tag = "source")]
pub enum ProvenanceSource {
    UserInstruction,
    WorkspaceFile {
        path: PathBuf,
    },
    ExternalWebContent {
        url: Option<String>,
    },
    ToolObservation {
        tool_name: Option<String>,
    },
    VerifierOutput {
        gate_id: Option<String>,
    },
    DurableMemory {
        key: Option<String>,
    },
    GeneratedSummary,
    ManaRecord {
        kind: ManaRecordKind,
        unit_id: Option<String>,
    },
    SystemPolicy,
    Extension {
        id: String,
    },
    Unknown,
}

impl ProvenanceSource {
    pub fn default_trust_label(&self) -> TrustLabel {
        match self {
            Self::UserInstruction => TrustLabel::UserInstruction,
            Self::WorkspaceFile { .. } | Self::SystemPolicy => TrustLabel::ProjectTrusted,
            Self::ExternalWebContent { .. } | Self::Unknown => TrustLabel::ExternalUntrusted,
            Self::ToolObservation { .. } => TrustLabel::ToolObserved,
            Self::VerifierOutput { .. } => TrustLabel::VerifierOutput,
            Self::DurableMemory { .. } => TrustLabel::DurableMemory,
            Self::GeneratedSummary => TrustLabel::GeneratedSummary,
            Self::ManaRecord { .. } => TrustLabel::ManaLedger,
            Self::Extension { .. } => TrustLabel::ToolObserved,
        }
    }

    pub fn default_risk_labels(&self) -> Vec<RiskLabel> {
        match self {
            Self::UserInstruction => vec![RiskLabel::UserAuthoritative],
            Self::WorkspaceFile { .. } | Self::SystemPolicy => vec![RiskLabel::ProjectPolicy],
            Self::ExternalWebContent { .. } => vec![
                RiskLabel::External,
                RiskLabel::LowTrust,
                RiskLabel::NetworkDerived,
            ],
            Self::ToolObservation { .. } => vec![RiskLabel::ToolOutput],
            Self::VerifierOutput { .. } => vec![RiskLabel::VerificationArtifact],
            Self::DurableMemory { .. } => vec![RiskLabel::DurableLedger],
            Self::GeneratedSummary => vec![RiskLabel::Generated],
            Self::ManaRecord { .. } => vec![RiskLabel::DurableLedger],
            Self::Extension { .. } => vec![RiskLabel::ToolOutput],
            Self::Unknown => vec![RiskLabel::LowTrust],
        }
    }
}

impl Default for ProvenanceSource {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RiskLabel {
    LowTrust,
    UserAuthoritative,
    ProjectPolicy,
    External,
    Stale,
    Generated,
    ToolOutput,
    ContainsInstructions,
    PossiblePromptInjection,
    SecretAdjacent,
    NetworkDerived,
    VerificationArtifact,
    DurableLedger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DerivedFrom {
    pub provenance_id: Option<String>,
    pub source: ProvenanceSource,
    pub trust: TrustLabel,
    pub risk: Vec<RiskLabel>,
    pub origin: Option<String>,
}

impl From<&Provenance> for DerivedFrom {
    fn from(provenance: &Provenance) -> Self {
        Self {
            provenance_id: provenance.id.clone(),
            source: provenance.source.clone(),
            trust: provenance.trust,
            risk: provenance.risk.clone(),
            origin: provenance.origin.clone(),
        }
    }
}

impl Default for DerivedFrom {
    fn default() -> Self {
        Self {
            provenance_id: None,
            source: ProvenanceSource::Unknown,
            trust: TrustLabel::Unknown,
            risk: vec![RiskLabel::LowTrust],
            origin: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ManaRecordKind {
    Fact,
    Note,
    Decision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrustBoundary {
    User,
    Project,
    External,
    Tool,
    Verifier,
    Memory,
    ManaLedger,
    Generated,
    Extension,
    Unknown,
}

impl From<&ProvenanceSource> for TrustBoundary {
    fn from(source: &ProvenanceSource) -> Self {
        match source {
            ProvenanceSource::UserInstruction => Self::User,
            ProvenanceSource::WorkspaceFile { .. } | ProvenanceSource::SystemPolicy => {
                Self::Project
            }
            ProvenanceSource::ExternalWebContent { .. } => Self::External,
            ProvenanceSource::ToolObservation { .. } => Self::Tool,
            ProvenanceSource::VerifierOutput { .. } => Self::Verifier,
            ProvenanceSource::DurableMemory { .. } => Self::Memory,
            ProvenanceSource::GeneratedSummary => Self::Generated,
            ProvenanceSource::ManaRecord { .. } => Self::ManaLedger,
            ProvenanceSource::Extension { .. } => Self::Extension,
            ProvenanceSource::Unknown => Self::Unknown,
        }
    }
}

fn lowest_authority_trust(trusts: impl IntoIterator<Item = TrustLabel>) -> TrustLabel {
    trusts
        .into_iter()
        .min_by_key(|trust| trust.authority_rank())
        .unwrap_or(TrustLabel::GeneratedSummary)
}

fn merge_risk_labels(labels: impl IntoIterator<Item = RiskLabel>) -> Vec<RiskLabel> {
    let mut merged = Vec::new();
    for label in labels {
        if !merged.contains(&label) {
            merged.push(label);
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trust_labels_default_source_categories() {
        assert_eq!(
            Provenance::user_instruction().trust,
            TrustLabel::UserInstruction
        );
        assert_eq!(
            Provenance::workspace_file("src/lib.rs").trust,
            TrustLabel::ProjectTrusted
        );
        let web = Provenance::external_web("https://example.com");
        assert_eq!(web.trust, TrustLabel::ExternalUntrusted);
        assert!(web.risk.contains(&RiskLabel::LowTrust));
        assert!(web.risk.contains(&RiskLabel::NetworkDerived));
        assert_eq!(
            Provenance::tool_observation("bash").trust,
            TrustLabel::ToolObserved
        );
        assert_eq!(
            Provenance::verifier_output("unit").trust,
            TrustLabel::VerifierOutput
        );
        assert_eq!(
            Provenance::durable_memory("project-style").trust,
            TrustLabel::DurableMemory
        );
        assert_eq!(
            Provenance::mana_record(ManaRecordKind::Fact, "394.8").trust,
            TrustLabel::ManaLedger
        );
    }

    #[test]
    fn trust_labels_serde_roundtrip() {
        let provenance = Provenance::external_web("https://example.com")
            .with_risk(RiskLabel::PossiblePromptInjection)
            .with_note("observed instruction-like content");
        let json = serde_json::to_string(&provenance).unwrap();
        assert!(json.contains("external-web-content"));
        assert!(json.contains("possible-prompt-injection"));
        let decoded: Provenance = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, provenance);
    }

    #[test]
    fn trust_labels_generated_summary_preserves_derivation_and_lowest_trust() {
        let mut user = Provenance::user_instruction();
        user.id = Some("ctx_user".into());
        let mut web = Provenance::external_web("https://example.com")
            .with_risk(RiskLabel::ContainsInstructions)
            .with_risk(RiskLabel::PossiblePromptInjection);
        web.id = Some("ctx_web".into());

        let summary = Provenance::generated_summary([user.clone(), web.clone()]);
        assert_eq!(summary.trust, TrustLabel::ExternalUntrusted);
        assert!(summary.risk.contains(&RiskLabel::Generated));
        assert!(summary.risk.contains(&RiskLabel::PossiblePromptInjection));
        assert_eq!(summary.derived_from.len(), 2);
        assert_eq!(
            summary.derived_from[0].provenance_id.as_deref(),
            Some("ctx_user")
        );
        assert_eq!(
            summary.derived_from[1].provenance_id.as_deref(),
            Some("ctx_web")
        );
    }

    #[test]
    fn trust_labels_trusted_context_maps_value_without_losing_provenance() {
        let context =
            TrustedContext::new("hello".to_string(), Provenance::workspace_file("README.md"));
        let mapped = context.map(|value| value.len());
        assert_eq!(mapped.value, 5);
        assert_eq!(mapped.provenance.trust, TrustLabel::ProjectTrusted);
        assert_eq!(
            TrustBoundary::from(&mapped.provenance.source),
            TrustBoundary::Project
        );
    }
}
