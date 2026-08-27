# Collaborative Workspace Reuse and Ownership Matrix

## Classification

- **Native**: the Collaborative surface directly hosts an existing Zed entity or action.
- **Adapter**: a registration, routing, identity-check, focus or selection adapter that retains canonical handles but no domain rows.
- **View**: ephemeral render props or view-local layout, focus, scroll, disclosure or optimistic reconciliation state.
- **New**: canonical functionality for which Standard Zed has no current owner.
- **Removed**: duplicate ownership or behavior deleted by the reuse audit.

Rows group only types that have the same capability, owner, lifecycle, registration
and proof. Every Collaborative Workspace type in the audited paths is named in a
row. Test-only helper types elsewhere in `visual_test_runner.rs` are outside the
Collaborative Workspace implementation and are not classified as product owners.

## Workspace composition

| Types | Capability | Authoritative owner / reused native behavior | Class | Production registration | Canonical-owner proof |
| --- | --- | --- | --- | --- | --- |
| `CollaborativeWorkspace` | Collaborative layout composition | The existing workspace `Project`, timeline/composer/review `AnyView` entities and native status/title bars | Adapter | `Workspace::new` creates one presentation entity behind `multiplayer-tools` | workspace switch/restart and cross-surface tests assert the same `Project` entity |
| `CollaborativeLayout`, `CollaborativeLayoutGeometry`, `DraggedCollaborativeReview` | Rail/timeline/review geometry and drag state | GPUI layout and native review view | View | owned by `CollaborativeWorkspace` | reference, narrow and wide geometry tests |
| `CollaborativeLayoutState` | Width, collapse and selected review presentation | Workspace KVP/settings persistence | View | loaded/saved by the existing `WorkspaceId` KVP path | restart and geometry tests |
| `CollaborativeComposerProvider`, `CollaborativeComposerRegistration`, `CollaborativeComposerHost`, `CollaborativeComposerSurface`, `CollaborativeComposerRegistrationError`, `CollaborativeComposerActionError`, `ComposerAction` | Route one selected native composer and its actions | ACP `MessageEditor` or channel `ChannelMessageComposer` | Adapter | agent/channel adapters register in `Workspace`; generation token unregisters stale selection | composer GPUI test submits/cancels the native ACP editor; channel selection tests replace only the visible provider |
| `CollaborativeTimelineProvider`, `CollaborativeTimelineRegistration`, `CollaborativeTimelineHost`, `CollaborativeTimelineRegistrationError` | Route one selected native timeline | `ThreadView` entry list or `MessageTimeline` | Adapter | agent/channel adapter registration in `Workspace` | timeline test compares registered entity IDs and proves unregister/switch drops the old view |
| `CollaborativeReviewSlot`, `CollaborativeReviewRegistration`, `CollaborativeReviewHost`, `CollaborativeReviewRegistrationError`, `CollaborativeReviewSelectionError` | Route selected native agent/project diff | `AgentDiffPane` and `ProjectDiff` | Adapter | adapters register an `AnyView` by canonical `Project` identity | review tests compare exact native entity IDs and stale registrations fail closed |
| `CollaborativeReviewSummarySource` | Stale-action token for generic Git activity | Native provider entity ID plus its externally supplied revision | Adapter | supplied only with a native action callback | action-routing tests reject provider/revision mismatch; it contains no files, hunks or Git state |
| `CollaborativeReviewAction`, `CollaborativeReviewActionState`, `CollaborativeReviewActionContext`, `CollaborativeReviewActionRequest`, `ExecutedCollaborativeReviewAction`, `CollaborativeReviewActionError` | Validate generic activity-card action routing | Native Git/agent callback is the only mutator | Adapter | `collab_ui::GitActivityCard` emits and invokes native callbacks | native Git regression proves mutation occurs in repository state; stale/conflict routes do not invoke callback |
| `CollaborativeNavigation`, `CollaborativeNavigationTarget`, `CollaborativeProjectTarget`, `CollaborativeRemoteTarget`, `PersistedCollaborativeNavigation`, `CollaborativeNavigationError`, `CollaborativeEntityLinkError` | Collaborative-only selection/back-forward relationships | Native `Workspace` open/focus actions; KVP stores only target/history presentation | View | `Workspace` owns one navigation value | navigation tests prove actions open canonical project/thread/channel owners and restart restores only target identity |
| `CollaborativeFocusRegion`, `CollaborativeFocusOrder` | Cross-region focus order | Native `FocusHandle`s from rail, `ThreadView` composer, review and `StatusBar` | View | assembled by `CollaborativeWorkspace::focus_region_handles` | keyboard-focus tests compare the active native composer focus handle |
| `CollaborativeAccessibilityContract`, `CollaborativeAnnouncement`, `CollaborativeAnnouncementRole` | Labels and announcements | Existing provider state and GPUI roles | View | rendered by native surfaces | accessibility tests cover labels and failure announcements |
| `CollaborativeShellState`, `CollaborativeShellStatus`, `CollaborativeShellScope`, `CollaborativeShellPhase`, `CollaborativeShellRetryRequested` | View-local loading/partial-failure disclosure | Native provider availability; no domain data | View | owned by `CollaborativeWorkspace` | shell-state retry and failure tests |
| `CollaborativeTopBar`, `CollaborativeTopBarActionAvailability`, `CollaborativeTopBarTestSnapshot` | Project title, participants and native layout/share actions | `Project`, live participant reader, native title-bar/call actions and layout entity | Adapter | composed by `title_bar::TitleBar` | top-bar tests observe project/provider updates and native avatar branch |

