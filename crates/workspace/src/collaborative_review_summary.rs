use std::{collections::HashSet, error::Error, fmt};

use gpui::EntityId;
use project::ProjectPath;

use crate::collaborative_review::CollaborativeReviewSlot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CollaborativeReviewSummarySource {
    slot: CollaborativeReviewSlot,
    provider_id: EntityId,
    revision: u64,
}

impl CollaborativeReviewSummarySource {
    pub fn new(slot: CollaborativeReviewSlot, provider_id: EntityId, revision: u64) -> Self {
        Self {
            slot,
            provider_id,
            revision,
        }
    }

    pub fn slot(self) -> CollaborativeReviewSlot {
        self.slot
    }

    pub fn provider_id(self) -> EntityId {
        self.provider_id
    }

    pub fn revision(self) -> u64 {
        self.revision
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollaborativeReviewFileSummary {
    file_id: String,
    project_path: ProjectPath,
}

impl CollaborativeReviewFileSummary {
    pub fn new(
        file_id: impl Into<String>,
        project_path: ProjectPath,
    ) -> Result<Self, CollaborativeReviewSummaryError> {
        let file_id = file_id.into();
        if file_id.trim().is_empty() {
            return Err(CollaborativeReviewSummaryError::EmptyFileId);
        }
        Ok(Self {
            file_id,
            project_path,
        })
    }

    pub fn file_id(&self) -> &str {
        &self.file_id
    }

    pub fn project_path(&self) -> &ProjectPath {
        &self.project_path
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollaborativeReviewSummary {
    source: CollaborativeReviewSummarySource,
    files: Vec<CollaborativeReviewFileSummary>,
    selected_file_id: Option<String>,
    additions: u32,
    deletions: u32,
}

impl CollaborativeReviewSummary {
    pub fn new(
        source: CollaborativeReviewSummarySource,
        files: Vec<CollaborativeReviewFileSummary>,
        selected_file_id: Option<String>,
        additions: u32,
        deletions: u32,
    ) -> Result<Self, CollaborativeReviewSummaryError> {
        let mut file_ids = HashSet::with_capacity(files.len());
        if files
            .iter()
            .any(|file| !file_ids.insert(file.file_id.clone()))
        {
            return Err(CollaborativeReviewSummaryError::DuplicateFile);
        }
        if let Some(selected_file_id) = selected_file_id.as_deref()
            && !file_ids.contains(selected_file_id)
        {
            return Err(CollaborativeReviewSummaryError::SelectedFileUnavailable);
        }
        Ok(Self {
            source,
            files,
            selected_file_id,
            additions,
            deletions,
        })
    }

    pub fn empty(source: CollaborativeReviewSummarySource) -> Self {
        Self {
            source,
            files: Vec::new(),
            selected_file_id: None,
            additions: 0,
            deletions: 0,
        }
    }

    pub fn source(&self) -> CollaborativeReviewSummarySource {
        self.source
    }

    pub fn files(&self) -> &[CollaborativeReviewFileSummary] {
        &self.files
    }

    pub fn selected_file_id(&self) -> Option<&str> {
        self.selected_file_id.as_deref()
    }

    pub fn additions(&self) -> u32 {
        self.additions
    }

    pub fn deletions(&self) -> u32 {
        self.deletions
    }

    pub fn select_file(
        &mut self,
        source: CollaborativeReviewSummarySource,
        file_id: &str,
    ) -> Result<bool, CollaborativeReviewSummaryError> {
        self.ensure_current_source(source)?;
        if !self.files.iter().any(|file| file.file_id() == file_id) {
            return Err(CollaborativeReviewSummaryError::FileUnavailable);
        }
        let changed = self.selected_file_id.as_deref() != Some(file_id);
        if changed {
            self.selected_file_id = Some(file_id.to_owned());
        }
        Ok(changed)
    }

    pub fn navigation_target(
        &self,
        source: CollaborativeReviewSummarySource,
        file_id: &str,
    ) -> Result<ProjectPath, CollaborativeReviewSummaryError> {
        self.ensure_current_source(source)?;
        self.files
            .iter()
            .find(|file| file.file_id() == file_id)
            .map(|file| file.project_path.clone())
            .ok_or(CollaborativeReviewSummaryError::FileUnavailable)
    }

    pub fn replace(
        &mut self,
        replacement: CollaborativeReviewSummary,
    ) -> Result<(), CollaborativeReviewSummaryError> {
        if replacement.source.slot != self.source.slot
            || replacement.source.provider_id != self.source.provider_id
        {
            return Err(CollaborativeReviewSummaryError::StaleProvider);
        }
        if replacement.source.revision <= self.source.revision {
            return Err(CollaborativeReviewSummaryError::StaleRevision);
        }
        *self = replacement;
        Ok(())
    }

    fn ensure_current_source(
        &self,
        source: CollaborativeReviewSummarySource,
    ) -> Result<(), CollaborativeReviewSummaryError> {
        if source.slot != self.source.slot || source.provider_id != self.source.provider_id {
            return Err(CollaborativeReviewSummaryError::StaleProvider);
        }
        if source.revision != self.source.revision {
            return Err(CollaborativeReviewSummaryError::StaleRevision);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CollaborativeReviewSummaryError {
    EmptyFileId,
    DuplicateFile,
    SelectedFileUnavailable,
    FileUnavailable,
    StaleProvider,
    StaleRevision,
}

impl fmt::Display for CollaborativeReviewSummaryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyFileId => "review file identity must not be empty",
            Self::DuplicateFile => "review file identity is duplicated",
            Self::SelectedFileUnavailable => "selected review file is unavailable",
            Self::FileUnavailable => "review file is unavailable",
            Self::StaleProvider => "review summary provider is no longer current",
            Self::StaleRevision => "review summary revision is no longer current",
        };
        formatter.write_str(message)
    }
}

impl Error for CollaborativeReviewSummaryError {}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};
    use settings::WorktreeId;
    use util::rel_path::RelPath;

    use super::*;

    fn project_path(path: &str) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_usize(1),
            path: RelPath::from_unix_str(path)
                .expect("test path should be relative")
                .into(),
        }
    }

