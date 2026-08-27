use crate::language_tool_tree::{LanguageToolNode, LanguageToolNodeId};
use collections::HashMap;
use project::{
    ProjectPath,
    source_coverage::{SourceCoverageSnapshot, SourceCoverageStatus},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceCoverageFilter {
    All,
    Covered,
    Uncovered,
    Partial,
}

pub struct SourceCoverageSummaryProjection {
    pub roots: Vec<LanguageToolNode>,
    navigation: HashMap<LanguageToolNodeId, ProjectPath>,
}

impl SourceCoverageSummaryProjection {
    pub fn project(snapshot: &SourceCoverageSnapshot, filter: SourceCoverageFilter) -> Self {
        let mut navigation = HashMap::default();
        let mut files = snapshot
            .files
            .iter()
            .filter(|file| match filter {
                SourceCoverageFilter::All => true,
                SourceCoverageFilter::Covered => file.covered_lines > 0,
                SourceCoverageFilter::Uncovered => file.uncovered_lines > 0,
                SourceCoverageFilter::Partial => file.truncated || snapshot.truncated,
            })
            .map(|file| {
                let id = LanguageToolNodeId(format!(
                    "source-coverage:{}:{}:{}",
                    snapshot.provider_id.0,
                    snapshot.generation,
                    file.path.path.as_unix_str()
                ));
                navigation.insert(id.clone(), file.path.clone());
                let total = file.covered_lines.saturating_add(file.uncovered_lines);
                let percentage =
                    (total > 0).then(|| u64::from(file.covered_lines) * 100 / u64::from(total));
                LanguageToolNode {
                    id,
                    label: file.path.path.as_unix_str().to_string(),
                    secondary_label: Some(match percentage {
                        Some(percentage) => format!(
                            "{percentage}% · {} covered · {} uncovered{}",
                            file.covered_lines,
                            file.uncovered_lines,
                            if file.truncated { " · truncated" } else { "" }
                        ),
                        None => "no executable lines".to_string(),
                    }),
                    icon: None,
                    accessibility_label: format!(
                        "Coverage for {}, {} covered lines, {} uncovered lines{}",
                        file.path.path.as_unix_str(),
                        file.covered_lines,
                        file.uncovered_lines,
                        if file.truncated { ", truncated" } else { "" }
                    ),
                    children: Vec::new(),
                    enabled: true,
                    activation_label: Some("Open covered source file".to_string()),
                }
            })
            .collect::<Vec<_>>();
        files.sort_by(|left, right| left.label.cmp(&right.label));
        let status = status_label(snapshot.status);
        let root = LanguageToolNode {
            id: LanguageToolNodeId(format!(
                "source-coverage:{}:{}",
                snapshot.provider_id.0, snapshot.generation
            )),
            label: format!("Coverage — {}", snapshot.provider_id.0),
            secondary_label: Some(format!(
                "{status} · {} file(s){}",
                files.len(),
                if snapshot.truncated {
                    " · truncated"
                } else {
                    ""
                }
            )),
            icon: None,
            accessibility_label: format!(
                "Source coverage provider {}, {status}, {} files{}",
                snapshot.provider_id.0,
                files.len(),
                if snapshot.truncated {
                    ", truncated"
                } else {
                    ""
                }
            ),
            children: files,
            enabled: false,
            activation_label: None,
        };
        Self {
            roots: vec![root],
            navigation,
        }
    }

    pub fn navigation(&self, id: &LanguageToolNodeId) -> Option<&ProjectPath> {
        self.navigation.get(id)
    }
}

fn status_label(status: SourceCoverageStatus) -> &'static str {
    match status {
        SourceCoverageStatus::Loading => "loading",
        SourceCoverageStatus::Current => "current",
        SourceCoverageStatus::Empty => "empty",
        SourceCoverageStatus::Partial => "partial",
        SourceCoverageStatus::Stale => "stale",
        SourceCoverageStatus::Error => "error",
        SourceCoverageStatus::Restricted => "restricted",
        SourceCoverageStatus::Disconnected => "disconnected",
        SourceCoverageStatus::Mismatch => "host mismatch",
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use project::source_coverage::{
        SourceCoverageFile, SourceCoveragePoint, SourceCoverageProviderId, SourceCoverageRange,
        SourceCoverageSnapshot,
    };
    use settings::WorktreeId;
    use util::rel_path::RelPath;

    use super::*;

    fn path(value: &str) -> ProjectPath {
        ProjectPath {
            worktree_id: WorktreeId::from_usize(1),
            path: Arc::from(RelPath::from_unix_str(value).expect("valid fixture path")),
        }
    }

    #[test]
    fn source_coverage_summary_filters_navigates_and_exposes_partial_state() {
        let snapshot = SourceCoverageSnapshot {
            project_generation: 1,
            provider_id: SourceCoverageProviderId("fake".to_string()),
            generation: 2,
            status: SourceCoverageStatus::Partial,
            files: vec![
                SourceCoverageFile {
                    path: path("src/covered.rs"),
                    ranges: Vec::new(),
                    covered_lines: 9,
                    uncovered_lines: 1,
                    truncated: false,
                },
                SourceCoverageFile {
                    path: path("src/uncovered.rs"),
                    ranges: Vec::new(),
                    covered_lines: 0,
                    uncovered_lines: 4,
                    truncated: true,
                },
            ],
            truncated: true,
            diagnostic: Some("partial report".to_string()),
        };
        let projection =
            SourceCoverageSummaryProjection::project(&snapshot, SourceCoverageFilter::Uncovered);
        assert_eq!(projection.roots[0].children.len(), 2);
        assert!(
            projection.roots[0]
                .secondary_label
                .as_deref()
                .is_some_and(|label| label.contains("partial"))
        );
        let file = &projection.roots[0].children[0];
        assert!(projection.navigation(&file.id).is_some());
        assert!(file.children.is_empty());
    }

    #[test]
    fn source_coverage_summary_projects_only_the_visible_window_at_scale() {
        let ranges = (0..100_000)
            .map(|line| SourceCoverageRange {
                start: SourceCoveragePoint { line, column: 0 },
                end: SourceCoveragePoint { line, column: 1 },
                hit_count: u64::from(line % 2),
            })
            .collect::<Vec<_>>();
        let summary =
            editor::source_coverage::summarize_visible_source_coverage(&ranges, 50_000..50_100);
        assert_eq!(summary.covered, 50);
        assert_eq!(summary.uncovered, 50);
    }

    #[gpui::test]
    fn source_coverage_summary_applies_replaces_and_clears_editor_gutter_markers(
        cx: &mut gpui::TestAppContext,
    ) {
        cx.update(|cx| {
            settings::init(cx);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            release_channel::init(semver::Version::new(0, 0, 0), cx);
            editor::init(cx);
        });
        let editor = cx.add_window(|window, cx| {
            let buffer = editor::MultiBuffer::build_simple("covered\nuncovered\n", cx);
            editor::Editor::new(editor::EditorMode::full(), buffer, None, window, cx)
        });
        let file = SourceCoverageFile {
            path: path("src/lib.rs"),
            ranges: vec![
                SourceCoverageRange {
                    start: SourceCoveragePoint { line: 0, column: 0 },
                    end: SourceCoveragePoint { line: 0, column: 1 },
                    hit_count: 1,
                },
                SourceCoverageRange {
                    start: SourceCoveragePoint { line: 1, column: 0 },
                    end: SourceCoveragePoint { line: 1, column: 1 },
                    hit_count: 0,
                },
            ],
            covered_lines: 1,
            uncovered_lines: 1,
            truncated: false,
        };
        editor
            .update(cx, |editor, _window, cx| {
                editor.apply_source_coverage(Some(&file), cx);
                assert_eq!(editor.source_coverage_gutter_counts(), (1, 1));
                editor.apply_source_coverage(
                    Some(&SourceCoverageFile {
                        ranges: vec![file.ranges[0].clone()],
                        covered_lines: 1,
                        uncovered_lines: 0,
                        ..file.clone()
                    }),
                    cx,
                );
                assert_eq!(editor.source_coverage_gutter_counts(), (1, 0));
                editor.apply_source_coverage(None, cx);
                assert_eq!(editor.source_coverage_gutter_counts(), (0, 0));
            })
            .expect("coverage editor window should remain open");
    }
}