## Participants, presence and status

| Types | Capability | Authoritative owner / reused native behavior | Class | Production registration | Canonical-owner proof |
| --- | --- | --- | --- | --- | --- |
| `CollaborativeParticipantProvider`, `CollaborativeParticipantProviderState`, `CollaborativeParticipantRegistration`, `CollaborativeParticipantHost`, `ParticipantStateReader`, `CollaborativeParticipantProviderError` | Route a live participant projection | Reader resolves active `ThreadView`, `ThreadMetadataStore`, `ActiveCall` room and native users on demand | Adapter | Zed composition registers once per active thread and only notifies on canonical updates | provider test changes its canonical reader without replacing copied rows; selection unregisters stale reader |
| `CollaborativeParticipant`, `CollaborativeParticipantIdentity`, `CollaborativeParticipantPresence`, `CollaborativeParticipantViewData`, `CollaborativeConnectionState`, `CollaborativeExecutionStatus`, `CollaborativeExecutionPhase`, `CollaborativeExecutionLocation` | Transient avatar/presence/execution props | `client::User`, room participant, ACP agent/thread/model and metadata | View | produced on each provider read; never persisted | top-bar/status tests observe authoritative user/thread changes |
| `CollaborativeParticipantAdapter`, `CollaborativeParticipantAdapterError`, `RoomStateReader` | Translate active native agent/call metadata | Weak active `ThreadView`, `ThreadMetadataStore`, `ActiveCall` | Adapter | reconciled from the active `AgentPanel` | entity identity and live-reader ownership tests |
| `CollaborativeParticipantStatus` | ACP-specific status-bar addition | live participant provider; native status bar continues owning project/Git/branch items | View | rendered as an extra native status item | status tests cover live execution/failure/unavailable states |
| `CollaborativeAwarenessStore`, `GlobalCollaborativeAwarenessStore`, `CollaborativeAwarenessTarget`, `AwarenessParticipantId`, `AwarenessPresenceStatus`, `AwarenessPresence`, `AwarenessTyping`, `AwarenessReminder`, `DurableAwarenessUpdate`, `CollaborativeAwarenessUpdate`, `CollaborativeAwarenessConnectionToken`, `CollaborativeAwarenessUpdateOutcome`, `CollaborativeAwarenessDisconnectOutcome`, `CollaborativeAwarenessFreshness`, `CollaborativeAwarenessPresentation`, `AwarenessBadge`, `SourceStatus`, `EphemeralAwareness`, `SourceAwareness`, `StoredDurableAwareness`, `TargetObservation`, `CollaborativeAwarenessError` | Duplicate presence/unread/reminder store | Replaced by `ChannelStore`, `ThreadMetadataStore`, active ACP statuses, project and call entities | Removed | no production registration existed; module deleted | architecture test forbids new Collaborative `Store` owners |
| `CollaborativeStatusProjection`, `CollaborativeRepositoryStatus`, `CollaborativeTaskPhase`, `CollaborativeStatus` | Duplicate project/Git/status reducer | Replaced by native `StatusBar` project, branch and Git items; only ACP-specific status remains | Removed | former custom status landmark removed | source invariant and Standard/multiplayer status-bar tests |

