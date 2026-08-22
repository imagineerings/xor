use std::{
    collections::hash_map::DefaultHasher,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    error::Error,
    fmt,
    hash::{Hash as _, Hasher as _},
    path::PathBuf,
    time::{Duration, Instant},
};

use agent_ui::ThreadId;
use gpui::{AnyElement, App, Context, Entity, Global, Role, SharedString, Task};
use project::{ProjectGroupKey, WorktreeId};
use ui::{Color, Label, LabelSize, prelude::*};

use crate::collaborative_navigation::{
    CollaborativeNavigationRow, CollaborativeNavigationSourceId,
};

const MAX_AWARENESS_SOURCES: usize = 64;
const MAX_AWARENESS_TARGETS: usize = 4_096;
const MAX_EPHEMERAL_PARTICIPANTS: usize = 64;
const MAX_PRESENCE_TTL: Duration = Duration::from_secs(180);
const MAX_TYPING_TTL: Duration = Duration::from_secs(60);

#[derive(Clone, Eq, Hash, PartialEq)]
pub struct CollaborativeAwarenessTarget(CollaborativeNavigationSourceId);

impl CollaborativeAwarenessTarget {
    pub fn project(project: ProjectGroupKey) -> Self {
        Self(CollaborativeNavigationSourceId::Project(project))
    }

    pub fn worktree(project: ProjectGroupKey, worktree_id: WorktreeId, path: PathBuf) -> Self {
        Self(CollaborativeNavigationSourceId::Worktree {
            project,
            worktree_id,
            path,
        })
    }

    pub fn repository(project: ProjectGroupKey, work_directory: PathBuf) -> Self {
        Self(CollaborativeNavigationSourceId::Repository {
            project,
            work_directory,
        })
    }

    pub const fn channel(channel_id: u64) -> Self {
        Self(CollaborativeNavigationSourceId::Channel(channel_id))
    }

    pub const fn thread(thread_id: ThreadId) -> Self {
        Self(CollaborativeNavigationSourceId::Thread(thread_id))
    }

    fn from_row(row: &CollaborativeNavigationRow) -> Self {
        Self(row.source_id().clone())
    }
}

impl fmt::Debug for CollaborativeAwarenessTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CollaborativeAwarenessTarget(<redacted>)")
    }
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AwarenessParticipantId(u128);

impl AwarenessParticipantId {
    pub const fn from_u128(value: u128) -> Self {
        Self(value)
    }
}

