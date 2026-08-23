use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    ops::Range,
};

use collaboration_domain::{
    AggregateId, GitCommitId, PatchRevision, PatchRevisionNumber, ReviewComment, ReviewDiffSide,
    ReviewFilePath, ReviewHunkId,
};
use gpui::{App, Entity, EntityId};
use language::Point;
use project::{Project, ProjectPath};
use workspace::{
    Workspace,
    collaborative_review::{
        CollaborativeReviewRegistration, CollaborativeReviewRegistrationError,
        CollaborativeReviewSlot,
    },
};

use crate::project_diff::ProjectDiff;

const MAX_STABLE_FILE_ID_BYTES: usize = 4_096;
const MAX_NATIVE_REVIEW_TARGETS: usize = 100_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CollaborativeReviewDiffSourceIdentity {
    project_id: EntityId,
    git_store_id: EntityId,
    project_diff_id: EntityId,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum CollaborativeReviewDiffSide {
    Base,
    Head,
}

impl From<ReviewDiffSide> for CollaborativeReviewDiffSide {
    fn from(side: ReviewDiffSide) -> Self {
        match side {
            ReviewDiffSide::Base => Self::Base,
            ReviewDiffSide::Head => Self::Head,
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct CollaborativeReviewDiffTargetId {
    file_id: String,
    hunk_id: ReviewHunkId,
    side: CollaborativeReviewDiffSide,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollaborativeReviewAnchor {
    repository_id: AggregateId,
    review_id: AggregateId,
    revision: PatchRevisionNumber,
    commit: GitCommitId,
    file_id: String,
    original_file_path: ReviewFilePath,
    hunk_id: ReviewHunkId,
    side: ReviewDiffSide,
}

impl CollaborativeReviewAnchor {
    pub fn from_comment(
        comment: &ReviewComment,
        stable_file_id: impl Into<String>,
    ) -> Result<Self, CollaborativeReviewDiffError> {
        let stable_file_id = stable_file_id.into();
        validate_stable_file_id(&stable_file_id)?;
        if comment.comment_id.as_uuid().is_nil()
            || comment.author_principal_id.as_uuid().is_nil()
            || comment.anchor.end_line < comment.anchor.start_line
        {
            return Err(CollaborativeReviewDiffError::InvalidAnchor);
        }
        Ok(Self {
            repository_id: comment.review.repository_id(),
            review_id: comment.review.review_id(),
            revision: comment.anchor.revision,
            commit: comment.anchor.commit.clone(),
            file_id: stable_file_id,
            original_file_path: comment.anchor.file_path.clone(),
            hunk_id: comment.anchor.hunk_id.clone(),
            side: comment.anchor.side,
        })
    }

    pub const fn repository_id(&self) -> AggregateId {
        self.repository_id
    }

    pub const fn review_id(&self) -> AggregateId {
        self.review_id
    }

    pub const fn revision(&self) -> PatchRevisionNumber {
        self.revision
    }

    pub fn commit(&self) -> &GitCommitId {
        &self.commit
    }

    pub fn file_id(&self) -> &str {
        &self.file_id
    }

    pub fn original_file_path(&self) -> &ReviewFilePath {
        &self.original_file_path
    }

    pub fn hunk_id(&self) -> &ReviewHunkId {
        &self.hunk_id
    }

    pub const fn side(&self) -> ReviewDiffSide {
        self.side
    }

    fn target_id(&self) -> CollaborativeReviewDiffTargetId {
        CollaborativeReviewDiffTargetId {
            file_id: self.file_id.clone(),
            hunk_id: self.hunk_id.clone(),
            side: self.side.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeReviewHunkState {
    Available,
    Conflicting,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollaborativeReviewDiffTarget {
    id: CollaborativeReviewDiffTargetId,
    current_file_path: ReviewFilePath,
    project_path: ProjectPath,
    hunk_range: Range<Point>,
    state: CollaborativeReviewHunkState,
}

impl CollaborativeReviewDiffTarget {
    pub fn new(
        stable_file_id: impl Into<String>,
        hunk_id: ReviewHunkId,
        side: ReviewDiffSide,
        current_file_path: ReviewFilePath,
        project_path: ProjectPath,
        hunk_range: Range<Point>,
        state: CollaborativeReviewHunkState,
    ) -> Result<Self, CollaborativeReviewDiffError> {
        let stable_file_id = stable_file_id.into();
        validate_stable_file_id(&stable_file_id)?;
        if hunk_range.start > hunk_range.end
            || project_path.path.as_unix_str() != current_file_path.as_str()
        {
            return Err(CollaborativeReviewDiffError::InvalidTarget);
        }
        Ok(Self {
            id: CollaborativeReviewDiffTargetId {
                file_id: stable_file_id,
                hunk_id,
                side: side.into(),
            },
            current_file_path,
            project_path,
            hunk_range,
            state,
        })
    }

    pub fn file_id(&self) -> &str {
        &self.id.file_id
    }

    pub fn hunk_id(&self) -> &ReviewHunkId {
        &self.id.hunk_id
    }

    pub fn current_file_path(&self) -> &ReviewFilePath {
        &self.current_file_path
    }

    pub fn project_path(&self) -> &ProjectPath {
        &self.project_path
    }

    pub fn hunk_range(&self) -> &Range<Point> {
        &self.hunk_range
    }

    pub const fn state(&self) -> CollaborativeReviewHunkState {
        self.state
    }
}

pub struct CollaborativeReviewDiffIndex {
    sources: CollaborativeReviewDiffSourceIdentity,
    repository_id: AggregateId,
    review_id: AggregateId,
    revision: PatchRevisionNumber,
    base_commit: GitCommitId,
    head_commit: GitCommitId,
    targets: HashMap<CollaborativeReviewDiffTargetId, CollaborativeReviewDiffTarget>,
    file_ids: HashSet<String>,
    deleted_files: HashMap<String, ReviewFilePath>,
}

impl CollaborativeReviewDiffIndex {
    fn new(
        sources: CollaborativeReviewDiffSourceIdentity,
        revision: &PatchRevision,
    ) -> Result<Self, CollaborativeReviewDiffError> {
        if revision.revision_id.as_uuid().is_nil()
            || revision.author_principal_id.as_uuid().is_nil()
            || revision.base_commit == revision.head_commit
        {
            return Err(CollaborativeReviewDiffError::InvalidRevision);
        }
        Ok(Self {
            sources,
            repository_id: revision.review.repository_id(),
            review_id: revision.review.review_id(),
            revision: revision.number,
            base_commit: revision.base_commit.clone(),
            head_commit: revision.head_commit.clone(),
            targets: HashMap::default(),
            file_ids: HashSet::default(),
            deleted_files: HashMap::default(),
        })
    }

    pub fn insert(
        &mut self,
        target: CollaborativeReviewDiffTarget,
    ) -> Result<(), CollaborativeReviewDiffError> {
        if self.deleted_files.contains_key(target.file_id()) {
            return Err(CollaborativeReviewDiffError::ConflictingFileState);
        }
        if self.targets.contains_key(&target.id) {
            return Err(CollaborativeReviewDiffError::DuplicateTarget);
        }
        if self.targets.len() + self.deleted_files.len() >= MAX_NATIVE_REVIEW_TARGETS {
            return Err(CollaborativeReviewDiffError::TooManyTargets);
        }
        self.file_ids.insert(target.file_id().to_owned());
        self.targets.insert(target.id.clone(), target);
        Ok(())
    }

    pub fn mark_file_deleted(
        &mut self,
        stable_file_id: impl Into<String>,
        last_known_file_path: ReviewFilePath,
    ) -> Result<(), CollaborativeReviewDiffError> {
        let stable_file_id = stable_file_id.into();
        validate_stable_file_id(&stable_file_id)?;
        if self.file_ids.contains(&stable_file_id) {
            return Err(CollaborativeReviewDiffError::ConflictingFileState);
        }
        if self.deleted_files.contains_key(&stable_file_id) {
            return Err(CollaborativeReviewDiffError::DuplicateDeletedFile);
        }
        if self.targets.len() + self.deleted_files.len() >= MAX_NATIVE_REVIEW_TARGETS {
            return Err(CollaborativeReviewDiffError::TooManyTargets);
        }
        self.deleted_files
            .insert(stable_file_id, last_known_file_path);
        Ok(())
    }

    fn resolve(
        &self,
        sources: CollaborativeReviewDiffSourceIdentity,
        anchor: &CollaborativeReviewAnchor,
    ) -> CollaborativeReviewDiffResolution {
        if self.sources != sources {
            return CollaborativeReviewDiffResolution::Stale {
                reason: CollaborativeReviewStaleReason::Sources,
            };
        }
        if self.repository_id != anchor.repository_id {
            return CollaborativeReviewDiffResolution::Stale {
                reason: CollaborativeReviewStaleReason::Repository,
            };
        }
        if self.review_id != anchor.review_id {
            return CollaborativeReviewDiffResolution::Stale {
                reason: CollaborativeReviewStaleReason::Review,
            };
        }
        if self.revision != anchor.revision {
            return CollaborativeReviewDiffResolution::Stale {
                reason: CollaborativeReviewStaleReason::Revision {
                    requested: anchor.revision,
                    current: self.revision,
                },
            };
        }
        let current_commit = match anchor.side {
            ReviewDiffSide::Base => &self.base_commit,
            ReviewDiffSide::Head => &self.head_commit,
        };
        if current_commit != &anchor.commit {
            return CollaborativeReviewDiffResolution::Stale {
                reason: CollaborativeReviewStaleReason::Commit {
                    requested: anchor.commit.clone(),
                    current: current_commit.clone(),
                },
            };
        }
        if let Some(target) = self.targets.get(&anchor.target_id()) {
            if target.state == CollaborativeReviewHunkState::Conflicting {
                return CollaborativeReviewDiffResolution::Conflicting {
                    target: target.clone(),
                };
            }
            if target.current_file_path == anchor.original_file_path {
                return CollaborativeReviewDiffResolution::Exact {
                    target: target.clone(),
                };
            }
            return CollaborativeReviewDiffResolution::Moved {
                original_file_path: anchor.original_file_path.clone(),
                target: target.clone(),
            };
        }
        if let Some(last_known_file_path) = self.deleted_files.get(&anchor.file_id) {
            return CollaborativeReviewDiffResolution::Deleted {
                file_id: anchor.file_id.clone(),
                last_known_file_path: last_known_file_path.clone(),
            };
        }
        CollaborativeReviewDiffResolution::Stale {
            reason: if self.file_ids.contains(&anchor.file_id) {
                CollaborativeReviewStaleReason::Hunk
            } else {
                CollaborativeReviewStaleReason::File
            },
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollaborativeReviewDiffResolution {
    Exact {
        target: CollaborativeReviewDiffTarget,
    },
    Moved {
        original_file_path: ReviewFilePath,
        target: CollaborativeReviewDiffTarget,
    },
    Stale {
        reason: CollaborativeReviewStaleReason,
    },
    Deleted {
        file_id: String,
        last_known_file_path: ReviewFilePath,
    },
    Conflicting {
        target: CollaborativeReviewDiffTarget,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CollaborativeReviewStaleReason {
    Sources,
    Repository,
    Review,
    Revision {
        requested: PatchRevisionNumber,
        current: PatchRevisionNumber,
    },
    Commit {
        requested: GitCommitId,
        current: GitCommitId,
    },
    File,
    Hunk,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeReviewDiffError {
    InvalidAnchor,
    InvalidRevision,
    InvalidStableFileId,
    InvalidTarget,
    DuplicateTarget,
    DuplicateDeletedFile,
    ConflictingFileState,
    TooManyTargets,
}

impl fmt::Display for CollaborativeReviewDiffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidAnchor => "the collaborative review anchor is invalid",
            Self::InvalidRevision => "the collaborative review revision is invalid",
            Self::InvalidStableFileId => "the collaborative review file identity is invalid",
            Self::InvalidTarget => "the native collaborative review target is invalid",
            Self::DuplicateTarget => "the native collaborative review target is duplicated",
            Self::DuplicateDeletedFile => "the deleted collaborative review file is duplicated",
            Self::ConflictingFileState => {
                "the collaborative review file is both current and deleted"
            }
            Self::TooManyTargets => "the native collaborative review target limit was exceeded",
        };
        formatter.write_str(message)
    }
}

impl Error for CollaborativeReviewDiffError {}

fn validate_stable_file_id(value: &str) -> Result<(), CollaborativeReviewDiffError> {
    if value.trim().is_empty()
        || value.len() > MAX_STABLE_FILE_ID_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(CollaborativeReviewDiffError::InvalidStableFileId);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollaborativeProjectReviewError {
    ProjectDiffUnavailable,
    StaleProject,
    StaleGitStore,
    Registration(CollaborativeReviewRegistrationError),
}

impl fmt::Display for CollaborativeProjectReviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectDiffUnavailable => formatter.write_str("the project diff is unavailable"),
            Self::StaleProject => formatter.write_str("the review belongs to another project"),
            Self::StaleGitStore => formatter.write_str("the review Git store is no longer current"),
            Self::Registration(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl Error for CollaborativeProjectReviewError {}

pub struct CollaborativeProjectReviewAdapter {
    project: Entity<Project>,
    git_store_id: EntityId,
    project_diff: Entity<ProjectDiff>,
}

impl CollaborativeProjectReviewAdapter {
    pub fn from_workspace(
        workspace: &Entity<Workspace>,
        cx: &App,
    ) -> Result<Self, CollaborativeProjectReviewError> {
        let workspace = workspace.read(cx);
        let project = workspace.project().clone();
        let git_store_id = project.read(cx).git_store().entity_id();
        let project_diff = workspace
            .item_of_type::<ProjectDiff>(cx)
            .ok_or(CollaborativeProjectReviewError::ProjectDiffUnavailable)?;
        Ok(Self {
            project,
            git_store_id,
            project_diff,
        })
    }

    pub fn project_diff(&self) -> &Entity<ProjectDiff> {
        &self.project_diff
    }

    pub fn review_diff_index(
        &self,
        revision: &PatchRevision,
    ) -> Result<CollaborativeReviewDiffIndex, CollaborativeReviewDiffError> {
        CollaborativeReviewDiffIndex::new(self.diff_source_identity(), revision)
    }

    pub fn resolve_review_anchor(
        &self,
        anchor: &CollaborativeReviewAnchor,
        index: &CollaborativeReviewDiffIndex,
        cx: &App,
    ) -> CollaborativeReviewDiffResolution {
        if self.project.read(cx).git_store().entity_id() != self.git_store_id {
            return CollaborativeReviewDiffResolution::Stale {
                reason: CollaborativeReviewStaleReason::Sources,
            };
        }
        index.resolve(self.diff_source_identity(), anchor)
    }

    pub fn register_in_workspace(
        &self,
        workspace: &mut Workspace,
        cx: &mut gpui::Context<Workspace>,
    ) -> Result<CollaborativeReviewRegistration, CollaborativeProjectReviewError> {
        if self.project.entity_id() != workspace.project().entity_id() {
            return Err(CollaborativeProjectReviewError::StaleProject);
        }
        if self.project.read(cx).git_store().entity_id() != self.git_store_id {
            return Err(CollaborativeProjectReviewError::StaleGitStore);
        }

        workspace
            .register_collaborative_review_provider(
                CollaborativeReviewSlot::ProjectChanges,
                &self.project,
                self.project_diff.clone().into(),
                cx,
            )
            .map_err(CollaborativeProjectReviewError::Registration)
    }

    fn diff_source_identity(&self) -> CollaborativeReviewDiffSourceIdentity {
        CollaborativeReviewDiffSourceIdentity {
            project_id: self.project.entity_id(),
            git_store_id: self.git_store_id,
            project_diff_id: self.project_diff.entity_id(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use collaboration_domain::{
        BranchCollaborationIdentity, BranchGeneration, BranchRefName, CommunityId,
        PatchRevisionNumber, PrincipalId, ReviewCommentAnchor, ReviewCommentBody, ReviewIdentity,
    };
    use fs::FakeFs;
    use gpui::{AppContext as _, BorrowAppContext as _, TestAppContext};
    use project::Project;
    use settings::{DiffViewStyle, SettingsStore, WorktreeId};
    use util::rel_path::RelPath;
    use uuid::Uuid;
    use workspace::MultiWorkspace;

    use super::*;

    fn aggregate(value: u128) -> AggregateId {
        AggregateId::from_uuid(Uuid::from_u128(value))
    }

    fn commit(value: u64) -> GitCommitId {
        GitCommitId::parse(format!("{value:040x}")).expect("valid commit")
    }

    fn review_identity(review_id: u128) -> ReviewIdentity {
        ReviewIdentity::new(
            aggregate(review_id),
            BranchCollaborationIdentity::new(
                CommunityId::from_uuid(Uuid::from_u128(1)),
                aggregate(2),
                BranchRefName::parse("refs/heads/feature/review").expect("valid branch"),
                BranchGeneration::FIRST,
            )
            .expect("valid branch identity"),
        )
        .expect("valid review identity")
    }

    fn revision(review_id: u128, number: u64, head: u64) -> PatchRevision {
        PatchRevision {
            revision_id: aggregate(10 + number as u128),
            review: review_identity(review_id),
            number: PatchRevisionNumber::new(number).expect("positive revision"),
            base_commit: commit(100),
            head_commit: commit(head),
            author_principal_id: PrincipalId::from_uuid(Uuid::from_u128(20)),
            created_at_millis: 1_900_000_000_000,
        }
    }

    fn comment(review_id: u128, revision: u64, head: u64, path: &str) -> ReviewComment {
        ReviewComment {
            comment_id: aggregate(30),
            review: review_identity(review_id),
            author_principal_id: PrincipalId::from_uuid(Uuid::from_u128(21)),
            body: ReviewCommentBody::new("Keep the tenant fence").expect("valid body"),
            anchor: ReviewCommentAnchor::new(
                PatchRevisionNumber::new(revision).expect("positive revision"),
                commit(head),
                ReviewFilePath::new(path).expect("valid path"),
                ReviewHunkId::parse("a".repeat(64)).expect("valid hunk"),
                ReviewDiffSide::Head,
                NonZeroU32::new(10).expect("nonzero line"),
                NonZeroU32::new(14).expect("nonzero line"),
            )
            .expect("valid anchor"),
            created_at_millis: 1_900_000_001_000,
        }
    }

    fn project_path(path: &str) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_usize(1),
            path: RelPath::from_unix_str(path)
                .expect("valid project path")
                .into(),
        }
    }

    fn target(path: &str, state: CollaborativeReviewHunkState) -> CollaborativeReviewDiffTarget {
        CollaborativeReviewDiffTarget::new(
            "stable-file-1",
            ReviewHunkId::parse("a".repeat(64)).expect("valid hunk"),
            ReviewDiffSide::Head,
            ReviewFilePath::new(path).expect("valid review path"),
            project_path(path),
            Point::new(9, 0)..Point::new(14, 0),
            state,
        )
        .expect("valid native target")
    }

    #[gpui::test]
    async fn collaborative_project_review_adapter(cx: &mut TestAppContext) {
        cx.update(|cx| {
            let store = SettingsStore::test(cx);
            cx.set_global(store);
            cx.update_global::<SettingsStore, _>(|store, cx| {
                store.update_user_settings(cx, |settings| {
                    settings.editor.diff_view_style = Some(DiffViewStyle::Unified);
                });
            });
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            editor::init(cx);
            crate::init(cx);
        });

        let fs = FakeFs::new(cx.executor());
        let project = Project::test(fs.clone(), [], cx).await;
        let other_project = Project::test(fs, [], cx).await;
        let (multi_workspace, cx) =
            cx.add_window_view(|window, cx| MultiWorkspace::test_new(project.clone(), window, cx));
        let workspace =
            multi_workspace.read_with(cx, |multi_workspace, _| multi_workspace.workspace().clone());

        let unavailable = cx
            .update(|_, cx| CollaborativeProjectReviewAdapter::from_workspace(&workspace, cx))
            .err()
            .expect("an unopened project diff should remain unavailable");
        assert_eq!(
            unavailable,
            CollaborativeProjectReviewError::ProjectDiffUnavailable
        );

        workspace.update_in(cx, |workspace, window, cx| {
            ProjectDiff::deploy_at(workspace, None, window, cx);
        });
        let adapter = cx
            .update(|_, cx| CollaborativeProjectReviewAdapter::from_workspace(&workspace, cx))
            .expect("the existing native project diff should adapt");
        let project_diff_id = adapter.project_diff().entity_id();
        workspace
            .update(cx, |workspace, cx| {
                adapter.register_in_workspace(workspace, cx)
            })
            .expect("the project diff should register in its canonical workspace");
        workspace.read_with(cx, |workspace, _| {
            assert_eq!(
                workspace
                    .collaborative_review()
                    .selected_view()
                    .expect("project review should be selected")
                    .entity_id(),
                project_diff_id
            );
        });

        let (other_multi_workspace, cx) =
            cx.add_window_view(|window, cx| MultiWorkspace::test_new(other_project, window, cx));
        let other_workspace = other_multi_workspace
            .read_with(cx, |multi_workspace, _| multi_workspace.workspace().clone());
        let stale = other_workspace
            .update(cx, |workspace, cx| {
                adapter.register_in_workspace(workspace, cx)
            })
            .expect_err("cross-project reuse should fail closed");
        assert_eq!(stale, CollaborativeProjectReviewError::StaleProject);
    }

    #[gpui::test]
    fn collaborative_review_diff_anchor_resolution(cx: &mut TestAppContext) {
        let sources = cx.update(|cx| CollaborativeReviewDiffSourceIdentity {
            project_id: cx.new(|_| ()).entity_id(),
            git_store_id: cx.new(|_| ()).entity_id(),
            project_diff_id: cx.new(|_| ()).entity_id(),
        });
        let current_revision = revision(3, 2, 102);
        let anchor = CollaborativeReviewAnchor::from_comment(
            &comment(3, 2, 102, "src/original.rs"),
            "stable-file-1",
        )
        .expect("valid collaborative anchor");

        let mut exact = CollaborativeReviewDiffIndex::new(sources, &current_revision)
            .expect("valid native diff index");
        exact
            .insert(target(
                "src/original.rs",
                CollaborativeReviewHunkState::Available,
            ))
            .expect("insert exact target");
        assert!(matches!(
            exact.resolve(sources, &anchor),
            CollaborativeReviewDiffResolution::Exact { .. }
        ));

        let mut moved = CollaborativeReviewDiffIndex::new(sources, &current_revision)
            .expect("valid native diff index");
        moved
            .insert(target(
                "src/renamed.rs",
                CollaborativeReviewHunkState::Available,
            ))
            .expect("insert moved target");
        let CollaborativeReviewDiffResolution::Moved {
            original_file_path,
            target: moved_target,
        } = moved.resolve(sources, &anchor)
        else {
            panic!("renamed stable file should resolve as moved")
        };
        assert_eq!(original_file_path.as_str(), "src/original.rs");
        assert_eq!(moved_target.project_path(), &project_path("src/renamed.rs"));

        let stale_revision = revision(3, 3, 103);
        let stale = CollaborativeReviewDiffIndex::new(sources, &stale_revision)
            .expect("valid stale native diff index");
        assert!(matches!(
            stale.resolve(sources, &anchor),
            CollaborativeReviewDiffResolution::Stale {
                reason: CollaborativeReviewStaleReason::Revision { .. }
            }
        ));

        let mut deleted = CollaborativeReviewDiffIndex::new(sources, &current_revision)
            .expect("valid native diff index");
        deleted
            .mark_file_deleted(
                "stable-file-1",
                ReviewFilePath::new("src/original.rs").expect("valid path"),
            )
            .expect("mark deleted file");
        assert!(matches!(
            deleted.resolve(sources, &anchor),
            CollaborativeReviewDiffResolution::Deleted { .. }
        ));

        let mut conflicting = CollaborativeReviewDiffIndex::new(sources, &current_revision)
            .expect("valid native diff index");
        conflicting
            .insert(target(
                "src/original.rs",
                CollaborativeReviewHunkState::Conflicting,
            ))
            .expect("insert conflicting target");
        assert!(matches!(
            conflicting.resolve(sources, &anchor),
            CollaborativeReviewDiffResolution::Conflicting { .. }
        ));
    }
}