## Agent timeline, composer and review

| Types | Capability | Authoritative owner / reused native behavior | Class | Production registration | Canonical-owner proof |
| --- | --- | --- | --- | --- | --- |
| `CollaborativeTimelineAdapter`, `CollaborativeTimelineAdapterError`, `CollaborativeAcpTimeline` | Mount only the active native entry list | Same `ThreadView`, `AgentThread`, `EntryViewState`, Markdown/code/tool/diff/terminal renderers as Agent Panel | Adapter | active `AgentPanel` reconciliation registers the list view | rich ACP GPUI/visual gates assert native code, diff, terminal, failure and permission selectors |
| `CollaborativeTimeline`, `CollaborativeTimelineEvent` | Generic activity stream when no richer native renderer exists | `ActivityReducer`; not used for ACP entries | View | owned by `MessageTimeline` and generic event projections | reducer ordering/update/disclosure tests |
| `CollaborativeActivityCard`, `ActivityCardKind`, `ActivityCardTone`, `ActivityInterventionKind`, `ActivityCardIntervention`, `ActivityCardSource`, `ActivityCardPresentation`, `ActivityCardToggleHandler`, `ActivityCardInterventionHandler` | Truthful generic event fallback | Canonical `ActivityItem` projection | View | rendered only by generic `CollaborativeTimeline` | activity-card exhaustive presentation/action tests |
| `CollaborativeComposerAdapter`, `CollaborativeComposerAdapterError` | Register active ACP composer | Exact `MessageEditor` entity and `MessageEditorEvent` lifecycle | Adapter | active `AgentPanel` reconciliation | GPUI test verifies empty rejection, focus, submit, generating state and cancel on the same thread |
| `CollaborativeAgentReviewAdapter`, `CollaborativeAgentReviewError` | Register agent changes | Exact `AgentDiffPane`, `AcpThread` and action log | Adapter | active thread composition | adapter test compares pane entity and rejects cross-project/stale action log |
| `PersonaDraft`, `TeamMemberDraft`, `TeamDraft`, `ManagedAgentDraft`, `CollaborativeAgentSettingsStatus`, `CollaborativeAgentSettingsEvent`, `CollaborativeAgentSettings`, `TeamDraftError` | Unsaved collaborative agent-settings form | Existing settings/agent configuration owners receive validated submission | View | opened from native agent settings UI | settings tests cover validation, save and failure without production fixtures |

## Rail

| Types | Capability | Authoritative owner / reused native behavior | Class | Production registration | Canonical-owner proof |
| --- | --- | --- | --- | --- | --- |
| `CollaborativeRail` | New left-rail composition and scroll | child native projections below | View | `Sidebar` mounts behind `multiplayer-tools` | rail GPUI tests cover sections, selection and scroll |
| `CollaborativeNavigationGroup`, `CollaborativeNavigationSourceId`, `CollaborativeNavigationRowId`, `CollaborativeNavigationBadge`, `CollaborativeNavigationRow`, `DuplicateCollaborativeNavigationRow`, `CollaborativeNavigationProjection` | Render-time row identity/props | `Project`, worktree/repository, `ChannelStore`, `ThreadMetadataStore` | View | recomputed while rendering each section | projection tests reject duplicate source IDs; owner update tests rerender rows |
| `CollaborativePinned`, `CollaborativePinnedState`, `RecentNavigationCandidate`, `CollaborativePinnedProjection` | Pinned/recent composition | canonical pinned navigation targets, `WorkspaceDb` recents, channels and thread metadata | View | rail section entity observes each owner | pin/unpin and canonical-owner update tests |
| `CollaborativeProjects`, `CollaborativeProjectSource`, `CollaborativeProjectGroupProjection` | Project/community hierarchy | live `MultiWorkspace` projects/worktrees/repositories and `ChannelStore` | View | rail section observes owner entities | project/channel update and navigation tests |
| `CollaborativeTasks`, `CollaborativeTaskState`, `CollaborativeTaskRow` | Task/thread rows | `ThreadMetadataStore` plus live `AgentPanel` thread status | View | rail section observes metadata and workspace | task status/navigation tests |

