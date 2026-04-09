use std::collections::{BTreeMap, BTreeSet, HashMap};

use mana_core::unit::{Status, Unit, UnitKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaReviewState {
    NoChange,
    Changed,
    NeedsDecision,
}

impl ManaReviewState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoChange => "no_change",
            Self::Changed => "changed",
            Self::NeedsDecision => "needs_decision",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaReviewScopeKind {
    None,
    Project,
    Root,
    ExplicitPath,
    Mixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaReviewScope {
    pub kind: ManaReviewScopeKind,
    pub display: String,
}

impl Default for ManaReviewScope {
    fn default() -> Self {
        Self {
            kind: ManaReviewScopeKind::None,
            display: "none".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ManaUnitRef {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

impl ManaUnitRef {
    pub fn from_snapshot(unit: &ManaUnitSnapshot) -> Self {
        Self {
            id: unit.id.clone(),
            title: unit.title.clone(),
            kind: Some(unit.kind.as_str().to_string()),
        }
    }

    pub fn new(id: impl Into<String>, title: impl Into<String>, kind: Option<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaUnitSnapshot {
    pub id: String,
    pub title: String,
    pub kind: ManaReviewUnitKind,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    pub priority: u8,
    pub is_archived: bool,
}

impl ManaUnitSnapshot {
    pub fn unit_ref(&self) -> ManaUnitRef {
        ManaUnitRef::from_snapshot(self)
    }
}

impl From<&Unit> for ManaUnitSnapshot {
    fn from(unit: &Unit) -> Self {
        Self {
            id: unit.id.clone(),
            title: unit.title.clone(),
            kind: ManaReviewUnitKind::from_unit(unit),
            status: unit.status.to_string(),
            parent: unit.parent.clone(),
            dependencies: unit.dependencies.clone(),
            labels: unit.labels.clone(),
            decisions: unit.decisions.clone(),
            description: unit.description.clone(),
            acceptance: unit.acceptance.clone(),
            design: unit.design.clone(),
            assignee: unit.assignee.clone(),
            priority: unit.priority,
            is_archived: unit.is_archived,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaReviewUnitKind {
    Epic,
    Job,
    Fact,
}

impl ManaReviewUnitKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Epic => "epic",
            Self::Job => "job",
            Self::Fact => "fact",
        }
    }

    pub fn from_unit(unit: &Unit) -> Self {
        match unit.kind {
            UnitKind::Epic => Self::Epic,
            UnitKind::Job => Self::Job,
            UnitKind::Fact => Self::Fact,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaMutationAction {
    Create,
    Close,
    Update,
    NotesAppend,
    DecisionAdd,
    DecisionResolve,
    Reopen,
    Fail,
    Delete,
    DepAdd,
    DepRemove,
    FactCreate,
}

impl ManaMutationAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Close => "close",
            Self::Update => "update",
            Self::NotesAppend => "notes_append",
            Self::DecisionAdd => "decision_add",
            Self::DecisionResolve => "decision_resolve",
            Self::Reopen => "reopen",
            Self::Fail => "fail",
            Self::Delete => "delete",
            Self::DepAdd => "dep_add",
            Self::DepRemove => "dep_remove",
            Self::FactCreate => "fact_create",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaTouchKind {
    Created,
    Updated,
    Closed,
    Reopened,
    Failed,
    Deleted,
    FactCreated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaUnitOrigin {
    Preexisting,
    CreatedInTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaUnitRole {
    Anchor,
    Child,
    Fact,
    DirectTarget,
    Related,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaAnchorKind {
    ReusedExisting,
    CreatedInTurn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaAnchorReason {
    AttachedParent,
    CreatedParent,
    PrimaryTarget,
    PrimaryFact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnManaAnchorUnit {
    pub unit: ManaUnitRef,
    pub anchor_kind: ManaAnchorKind,
    pub reason: ManaAnchorReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnManaTouchedUnit {
    pub unit: ManaUnitRef,
    pub touch_kind: ManaTouchKind,
    pub unit_origin: ManaUnitOrigin,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<ManaUnitRole>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnManaProposedChild {
    pub unit: ManaUnitRef,
    pub parent: ManaUnitRef,
    pub child_kind: ManaReviewUnitKind,
    pub child_origin: ManaUnitOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaFieldChangeKind {
    Set,
    Added,
    Removed,
    Replaced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnManaFieldChange {
    pub unit: ManaUnitRef,
    pub field: String,
    pub change_kind: ManaFieldChangeKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,
    pub source_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnManaNoteAppend {
    pub unit: ManaUnitRef,
    pub appended_text: String,
    pub source_action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaDecisionEventKind {
    Added,
    Resolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnManaDecisionEvent {
    pub unit: ManaUnitRef,
    pub event_kind: ManaDecisionEventKind,
    pub decision_text: String,
    pub source_action: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManaConsequentialChoiceCategory {
    OwnershipBoundary,
    Architecture,
    ExecutionLaunch,
    ScopeChange,
    PruneOrDelete,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnManaConsequentialChoice {
    pub unit: ManaUnitRef,
    pub decision_text: String,
    pub category: ManaConsequentialChoiceCategory,
    pub why_consequential: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_question: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnManaReview {
    pub turn_index: u32,
    pub state: ManaReviewState,
    pub scope: ManaReviewScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_unit: Option<TurnManaAnchorUnit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touched_units: Vec<TurnManaTouchedUnit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub proposed_children: Vec<TurnManaProposedChild>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub material_field_changes: Vec<TurnManaFieldChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes_appended: Vec<TurnManaNoteAppend>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decision_events: Vec<TurnManaDecisionEvent>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unresolved_consequential_choices: Vec<TurnManaConsequentialChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_question: Option<String>,
}

impl TurnManaReview {
    pub fn no_change(turn_index: u32) -> Self {
        Self {
            turn_index,
            state: ManaReviewState::NoChange,
            scope: ManaReviewScope::default(),
            anchor_unit: None,
            touched_units: Vec::new(),
            proposed_children: Vec::new(),
            material_field_changes: Vec::new(),
            notes_appended: Vec::new(),
            decision_events: Vec::new(),
            unresolved_consequential_choices: Vec::new(),
            next_question: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManaMutationRecord {
    pub action: ManaMutationAction,
    pub scope: ManaReviewScope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_unit: Option<ManaUnitSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_unit: Option<ManaUnitSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deleted_unit: Option<ManaUnitRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_unit: Option<ManaUnitRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_unit: Option<ManaUnitRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_changes: Vec<TurnManaFieldChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes_appended: Vec<TurnManaNoteAppend>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decision_events: Vec<TurnManaDecisionEvent>,
}

#[derive(Debug, Default)]
pub struct TurnManaReviewAccumulator {
    turn_index: u32,
    mutations: Vec<ManaMutationRecord>,
}

impl TurnManaReviewAccumulator {
    pub fn begin_turn(&mut self, turn_index: u32) {
        self.turn_index = turn_index;
        self.mutations.clear();
    }

    pub fn push(&mut self, record: ManaMutationRecord) {
        self.mutations.push(record);
    }

    pub fn finalize(&self) -> TurnManaReview {
        if self.mutations.is_empty() {
            return TurnManaReview::no_change(self.turn_index);
        }

        let scope = summarize_scope(&self.mutations);
        let mut aggregates: BTreeMap<String, UnitAggregate> = BTreeMap::new();

        for mutation in &self.mutations {
            match mutation.action {
                ManaMutationAction::Delete => {
                    if let Some(unit) = &mutation.deleted_unit {
                        let aggregate = aggregates
                            .entry(unit.id.clone())
                            .or_insert_with(|| UnitAggregate::new(unit.clone()));
                        aggregate.deleted_in_turn = true;
                        aggregate.touch_actions.insert(ManaTouchKind::Deleted);
                        aggregate.roles.insert(ManaUnitRole::DirectTarget);
                    }
                }
                _ => {
                    let Some(unit) = mutation
                        .after_unit
                        .as_ref()
                        .or(mutation.before_unit.as_ref())
                    else {
                        continue;
                    };
                    let unit_ref = unit.unit_ref();
                    let aggregate = aggregates
                        .entry(unit.id.clone())
                        .or_insert_with(|| UnitAggregate::new(unit_ref));

                    if aggregate.first_before.is_none() {
                        aggregate.first_before = mutation.before_unit.clone();
                    }
                    if mutation.before_unit.is_some() && aggregate.first_before.is_none() {
                        aggregate.first_before = mutation.before_unit.clone();
                    }
                    if let Some(before) = &mutation.before_unit {
                        aggregate.first_before.get_or_insert_with(|| before.clone());
                    }
                    if let Some(after) = &mutation.after_unit {
                        aggregate.latest_after = Some(after.clone());
                    }
                    aggregate.roles.insert(ManaUnitRole::DirectTarget);
                    if matches!(unit.kind, ManaReviewUnitKind::Fact) {
                        aggregate.roles.insert(ManaUnitRole::Fact);
                    }
                    if let Some(parent) = &mutation.parent_unit {
                        aggregate.parent_unit = Some(parent.clone());
                        aggregate.roles.insert(ManaUnitRole::Child);
                    } else if let Some(parent_id) = unit.parent.as_ref() {
                        aggregate.parent_unit.get_or_insert_with(|| {
                            ManaUnitRef::new(parent_id.clone(), parent_id.clone(), Some("epic".into()))
                        });
                        aggregate.roles.insert(ManaUnitRole::Child);
                    }

                    match mutation.action {
                        ManaMutationAction::Create => {
                            aggregate.created_in_turn = true;
                            aggregate.touch_actions.insert(ManaTouchKind::Created);
                        }
                        ManaMutationAction::FactCreate => {
                            aggregate.created_in_turn = true;
                            aggregate.touch_actions.insert(ManaTouchKind::FactCreated);
                            aggregate.roles.insert(ManaUnitRole::Fact);
                        }
                        ManaMutationAction::Close => {
                            aggregate.touch_actions.insert(ManaTouchKind::Closed);
                        }
                        ManaMutationAction::Reopen => {
                            aggregate.touch_actions.insert(ManaTouchKind::Reopened);
                        }
                        ManaMutationAction::Fail => {
                            aggregate.touch_actions.insert(ManaTouchKind::Failed);
                        }
                        _ => {
                            aggregate.touch_actions.insert(ManaTouchKind::Updated);
                        }
                    }
                }
            }

            if let Some(related) = &mutation.related_unit {
                let aggregate = aggregates
                    .entry(related.id.clone())
                    .or_insert_with(|| UnitAggregate::new(related.clone()));
                aggregate.roles.insert(ManaUnitRole::Related);
            }

            for field_change in &mutation.field_changes {
                if let Some(aggregate) = aggregates.get_mut(&field_change.unit.id) {
                    aggregate.record_field_change(field_change.clone());
                }
            }
            for note in &mutation.notes_appended {
                if let Some(aggregate) = aggregates.get_mut(&note.unit.id) {
                    aggregate.notes.push(note.clone());
                }
            }
            for event in &mutation.decision_events {
                if let Some(aggregate) = aggregates.get_mut(&event.unit.id) {
                    aggregate.record_decision_event(event.clone());
                }
            }
        }

        let created_and_deleted: BTreeSet<String> = aggregates
            .iter()
            .filter_map(|(id, aggregate)| {
                (aggregate.created_in_turn && aggregate.deleted_in_turn).then_some(id.clone())
            })
            .collect();

        let mut touched_units = Vec::new();
        let mut proposed_children = Vec::new();
        let mut material_field_changes = Vec::new();
        let mut notes_appended = Vec::new();
        let mut decision_events = Vec::new();
        let mut unresolved_consequential_choices = Vec::new();

        for (id, aggregate) in &aggregates {
            if created_and_deleted.contains(id) {
                continue;
            }

            let surviving_field_changes = aggregate.surviving_field_changes();
            let surviving_decision_events = aggregate.surviving_decision_events();
            let final_unit = aggregate.latest_after.as_ref();
            let unresolved_choices = final_unit
                .map(classify_unresolved_choices)
                .unwrap_or_default();

            let has_surviving_material = aggregate.created_in_turn
                || aggregate.deleted_in_turn
                || !surviving_field_changes.is_empty()
                || !aggregate.notes.is_empty()
                || !surviving_decision_events.is_empty();

            if has_surviving_material {
                let unit_ref = aggregate.display_unit_ref();
                touched_units.push(TurnManaTouchedUnit {
                    unit: unit_ref.clone(),
                    touch_kind: aggregate.touch_kind(),
                    unit_origin: if aggregate.created_in_turn {
                        ManaUnitOrigin::CreatedInTurn
                    } else {
                        ManaUnitOrigin::Preexisting
                    },
                    roles: aggregate.roles.iter().copied().collect(),
                });

                if aggregate.created_in_turn {
                    if let (Some(parent), Some(after)) = (&aggregate.parent_unit, final_unit) {
                        if matches!(after.kind, ManaReviewUnitKind::Epic | ManaReviewUnitKind::Job) {
                            proposed_children.push(TurnManaProposedChild {
                                unit: unit_ref,
                                parent: parent.clone(),
                                child_kind: after.kind,
                                child_origin: ManaUnitOrigin::CreatedInTurn,
                            });
                        }
                    }
                }
            }

            material_field_changes.extend(surviving_field_changes);
            notes_appended.extend(aggregate.notes.clone());
            decision_events.extend(surviving_decision_events);
            unresolved_consequential_choices.extend(unresolved_choices);
        }

        touched_units.sort_by(|a, b| a.unit.id.cmp(&b.unit.id));
        proposed_children.sort_by(|a, b| a.unit.id.cmp(&b.unit.id));
        material_field_changes.sort_by(|a, b| {
            a.unit
                .id
                .cmp(&b.unit.id)
                .then(a.field.cmp(&b.field))
                .then(a.source_action.cmp(&b.source_action))
        });
        notes_appended.sort_by(|a, b| a.unit.id.cmp(&b.unit.id));
        decision_events.sort_by(|a, b| {
            a.unit
                .id
                .cmp(&b.unit.id)
                .then(a.decision_text.cmp(&b.decision_text))
        });
        unresolved_consequential_choices.sort_by(|a, b| {
            a.unit
                .id
                .cmp(&b.unit.id)
                .then(a.decision_text.cmp(&b.decision_text))
        });

        let anchor_unit = derive_anchor_unit(&aggregates, &created_and_deleted, &proposed_children, &touched_units);
        let next_question = unresolved_consequential_choices
            .first()
            .and_then(|choice| choice.suggested_question.clone())
            .or_else(|| {
                unresolved_consequential_choices.first().map(|choice| {
                    format!(
                        "{} · {}",
                        choice.unit.id, choice.decision_text
                    )
                })
            });

        let state = if touched_units.is_empty()
            && proposed_children.is_empty()
            && material_field_changes.is_empty()
            && notes_appended.is_empty()
            && decision_events.is_empty()
            && unresolved_consequential_choices.is_empty()
        {
            ManaReviewState::NoChange
        } else if unresolved_consequential_choices.is_empty() {
            ManaReviewState::Changed
        } else {
            ManaReviewState::NeedsDecision
        };

        TurnManaReview {
            turn_index: self.turn_index,
            state,
            scope,
            anchor_unit,
            touched_units,
            proposed_children,
            material_field_changes,
            notes_appended,
            decision_events,
            unresolved_consequential_choices,
            next_question,
        }
    }
}

#[derive(Debug, Clone)]
struct FieldAggregate {
    unit: ManaUnitRef,
    field: String,
    change_kind: ManaFieldChangeKind,
    before: Option<String>,
    after: Option<String>,
    source_action: String,
}

#[derive(Debug, Clone)]
struct UnitAggregate {
    display_unit: ManaUnitRef,
    first_before: Option<ManaUnitSnapshot>,
    latest_after: Option<ManaUnitSnapshot>,
    parent_unit: Option<ManaUnitRef>,
    created_in_turn: bool,
    deleted_in_turn: bool,
    touch_actions: BTreeSet<ManaTouchKind>,
    roles: BTreeSet<ManaUnitRole>,
    singular_field_changes: BTreeMap<String, FieldAggregate>,
    label_changes: BTreeMap<String, i32>,
    dependency_changes: BTreeMap<String, i32>,
    notes: Vec<TurnManaNoteAppend>,
    decision_added: HashMap<String, usize>,
    decision_resolved: HashMap<String, usize>,
    decision_event_order: Vec<TurnManaDecisionEvent>,
}

impl UnitAggregate {
    fn new(display_unit: ManaUnitRef) -> Self {
        Self {
            display_unit,
            first_before: None,
            latest_after: None,
            parent_unit: None,
            created_in_turn: false,
            deleted_in_turn: false,
            touch_actions: BTreeSet::new(),
            roles: BTreeSet::new(),
            singular_field_changes: BTreeMap::new(),
            label_changes: BTreeMap::new(),
            dependency_changes: BTreeMap::new(),
            notes: Vec::new(),
            decision_added: HashMap::new(),
            decision_resolved: HashMap::new(),
            decision_event_order: Vec::new(),
        }
    }

    fn display_unit_ref(&self) -> ManaUnitRef {
        self.latest_after
            .as_ref()
            .map(ManaUnitSnapshot::unit_ref)
            .unwrap_or_else(|| self.display_unit.clone())
    }

    fn record_field_change(&mut self, change: TurnManaFieldChange) {
        match change.field.as_str() {
            "labels" => {
                if let Some(label) = change.after.clone().or(change.before.clone()) {
                    let delta = match change.change_kind {
                        ManaFieldChangeKind::Added => 1,
                        ManaFieldChangeKind::Removed => -1,
                        _ => 0,
                    };
                    *self.label_changes.entry(label).or_insert(0) += delta;
                }
            }
            "dependencies" => {
                if let Some(dep) = change.after.clone().or(change.before.clone()) {
                    let delta = match change.change_kind {
                        ManaFieldChangeKind::Added => 1,
                        ManaFieldChangeKind::Removed => -1,
                        _ => 0,
                    };
                    *self.dependency_changes.entry(dep).or_insert(0) += delta;
                }
            }
            _ => {
                let key = change.field.clone();
                self.singular_field_changes
                    .entry(key)
                    .and_modify(|entry| {
                        entry.after = change.after.clone();
                        entry.change_kind = change.change_kind;
                        entry.source_action = change.source_action.clone();
                    })
                    .or_insert(FieldAggregate {
                        unit: change.unit,
                        field: change.field,
                        change_kind: change.change_kind,
                        before: change.before,
                        after: change.after,
                        source_action: change.source_action,
                    });
            }
        }
    }

    fn record_decision_event(&mut self, event: TurnManaDecisionEvent) {
        match event.event_kind {
            ManaDecisionEventKind::Added => {
                *self
                    .decision_added
                    .entry(event.decision_text.clone())
                    .or_insert(0) += 1;
            }
            ManaDecisionEventKind::Resolved => {
                *self
                    .decision_resolved
                    .entry(event.decision_text.clone())
                    .or_insert(0) += 1;
            }
        }
        self.decision_event_order.push(event);
    }

    fn surviving_decision_events(&self) -> Vec<TurnManaDecisionEvent> {
        let mut remaining_added = self.decision_added.clone();
        let mut remaining_resolved = self.decision_resolved.clone();

        for (decision, added_count) in &self.decision_added {
            if let Some(resolved_count) = remaining_resolved.get_mut(decision) {
                let cancelled = (*added_count).min(*resolved_count);
                if let Some(added_remaining) = remaining_added.get_mut(decision) {
                    *added_remaining -= cancelled;
                }
                *resolved_count -= cancelled;
            }
        }

        self.decision_event_order
            .iter()
            .filter(|event| match event.event_kind {
                ManaDecisionEventKind::Added => remaining_added
                    .get(&event.decision_text)
                    .copied()
                    .unwrap_or_default()
                    > 0,
                ManaDecisionEventKind::Resolved => remaining_resolved
                    .get(&event.decision_text)
                    .copied()
                    .unwrap_or_default()
                    > 0,
            })
            .cloned()
            .collect()
    }

    fn surviving_field_changes(&self) -> Vec<TurnManaFieldChange> {
        let mut out = Vec::new();

        for aggregate in self.singular_field_changes.values() {
            if aggregate.before == aggregate.after {
                continue;
            }
            out.push(TurnManaFieldChange {
                unit: aggregate.unit.clone(),
                field: aggregate.field.clone(),
                change_kind: aggregate.change_kind,
                before: aggregate.before.clone(),
                after: aggregate.after.clone(),
                source_action: aggregate.source_action.clone(),
            });
        }

        for (label, delta) in &self.label_changes {
            if *delta > 0 {
                out.push(TurnManaFieldChange {
                    unit: self.display_unit_ref(),
                    field: "labels".to_string(),
                    change_kind: ManaFieldChangeKind::Added,
                    before: None,
                    after: Some(label.clone()),
                    source_action: ManaMutationAction::Update.as_str().to_string(),
                });
            } else if *delta < 0 {
                out.push(TurnManaFieldChange {
                    unit: self.display_unit_ref(),
                    field: "labels".to_string(),
                    change_kind: ManaFieldChangeKind::Removed,
                    before: Some(label.clone()),
                    after: None,
                    source_action: ManaMutationAction::Update.as_str().to_string(),
                });
            }
        }

        for (dep, delta) in &self.dependency_changes {
            if *delta > 0 {
                out.push(TurnManaFieldChange {
                    unit: self.display_unit_ref(),
                    field: "dependencies".to_string(),
                    change_kind: ManaFieldChangeKind::Added,
                    before: None,
                    after: Some(dep.clone()),
                    source_action: ManaMutationAction::DepAdd.as_str().to_string(),
                });
            } else if *delta < 0 {
                out.push(TurnManaFieldChange {
                    unit: self.display_unit_ref(),
                    field: "dependencies".to_string(),
                    change_kind: ManaFieldChangeKind::Removed,
                    before: Some(dep.clone()),
                    after: None,
                    source_action: ManaMutationAction::DepRemove.as_str().to_string(),
                });
            }
        }

        out
    }

    fn touch_kind(&self) -> ManaTouchKind {
        if self.deleted_in_turn {
            ManaTouchKind::Deleted
        } else if self.created_in_turn {
            match self.latest_after.as_ref().map(|unit| unit.kind) {
                Some(ManaReviewUnitKind::Fact) => ManaTouchKind::FactCreated,
                _ => ManaTouchKind::Created,
            }
        } else if self.touch_actions.contains(&ManaTouchKind::Failed) {
            ManaTouchKind::Failed
        } else if self.touch_actions.contains(&ManaTouchKind::Reopened) {
            ManaTouchKind::Reopened
        } else if self.touch_actions.contains(&ManaTouchKind::Closed) {
            ManaTouchKind::Closed
        } else {
            ManaTouchKind::Updated
        }
    }
}

fn summarize_scope(mutations: &[ManaMutationRecord]) -> ManaReviewScope {
    let unique: BTreeSet<(ManaReviewScopeKind, String)> = mutations
        .iter()
        .map(|mutation| (mutation.scope.kind.clone(), mutation.scope.display.clone()))
        .collect();

    if unique.is_empty() {
        return ManaReviewScope::default();
    }
    if unique.len() == 1 {
        let (kind, display) = unique.into_iter().next().unwrap();
        return ManaReviewScope { kind, display };
    }

    ManaReviewScope {
        kind: ManaReviewScopeKind::Mixed,
        display: "mixed".to_string(),
    }
}

fn derive_anchor_unit(
    aggregates: &BTreeMap<String, UnitAggregate>,
    created_and_deleted: &BTreeSet<String>,
    proposed_children: &[TurnManaProposedChild],
    touched_units: &[TurnManaTouchedUnit],
) -> Option<TurnManaAnchorUnit> {
    if !proposed_children.is_empty() {
        let mut parent_counts: BTreeMap<String, (&ManaUnitRef, usize)> = BTreeMap::new();
        for child in proposed_children {
            parent_counts
                .entry(child.parent.id.clone())
                .and_modify(|(_, count)| *count += 1)
                .or_insert((&child.parent, 1));
        }
        if let Some((parent_id, (parent_ref, _))) = parent_counts.into_iter().max_by_key(|(_, (_, count))| *count) {
            let created_in_turn = aggregates
                .get(&parent_id)
                .map(|aggregate| aggregate.created_in_turn && !created_and_deleted.contains(&parent_id))
                .unwrap_or(false);
            return Some(TurnManaAnchorUnit {
                unit: parent_ref.clone(),
                anchor_kind: if created_in_turn {
                    ManaAnchorKind::CreatedInTurn
                } else {
                    ManaAnchorKind::ReusedExisting
                },
                reason: if created_in_turn {
                    ManaAnchorReason::CreatedParent
                } else {
                    ManaAnchorReason::AttachedParent
                },
            });
        }
    }

    if touched_units.len() == 1 {
        let touched = touched_units.first().unwrap();
        let reason = if touched.roles.contains(&ManaUnitRole::Fact) {
            ManaAnchorReason::PrimaryFact
        } else {
            ManaAnchorReason::PrimaryTarget
        };
        let anchor_kind = if matches!(touched.unit_origin, ManaUnitOrigin::CreatedInTurn) {
            ManaAnchorKind::CreatedInTurn
        } else {
            ManaAnchorKind::ReusedExisting
        };
        return Some(TurnManaAnchorUnit {
            unit: touched.unit.clone(),
            anchor_kind,
            reason,
        });
    }

    None
}

fn classify_unresolved_choices(unit: &ManaUnitSnapshot) -> Vec<TurnManaConsequentialChoice> {
    unit.decisions
        .iter()
        .filter_map(|decision| classify_consequential_choice(&unit.unit_ref(), decision))
        .collect()
}

fn classify_consequential_choice(
    unit: &ManaUnitRef,
    decision: &str,
) -> Option<TurnManaConsequentialChoice> {
    let lower = decision.to_ascii_lowercase();

    let (category, why) = if contains_any(
        &lower,
        &["mana", "imp", "root", "project", "ownership", "boundary"],
    ) {
        (
            ManaConsequentialChoiceCategory::OwnershipBoundary,
            "changes ownership or storage boundary".to_string(),
        )
    } else if contains_any(
        &lower,
        &["architecture", "design", "contract", "interface", "model"],
    ) {
        (
            ManaConsequentialChoiceCategory::Architecture,
            "changes architecture direction".to_string(),
        )
    } else if contains_any(
        &lower,
        &["launch", "execute", "run", "implement", "start", "ship"],
    ) {
        (
            ManaConsequentialChoiceCategory::ExecutionLaunch,
            "changes whether or how execution should start".to_string(),
        )
    } else if contains_any(
        &lower,
        &["scope", "split", "phase", "defer", "cut"],
    ) {
        (
            ManaConsequentialChoiceCategory::ScopeChange,
            "changes preserved scope or decomposition".to_string(),
        )
    } else if contains_any(
        &lower,
        &["delete", "prune", "remove", "archive"],
    ) {
        (
            ManaConsequentialChoiceCategory::PruneOrDelete,
            "changes whether captured structure should be removed".to_string(),
        )
    } else if decision.trim_end().ends_with('?')
        || lower.starts_with("choose ")
        || lower.starts_with("decide ")
    {
        (
            ManaConsequentialChoiceCategory::Other,
            "is phrased as an unresolved decision".to_string(),
        )
    } else {
        return None;
    };

    Some(TurnManaConsequentialChoice {
        unit: unit.clone(),
        decision_text: decision.to_string(),
        category,
        why_consequential: why,
        suggested_question: Some(format!("{} · {}", unit.id, decision)),
    })
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn _status_string(status: Status) -> String {
    status.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope() -> ManaReviewScope {
        ManaReviewScope {
            kind: ManaReviewScopeKind::Project,
            display: "project".to_string(),
        }
    }

    fn unit(id: &str, title: &str, kind: ManaReviewUnitKind) -> ManaUnitSnapshot {
        ManaUnitSnapshot {
            id: id.to_string(),
            title: title.to_string(),
            kind,
            status: "open".to_string(),
            parent: None,
            dependencies: Vec::new(),
            labels: Vec::new(),
            decisions: Vec::new(),
            description: None,
            acceptance: None,
            design: None,
            assignee: None,
            priority: 2,
            is_archived: false,
        }
    }

    #[test]
    fn no_mutations_becomes_no_change() {
        let mut acc = TurnManaReviewAccumulator::default();
        acc.begin_turn(3);
        let review = acc.finalize();
        assert_eq!(review.state, ManaReviewState::NoChange);
        assert_eq!(review.state.as_str(), "no_change");
    }

    #[test]
    fn create_then_delete_same_unit_is_net_zero() {
        let mut acc = TurnManaReviewAccumulator::default();
        acc.begin_turn(1);
        let created = unit("28.5", "child", ManaReviewUnitKind::Job);
        acc.push(ManaMutationRecord {
            action: ManaMutationAction::Create,
            scope: scope(),
            before_unit: None,
            after_unit: Some(created.clone()),
            deleted_unit: None,
            parent_unit: Some(ManaUnitRef::new("28", "parent", Some("epic".into()))),
            related_unit: None,
            field_changes: Vec::new(),
            notes_appended: Vec::new(),
            decision_events: Vec::new(),
        });
        acc.push(ManaMutationRecord {
            action: ManaMutationAction::Delete,
            scope: scope(),
            before_unit: Some(created.clone()),
            after_unit: None,
            deleted_unit: Some(created.unit_ref()),
            parent_unit: None,
            related_unit: None,
            field_changes: vec![TurnManaFieldChange {
                unit: created.unit_ref(),
                field: "lifecycle.deleted".into(),
                change_kind: ManaFieldChangeKind::Set,
                before: Some("false".into()),
                after: Some("true".into()),
                source_action: "delete".into(),
            }],
            notes_appended: Vec::new(),
            decision_events: Vec::new(),
        });
        let review = acc.finalize();
        assert_eq!(review.state, ManaReviewState::NoChange);
        assert!(review.touched_units.is_empty());
    }

    #[test]
    fn unresolved_architecture_decision_requires_decision() {
        let mut acc = TurnManaReviewAccumulator::default();
        acc.begin_turn(2);
        let mut after = unit("28", "boundary work", ManaReviewUnitKind::Epic);
        after.decisions = vec!["Choose architecture boundary between mana and imp".into()];
        acc.push(ManaMutationRecord {
            action: ManaMutationAction::DecisionAdd,
            scope: scope(),
            before_unit: Some(unit("28", "boundary work", ManaReviewUnitKind::Epic)),
            after_unit: Some(after.clone()),
            deleted_unit: None,
            parent_unit: None,
            related_unit: None,
            field_changes: Vec::new(),
            notes_appended: Vec::new(),
            decision_events: vec![TurnManaDecisionEvent {
                unit: after.unit_ref(),
                event_kind: ManaDecisionEventKind::Added,
                decision_text: "Choose architecture boundary between mana and imp".into(),
                source_action: "decision_add".into(),
            }],
        });
        let review = acc.finalize();
        assert_eq!(review.state, ManaReviewState::NeedsDecision);
        assert_eq!(review.unresolved_consequential_choices.len(), 1);
        assert!(review.next_question.is_some());
    }
}