impl fmt::Debug for AwarenessParticipantId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("AwarenessParticipantId(<redacted>)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AwarenessPresenceStatus {
    Online,
    Away,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AwarenessPresence {
    pub participant_id: AwarenessParticipantId,
    pub status: AwarenessPresenceStatus,
    pub expires_at: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AwarenessTyping {
    pub participant_id: AwarenessParticipantId,
    pub expires_at: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AwarenessReminder {
    None,
    Scheduled { due_at: Instant },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DurableAwarenessUpdate {
    pub revision: u64,
    pub unread_count: u32,
    pub manually_unread: bool,
    pub reminder: AwarenessReminder,
}

#[derive(Clone, Eq, PartialEq)]
pub struct CollaborativeAwarenessUpdate {
    pub target: CollaborativeAwarenessTarget,
    pub sequence: u64,
    pub durable: Option<DurableAwarenessUpdate>,
    pub presence: Vec<AwarenessPresence>,
    pub typing: Vec<AwarenessTyping>,
}

impl fmt::Debug for CollaborativeAwarenessUpdate {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CollaborativeAwarenessUpdate")
            .field("target", &self.target)
            .field("sequence", &self.sequence)
            .field("has_durable_state", &self.durable.is_some())
            .field("presence_count", &self.presence.len())
            .field("typing_count", &self.typing.len())
            .finish()
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct CollaborativeAwarenessConnectionToken {
    source_id: u128,
    generation: u64,
}

impl fmt::Debug for CollaborativeAwarenessConnectionToken {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("CollaborativeAwarenessConnectionToken(<redacted>)")
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeAwarenessUpdateOutcome {
    Applied,
    Duplicate,
    IgnoredStale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeAwarenessDisconnectOutcome {
    Disconnected,
    Stale,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeAwarenessFreshness {
    Fresh,
    Recovering,
    Stale,
    Offline,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollaborativeAwarenessPresentation {
    pub unread_count: u32,
    pub manually_unread: bool,
    pub reminder_due: bool,
    pub reminder_scheduled: bool,
    pub online_count: usize,
    pub away_count: usize,
    pub typing_count: usize,
    pub freshness: CollaborativeAwarenessFreshness,
    pub last_trustworthy_age: Duration,
}

impl CollaborativeAwarenessPresentation {
    fn badges(&self) -> Vec<AwarenessBadge> {
        let mut badges = Vec::new();
        if self.unread_count > 0 {
            let unread: SharedString = if self.unread_count > 99 {
                "99+ unread".into()
            } else {
                format!("{} unread", self.unread_count).into()
            };
            badges.push(AwarenessBadge::new(unread, Color::Accent));
        } else if self.manually_unread {
            badges.push(AwarenessBadge::new("Unread", Color::Accent));
        }
        if self.reminder_due {
            badges.push(AwarenessBadge::new("Reminder due", Color::Warning));
        } else if self.reminder_scheduled {
            badges.push(AwarenessBadge::new("Reminder", Color::Muted));
        }
        if self.online_count > 0 {
            badges.push(AwarenessBadge::new(
                format!("{} online", self.online_count),
                Color::Muted,
            ));
        }
        if self.away_count > 0 {
            badges.push(AwarenessBadge::new(
                format!("{} away", self.away_count),
                Color::Muted,
            ));
        }
        if self.typing_count > 0 {
            badges.push(AwarenessBadge::new(
                format!("{} typing", self.typing_count),
                Color::Accent,
            ));
        }
        match self.freshness {
            CollaborativeAwarenessFreshness::Fresh => {}
            CollaborativeAwarenessFreshness::Recovering => {
                badges.push(AwarenessBadge::new("Reconnecting…", Color::Warning))
            }
            CollaborativeAwarenessFreshness::Stale => {
                badges.push(AwarenessBadge::new("Stale · retrying", Color::Warning))
            }
            CollaborativeAwarenessFreshness::Offline => {
                badges.push(AwarenessBadge::new("Offline · retrying", Color::Error))
            }
        }
        badges
    }

    fn accessibility_label(&self) -> SharedString {
        let mut labels = self
            .badges()
            .into_iter()
            .map(|badge| badge.label.to_string())
            .collect::<Vec<_>>();
        let age_seconds = self.last_trustworthy_age.as_secs();
        let age_unit = if age_seconds == 1 {
            "second"
        } else {
            "seconds"
        };
        labels.push(format!(
            "last trustworthy update {age_seconds} {age_unit} ago"
        ));
        labels.join(", ").into()
    }
}

#[derive(Clone)]
struct AwarenessBadge {
    label: SharedString,
    color: Color,
}

impl AwarenessBadge {
    fn new(label: impl Into<SharedString>, color: Color) -> Self {
        Self {
            label: label.into(),
            color,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceStatus {
    Healthy,
    Partial,
    Recovering,
    Offline,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct EphemeralAwareness {
    presence: Vec<AwarenessPresence>,
    typing: Vec<AwarenessTyping>,
}

struct SourceAwareness {
    generation: u64,
    status: SourceStatus,
    last_sequence: u64,
    last_update: Option<CollaborativeAwarenessUpdate>,
    rows: HashMap<CollaborativeAwarenessTarget, EphemeralAwareness>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct StoredDurableAwareness {
    state: DurableAwarenessUpdate,
}

struct TargetObservation {
    sources: HashSet<u128>,
    last_trustworthy_at: Instant,
}

struct GlobalCollaborativeAwarenessStore(Entity<CollaborativeAwarenessStore>);

impl Global for GlobalCollaborativeAwarenessStore {}

pub struct CollaborativeAwarenessStore {
    sources: HashMap<u128, SourceAwareness>,
    durable_rows: HashMap<CollaborativeAwarenessTarget, StoredDurableAwareness>,
    observations: HashMap<CollaborativeAwarenessTarget, TargetObservation>,
    expiration_task: Task<()>,
}

impl CollaborativeAwarenessStore {
    pub fn init(cx: &mut App) -> Entity<Self> {
        if let Some(store) = Self::try_global(cx) {
            return store;
        }
        let store = cx.new(|_| Self {
            sources: HashMap::new(),
            durable_rows: HashMap::new(),
            observations: HashMap::new(),
            expiration_task: Task::ready(()),
        });
        cx.set_global(GlobalCollaborativeAwarenessStore(store.clone()));
        store
    }

    pub fn try_global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalCollaborativeAwarenessStore>()
            .map(|global| global.0.clone())
    }

    pub fn connect(
        &mut self,
        source_id: u128,
        generation: u64,
        now: Instant,
        cx: &mut Context<Self>,
    ) -> Result<CollaborativeAwarenessConnectionToken, CollaborativeAwarenessError> {
        if source_id == 0 || generation == 0 {
            return Err(CollaborativeAwarenessError::InvalidUpdate);
        }
        self.prune(now);
        if let Some(source) = self.sources.get_mut(&source_id) {
            if generation < source.generation
                || (generation == source.generation && source.status == SourceStatus::Offline)
            {
                return Err(CollaborativeAwarenessError::StaleConnection);
            }
            if generation > source.generation {
                source.generation = generation;
                source.status = SourceStatus::Recovering;
                source.last_sequence = 0;
                source.last_update = None;
                source.rows.clear();
                self.schedule_expiration(now, cx);
                cx.notify();
            }
        } else {
            if self.sources.len() >= MAX_AWARENESS_SOURCES {
                return Err(CollaborativeAwarenessError::CapacityExceeded);
            }
            self.sources.insert(
                source_id,
                SourceAwareness {
                    generation,
                    status: SourceStatus::Recovering,
                    last_sequence: 0,
                    last_update: None,
                    rows: HashMap::new(),
                },
            );
            cx.notify();
        }
        Ok(CollaborativeAwarenessConnectionToken {
            source_id,
            generation,
        })
    }

    pub fn apply_update(
        &mut self,
        token: CollaborativeAwarenessConnectionToken,
        update: CollaborativeAwarenessUpdate,
        now: Instant,
        cx: &mut Context<Self>,
    ) -> Result<CollaborativeAwarenessUpdateOutcome, CollaborativeAwarenessError> {
        validate_update(&update, now)?;
        self.prune(now);
        let source = self
            .sources
            .get(&token.source_id)
            .ok_or(CollaborativeAwarenessError::StaleConnection)?;
        if source.generation != token.generation || source.status == SourceStatus::Offline {
            return Err(CollaborativeAwarenessError::StaleConnection);
        }
        if update.sequence < source.last_sequence {
            return Ok(CollaborativeAwarenessUpdateOutcome::IgnoredStale);
        }
        if update.sequence == source.last_sequence {
            return if source.last_update.as_ref() == Some(&update) {
                Ok(CollaborativeAwarenessUpdateOutcome::Duplicate)
            } else {
                Err(CollaborativeAwarenessError::SequenceConflict)
            };
        }
        if !self.observations.contains_key(&update.target)
            && self.observations.len() >= MAX_AWARENESS_TARGETS
        {
            return Err(CollaborativeAwarenessError::CapacityExceeded);
        }
        if let Some(candidate) = update.durable
            && let Some(current) = self.durable_rows.get(&update.target)
            && candidate.revision == current.state.revision
            && candidate != current.state
        {
            return Err(CollaborativeAwarenessError::RevisionConflict);
        }

        if let Some(candidate) = update.durable {
            let replace = self
                .durable_rows
                .get(&update.target)
                .is_none_or(|current| candidate.revision > current.state.revision);
            if replace {
                self.durable_rows.insert(
                    update.target.clone(),
                    StoredDurableAwareness { state: candidate },
                );
            }
        }
        self.observations
            .entry(update.target.clone())
            .and_modify(|observation| {
                observation.sources.insert(token.source_id);
                observation.last_trustworthy_at = now;
            })
            .or_insert_with(|| TargetObservation {
                sources: HashSet::from([token.source_id]),
                last_trustworthy_at: now,
            });
        let source = self
            .sources
            .get_mut(&token.source_id)
            .ok_or(CollaborativeAwarenessError::StaleConnection)?;
        source.status = SourceStatus::Healthy;
        source.last_sequence = update.sequence;
        source.rows.insert(
            update.target.clone(),
            EphemeralAwareness {
                presence: update.presence.clone(),
                typing: update.typing.clone(),
            },
        );
        source.last_update = Some(update);
        self.schedule_expiration(now, cx);
        cx.notify();
        Ok(CollaborativeAwarenessUpdateOutcome::Applied)
    }

    pub fn mark_partial(
        &mut self,
        token: CollaborativeAwarenessConnectionToken,
        cx: &mut Context<Self>,
    ) -> Result<(), CollaborativeAwarenessError> {
        let source = self
            .sources
            .get_mut(&token.source_id)
            .ok_or(CollaborativeAwarenessError::StaleConnection)?;
        if source.generation != token.generation || source.status == SourceStatus::Offline {
            return Err(CollaborativeAwarenessError::StaleConnection);
        }
        if source.status != SourceStatus::Partial {
            source.status = SourceStatus::Partial;
            cx.notify();
        }
        Ok(())
    }

    pub fn disconnect(
        &mut self,
        token: CollaborativeAwarenessConnectionToken,
        now: Instant,
        cx: &mut Context<Self>,
    ) -> CollaborativeAwarenessDisconnectOutcome {
        self.prune(now);
        let Some(source) = self.sources.get_mut(&token.source_id) else {
            return CollaborativeAwarenessDisconnectOutcome::Stale;
        };
        if source.generation != token.generation || source.status == SourceStatus::Offline {
            return CollaborativeAwarenessDisconnectOutcome::Stale;
        }
        source.status = SourceStatus::Offline;
        source.rows.clear();
        self.schedule_expiration(now, cx);
        cx.notify();
        CollaborativeAwarenessDisconnectOutcome::Disconnected
    }

    pub fn presentation(
        &self,
        target: &CollaborativeAwarenessTarget,
        now: Instant,
    ) -> Option<CollaborativeAwarenessPresentation> {
        let observation = self.observations.get(target)?;
        let durable = self.durable_rows.get(target).map(|stored| stored.state);
        let mut participants = BTreeMap::<AwarenessParticipantId, AwarenessPresenceStatus>::new();
        let mut typing = BTreeSet::<AwarenessParticipantId>::new();
        let mut statuses = Vec::new();
        for source_id in &observation.sources {
            let Some(source) = self.sources.get(source_id) else {
                continue;
            };
            statuses.push(source.status);
            if source.status == SourceStatus::Offline {
                continue;
            }
            let Some(row) = source.rows.get(target) else {
                continue;
            };
            for presence in &row.presence {
                if now >= presence.expires_at {
                    continue;
                }
                participants
                    .entry(presence.participant_id)
                    .and_modify(|status| {
                        if presence.status == AwarenessPresenceStatus::Online {
                            *status = AwarenessPresenceStatus::Online;
                        }
                    })
                    .or_insert(presence.status);
            }
            typing.extend(
                row.typing
                    .iter()
                    .filter(|typing| now < typing.expires_at)
                    .map(|typing| typing.participant_id),
            );
        }
        let freshness = if statuses.contains(&SourceStatus::Healthy) {
            CollaborativeAwarenessFreshness::Fresh
        } else if statuses.contains(&SourceStatus::Partial) {
            CollaborativeAwarenessFreshness::Stale
        } else if statuses.contains(&SourceStatus::Recovering) {
            CollaborativeAwarenessFreshness::Recovering
        } else {
            CollaborativeAwarenessFreshness::Offline
        };
        let reminder = durable.map_or(AwarenessReminder::None, |state| state.reminder);
        let reminder_due =
            matches!(reminder, AwarenessReminder::Scheduled { due_at } if now >= due_at);
        Some(CollaborativeAwarenessPresentation {
            unread_count: durable.map_or(0, |state| state.unread_count),
            manually_unread: durable.is_some_and(|state| state.manually_unread),
            reminder_due,
            reminder_scheduled: reminder != AwarenessReminder::None,
            online_count: participants
                .values()
                .filter(|status| **status == AwarenessPresenceStatus::Online)
                .count(),
            away_count: participants
                .values()
                .filter(|status| **status == AwarenessPresenceStatus::Away)
                .count(),
            typing_count: typing.len(),
            freshness,
            last_trustworthy_age: now.saturating_duration_since(observation.last_trustworthy_at),
        })
    }

    fn prune(&mut self, now: Instant) {
        for source in self.sources.values_mut() {
            for row in source.rows.values_mut() {
                row.presence.retain(|presence| now < presence.expires_at);
                row.typing.retain(|typing| now < typing.expires_at);
            }
        }
    }

    fn next_deadline(&self, now: Instant) -> Option<Instant> {
        self.sources
            .values()
            .flat_map(|source| source.rows.values())
            .flat_map(|row| {
                row.presence
                    .iter()
                    .map(|presence| presence.expires_at)
                    .chain(row.typing.iter().map(|typing| typing.expires_at))
            })
            .chain(self.durable_rows.values().filter_map(|stored| {
                let AwarenessReminder::Scheduled { due_at } = stored.state.reminder else {
                    return None;
                };
                Some(due_at)
            }))
            .filter(|deadline| *deadline > now)
            .min()
    }

    fn schedule_expiration(&mut self, now: Instant, cx: &mut Context<Self>) {
        self.expiration_task = self.next_deadline(now).map_or_else(
            || Task::ready(()),
            |deadline| {
                let delay = deadline.saturating_duration_since(now);
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(delay).await;
                    if let Err(error) = this.update(cx, |this, cx| {
                        let now = cx.background_executor().now();
                        this.prune(now);
                        this.schedule_expiration(now, cx);
                        cx.notify();
                    }) {
                        log::error!("failed to expire collaborative awareness: {error:#}");
                    }
                })
            },
        );
    }
}

fn validate_update(
    update: &CollaborativeAwarenessUpdate,
    now: Instant,
) -> Result<(), CollaborativeAwarenessError> {
    if update.sequence == 0
        || (update.durable.is_none() && update.presence.is_empty() && update.typing.is_empty())
        || update.durable.is_some_and(|durable| durable.revision == 0)
        || update.presence.len() > MAX_EPHEMERAL_PARTICIPANTS
        || update.typing.len() > MAX_EPHEMERAL_PARTICIPANTS
    {
        return Err(CollaborativeAwarenessError::InvalidUpdate);
    }
    let max_presence_expiry = now
        .checked_add(MAX_PRESENCE_TTL)
        .ok_or(CollaborativeAwarenessError::InvalidUpdate)?;
    let max_typing_expiry = now
        .checked_add(MAX_TYPING_TTL)
        .ok_or(CollaborativeAwarenessError::InvalidUpdate)?;
    let mut presence_ids = HashSet::new();
    for presence in &update.presence {
        if !presence_ids.insert(presence.participant_id)
            || presence.expires_at <= now
            || presence.expires_at > max_presence_expiry
        {
            return Err(CollaborativeAwarenessError::InvalidUpdate);
        }
    }
    let mut typing_ids = HashSet::new();
    for typing in &update.typing {
        if !typing_ids.insert(typing.participant_id)
            || typing.expires_at <= now
            || typing.expires_at > max_typing_expiry
        {
            return Err(CollaborativeAwarenessError::InvalidUpdate);
        }
    }
    Ok(())
}

pub(crate) fn observe_collaborative_awareness<T: 'static>(cx: &mut Context<T>) {
    let store = CollaborativeAwarenessStore::init(cx);
    cx.observe(&store, |_, _, cx| cx.notify()).detach();
}

pub(crate) fn render_collaborative_awareness(
    row: &CollaborativeNavigationRow,
    cx: &App,
) -> Option<AnyElement> {
    let store = CollaborativeAwarenessStore::try_global(cx)?;
    let target = CollaborativeAwarenessTarget::from_row(row);
    let presentation = store
        .read(cx)
        .presentation(&target, cx.background_executor().now())?;
    let mut element_id = DefaultHasher::new();
    target.hash(&mut element_id);
    let element_id = element_id.finish();
    let accessibility_label = presentation.accessibility_label();
    let badges = presentation.badges();
    if badges.is_empty() {
        return None;
    }
    Some(
        h_flex()
            .id(("collaborative-awareness", element_id))
            .gap_1()
            .role(Role::Status)
            .aria_label(accessibility_label)
            .children(badges.into_iter().map(|badge| {
                Label::new(badge.label)
                    .size(LabelSize::XSmall)
                    .color(badge.color)
            }))
            .into_any_element(),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeAwarenessError {
    InvalidUpdate,
    StaleConnection,
    SequenceConflict,
    RevisionConflict,
    CapacityExceeded,
}

impl fmt::Display for CollaborativeAwarenessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUpdate => formatter.write_str("collaborative awareness update is invalid"),
            Self::StaleConnection => {
                formatter.write_str("collaborative awareness connection is stale")
            }
            Self::SequenceConflict => {
                formatter.write_str("collaborative awareness sequence has conflicting content")
            }
            Self::RevisionConflict => {
                formatter.write_str("collaborative awareness revision has conflicting content")
            }
            Self::CapacityExceeded => {
                formatter.write_str("collaborative awareness capacity was exceeded")
            }
        }
    }
}

impl Error for CollaborativeAwarenessError {}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;

    fn update(
        target: CollaborativeAwarenessTarget,
        sequence: u64,
        revision: u64,
        unread_count: u32,
        participant_id: u128,
        now: Instant,
    ) -> CollaborativeAwarenessUpdate {
        CollaborativeAwarenessUpdate {
            target,
            sequence,
            durable: Some(DurableAwarenessUpdate {
                revision,
                unread_count,
                manually_unread: false,
                reminder: AwarenessReminder::Scheduled {
                    due_at: now + Duration::from_secs(30),
                },
            }),
            presence: vec![AwarenessPresence {
                participant_id: AwarenessParticipantId::from_u128(participant_id),
                status: AwarenessPresenceStatus::Online,
                expires_at: now + Duration::from_secs(10),
            }],
            typing: vec![AwarenessTyping {
                participant_id: AwarenessParticipantId::from_u128(participant_id),
                expires_at: now + Duration::from_secs(5),
            }],
        }
    }

    #[gpui::test]
    fn collaborative_awareness_merges_multi_device_updates(cx: &mut TestAppContext) {
        let store = cx.new(|_| CollaborativeAwarenessStore {
            sources: HashMap::new(),
            durable_rows: HashMap::new(),
            observations: HashMap::new(),
            expiration_task: Task::ready(()),
        });
        let now = cx.executor().now();
        let target = CollaborativeAwarenessTarget::channel(42);
        let first = store
            .update(cx, |store, cx| store.connect(1, 1, now, cx))
            .expect("connect first device");
        let second = store
            .update(cx, |store, cx| store.connect(2, 1, now, cx))
            .expect("connect second device");
        store
            .update(cx, |store, cx| {
                store.apply_update(first, update(target.clone(), 1, 1, 3, 10, now), now, cx)
            })
            .expect("apply first device");
        store
            .update(cx, |store, cx| {
                store.apply_update(second, update(target.clone(), 1, 2, 7, 11, now), now, cx)
            })
            .expect("apply second device");

        let presentation = store
            .read_with(cx, |store, _| store.presentation(&target, now))
            .expect("awareness presentation");
        assert_eq!(presentation.unread_count, 7);
        assert_eq!(presentation.online_count, 2);
        assert_eq!(presentation.typing_count, 2);
        assert_eq!(
            presentation
                .badges()
                .into_iter()
                .map(|badge| badge.label)
                .collect::<Vec<_>>(),
            vec![
                SharedString::from("7 unread"),
                SharedString::from("Reminder"),
                SharedString::from("2 online"),
                SharedString::from("2 typing"),
            ]
        );
        assert_eq!(
            presentation.freshness,
            CollaborativeAwarenessFreshness::Fresh
        );
    }

    #[gpui::test]
    fn collaborative_awareness_retains_last_trustworthy_state_offline(cx: &mut TestAppContext) {
        let store = cx.new(|_| CollaborativeAwarenessStore {
            sources: HashMap::new(),
            durable_rows: HashMap::new(),
            observations: HashMap::new(),
            expiration_task: Task::ready(()),
        });
        let now = cx.executor().now();
        let target = CollaborativeAwarenessTarget::channel(42);
        let token = store
            .update(cx, |store, cx| store.connect(1, 1, now, cx))
            .expect("connect device");
        store
            .update(cx, |store, cx| {
                store.apply_update(token, update(target.clone(), 1, 1, 4, 10, now), now, cx)
            })
            .expect("apply update");
        store
            .update(cx, |store, cx| store.mark_partial(token, cx))
            .expect("mark partial");
        assert_eq!(
            store
                .read_with(cx, |store, _| store.presentation(&target, now))
                .expect("stale presentation")
                .freshness,
            CollaborativeAwarenessFreshness::Stale
        );
        assert_eq!(
            store.update(cx, |store, cx| store.disconnect(token, now, cx)),
            CollaborativeAwarenessDisconnectOutcome::Disconnected
        );

        let presentation = store
            .read_with(cx, |store, _| {
                store.presentation(&target, now + Duration::from_secs(1))
            })
            .expect("offline presentation");
        assert_eq!(presentation.unread_count, 4);
        assert_eq!(presentation.online_count, 0);
        assert_eq!(presentation.typing_count, 0);
        assert_eq!(
            presentation.freshness,
            CollaborativeAwarenessFreshness::Offline
        );
        assert_eq!(presentation.last_trustworthy_age, Duration::from_secs(1));
        assert_eq!(
            presentation.accessibility_label(),
            "4 unread, Reminder, Offline · retrying, last trustworthy update 1 second ago"
        );
    }

    #[gpui::test]
    fn collaborative_awareness_reconnect_fences_stale_generations(cx: &mut TestAppContext) {
        let store = cx.new(|_| CollaborativeAwarenessStore {
            sources: HashMap::new(),
            durable_rows: HashMap::new(),
            observations: HashMap::new(),
            expiration_task: Task::ready(()),
        });
        let now = cx.executor().now();
        let target = CollaborativeAwarenessTarget::channel(42);
        let first = store
            .update(cx, |store, cx| store.connect(1, 1, now, cx))
            .expect("connect first generation");
        store
            .update(cx, |store, cx| {
                store.apply_update(first, update(target.clone(), 1, 1, 2, 10, now), now, cx)
            })
            .expect("apply first generation");
        let second = store
            .update(cx, |store, cx| {
                store.connect(1, 2, now + Duration::from_secs(1), cx)
            })
            .expect("connect second generation");
        assert_eq!(
            store
                .read_with(cx, |store, _| {
                    store.presentation(&target, now + Duration::from_secs(1))
                })
                .expect("recovering presentation")
                .freshness,
            CollaborativeAwarenessFreshness::Recovering
        );
        assert_eq!(
            store.update(cx, |store, cx| {
                store.apply_update(
                    first,
                    update(target.clone(), 2, 2, 9, 11, now + Duration::from_secs(1)),
                    now + Duration::from_secs(1),
                    cx,
                )
            }),
            Err(CollaborativeAwarenessError::StaleConnection)
        );
        store
            .update(cx, |store, cx| {
                store.apply_update(
                    second,
                    update(target.clone(), 1, 2, 9, 11, now + Duration::from_secs(1)),
                    now + Duration::from_secs(1),
                    cx,
                )
            })
            .expect("apply current generation");
        assert_eq!(
            store
                .read_with(cx, |store, _| {
                    store.presentation(&target, now + Duration::from_secs(1))
                })
                .expect("fresh presentation")
                .unread_count,
            9
        );
    }

    #[gpui::test]
    fn collaborative_awareness_expires_with_gpui_time(cx: &mut TestAppContext) {
        let store = cx.new(|_| CollaborativeAwarenessStore {
            sources: HashMap::new(),
            durable_rows: HashMap::new(),
            observations: HashMap::new(),
            expiration_task: Task::ready(()),
        });
        let now = cx.executor().now();
        let target = CollaborativeAwarenessTarget::channel(42);
        let token = store
            .update(cx, |store, cx| store.connect(1, 1, now, cx))
            .expect("connect device");
        let expiring = CollaborativeAwarenessUpdate {
            target: target.clone(),
            sequence: 1,
            durable: Some(DurableAwarenessUpdate {
                revision: 1,
                unread_count: 1,
                manually_unread: false,
                reminder: AwarenessReminder::Scheduled {
                    due_at: now + Duration::from_secs(3),
                },
            }),
            presence: vec![AwarenessPresence {
                participant_id: AwarenessParticipantId::from_u128(10),
                status: AwarenessPresenceStatus::Online,
                expires_at: now + Duration::from_secs(2),
            }],
            typing: vec![AwarenessTyping {
                participant_id: AwarenessParticipantId::from_u128(10),
                expires_at: now + Duration::from_secs(1),
            }],
        };
        store
            .update(cx, |store, cx| store.apply_update(token, expiring, now, cx))
            .expect("apply expiring update");

        cx.executor().advance_clock(Duration::from_secs(1));
        cx.run_until_parked();
        let after_typing = store
            .read_with(cx, |store, _| {
                store.presentation(&target, now + Duration::from_secs(1))
            })
            .expect("typing expiry presentation");
        assert_eq!(after_typing.typing_count, 0);
        assert_eq!(after_typing.online_count, 1);

        cx.executor().advance_clock(Duration::from_secs(1));
        cx.run_until_parked();
        let after_presence = store
            .read_with(cx, |store, _| {
                store.presentation(&target, now + Duration::from_secs(2))
            })
            .expect("presence expiry presentation");
        assert_eq!(after_presence.online_count, 0);
        assert!(!after_presence.reminder_due);

        cx.executor().advance_clock(Duration::from_secs(1));
        cx.run_until_parked();
        let reminder_due = store
            .read_with(cx, |store, _| {
                store.presentation(&target, now + Duration::from_secs(3))
            })
            .expect("reminder due presentation");
        assert!(reminder_due.reminder_due);
        assert_eq!(
            reminder_due
                .badges()
                .into_iter()
                .map(|badge| badge.label)
                .collect::<Vec<_>>(),
            vec![
                SharedString::from("1 unread"),
                SharedString::from("Reminder due"),
            ]
        );
    }
}