## Review

| Types | Capability | Authoritative owner / reused native behavior | Class | Production registration | Canonical-owner proof |
| --- | --- | --- | --- | --- | --- |
| `CollaborativeProjectReviewAdapter`, `CollaborativeProjectReviewError` | Register project changes | Exact `ProjectDiff`, `Project` and Git store entities | Adapter | Zed composition reuses an open `ProjectDiff` or creates the native item | adapter test compares the exact registered `ProjectDiff` entity |
| `CollaborativeReviewAnchor`, `CollaborativeReviewDiffTarget`, `CollaborativeReviewDiffIndex`, `CollaborativeReviewHunkState`, `CollaborativeReviewDiffResolution`, `CollaborativeReviewStaleReason`, `CollaborativeReviewDiffError`, `CollaborativeReviewDiffSourceIdentity`, `CollaborativeReviewDiffSide`, `CollaborativeReviewDiffTargetId` | Parallel file/hunk/commit/revision index | Replaced by native `ProjectDiff` navigation and diff state | Removed | had no production consumer | source invariant rejects reintroduction; adapter identity test covers native owner |
| `CollaborativeReviewFileSummary`, `CollaborativeReviewSummary`, `CollaborativeReviewSummaryError` | Parallel file list and diff totals | Replaced by native `ProjectDiff` and native status-bar Git owners | Removed | former values were test/activity-only | source invariant and native review regression |

## Hosted channel messaging

| Types | Capability | Authoritative owner / reused native behavior | Class | Production registration | Canonical-owner proof |
| --- | --- | --- | --- | --- | --- |
| `ChannelMessagingTransport`, `GlobalChannelMessaging`, `ChannelMessagingViews`, `ActiveChannel`, `PendingOperation`, `MessageTarget`, `ChannelMessageComposer` | Versioned RPC/reconnect/operation routing and composer | Collab/PostgreSQL signed-event/message projection is authoritative; Redis is notification-only | New | opened for an authorized selected `Channel`; selection/release closes subscriptions | PostgreSQL/RPC/replay tests and selection cleanup tests; Epic 51 retains the missing full two-client GUI proof |
| `MessageTimeline`, `MessageTimelineState`, `OptimisticState`, `MessageTimelineAuthorKind`, `MessageTimelineAuthor`, `MessageTimelineReaction`, `MessageTimelineContext`, `MessageTimelineEntry`, `OptimisticMessage`, `MessageTimelinePage`, `MessageTimelineError` | Window rendering and bounded optimistic reconciliation | Canonical server records and stable operation outcomes | New/View | transport feeds pages/live outcomes into the selected entity | pagination, duplicate suppression, optimistic replacement, reconnect and restart tests; no database authority exists in UI |

## Visual evidence

| Types | Capability | Authoritative owner / reused native behavior | Class | Production registration | Canonical-owner proof |
| --- | --- | --- | --- | --- | --- |
| `CollaborativeVisualVariant`, `CollaborativeRasterMetrics`, `ImageComparison`, `TestResult`, `StubAgentServer` | Exact-size test scenario, native test agent and measurements | Production composition and checked-in references | View/test | visual-test binary only | semantic selectors, region metrics, split/collapse assertions and artifact dimensions |

## Removed files and reduced modules

- Deleted `sidebar::collaborative_awareness`; no production adapter fed it.
- Reduced `workspace::collaborative_review_summary` to the action-source token;
  file lists, selection, navigation targets and diff totals remain native-owned.
- Reduced `git_ui::collaborative_review` to the `ProjectDiff` identity adapter.
- Deleted `workspace::collaborative_status`; the native status bar retains project,
  worktree, repository, branch and Git state.
- Replaced participant `view_data` synchronization across Zed, Workspace,
  Collaborative Workspace and Status Bar with a live canonical state reader.

## Explicitly retained adapters

Timeline, composer, participant and review provider/host/registration types remain
because `workspace` cannot depend on `agent_ui`, `git_ui` or `collab_ui` without
creating dependency cycles. They carry only `Entity`, `WeakEntity`, `AnyView`,
callbacks and generation tokens; selection controls their lifetime. Layout,
focus, navigation and accessibility types remain because those relationships are
specific to the Collaborative presentation and are not authoritative domain data.