    fn file(file_id: &str, path: &str) -> CollaborativeReviewFileSummary {
        CollaborativeReviewFileSummary::new(file_id, project_path(path))
            .expect("test file should be valid")
    }

    #[gpui::test]
    fn collaborative_review_summary(cx: &mut TestAppContext) {
        let (provider_id, replacement_provider_id) =
            cx.update(|cx| (cx.new(|_| ()).entity_id(), cx.new(|_| ()).entity_id()));
        let source = CollaborativeReviewSummarySource::new(
            CollaborativeReviewSlot::ProjectChanges,
            provider_id,
            1,
        );
        let mut summary = CollaborativeReviewSummary::new(
            source,
            vec![file("file-1", "src/main.rs"), file("file-2", "src/lib.rs")],
            Some("file-1".into()),
            12,
            4,
        )
        .expect("current native summary should be valid");
        assert_eq!(summary.additions(), 12);
        assert_eq!(summary.deletions(), 4);
        assert_eq!(summary.selected_file_id(), Some("file-1"));
        assert_eq!(
            summary
                .navigation_target(source, "file-1")
                .expect("current file should navigate"),
            project_path("src/main.rs")
        );
        assert!(
            summary
                .select_file(source, "file-2")
                .expect("current file should select")
        );
        assert_eq!(summary.selected_file_id(), Some("file-2"));

        let stale_provider = CollaborativeReviewSummarySource::new(
            CollaborativeReviewSlot::ProjectChanges,
            replacement_provider_id,
            1,
        );
        assert_eq!(
            summary.navigation_target(stale_provider, "file-1"),
            Err(CollaborativeReviewSummaryError::StaleProvider)
        );
        let stale_revision = CollaborativeReviewSummarySource::new(
            CollaborativeReviewSlot::ProjectChanges,
            provider_id,
            0,
        );
        assert_eq!(
            summary.select_file(stale_revision, "file-1"),
            Err(CollaborativeReviewSummaryError::StaleRevision)
        );
        assert_eq!(
            summary.navigation_target(source, "missing"),
            Err(CollaborativeReviewSummaryError::FileUnavailable)
        );

        let next_source = CollaborativeReviewSummarySource::new(
            CollaborativeReviewSlot::ProjectChanges,
            provider_id,
            2,
        );
        summary
            .replace(CollaborativeReviewSummary::empty(next_source))
            .expect("newer zero-change projection should replace current summary");
        assert!(summary.files().is_empty());
        assert_eq!(summary.selected_file_id(), None);
        assert_eq!((summary.additions(), summary.deletions()), (0, 0));
        assert_eq!(
            summary.replace(CollaborativeReviewSummary::empty(source)),
            Err(CollaborativeReviewSummaryError::StaleRevision)
        );
    }
}
