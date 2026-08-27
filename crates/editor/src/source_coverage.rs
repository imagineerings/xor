use std::ops::Range;

use gpui::Context;
use language::{Bias, Point};
use project::source_coverage::{SourceCoverageFile, SourceCoverageRange};
use theme::ActiveTheme as _;

use crate::Editor;

enum CoveredSourceRange {}
enum UncoveredSourceRange {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceCoverageVisibleSummary {
    pub covered: usize,
    pub uncovered: usize,
}

pub fn summarize_visible_source_coverage(
    ranges: &[SourceCoverageRange],
    visible_lines: Range<u32>,
) -> SourceCoverageVisibleSummary {
    let mut summary = SourceCoverageVisibleSummary::default();
    let start = ranges.partition_point(|range| range.end.line < visible_lines.start);
    for range in &ranges[start..] {
        if range.start.line >= visible_lines.end {
            break;
        }
        if range.is_covered() {
            summary.covered += 1;
        } else {
            summary.uncovered += 1;
        }
    }
    summary
}

impl Editor {
    pub fn apply_source_coverage(
        &mut self,
        coverage: Option<&SourceCoverageFile>,
        cx: &mut Context<Self>,
    ) {
        let Some(coverage) = coverage else {
            self.clear_gutter_highlights::<CoveredSourceRange>(cx);
            self.clear_gutter_highlights::<UncoveredSourceRange>(cx);
            return;
        };
        let snapshot = self.buffer().read(cx).snapshot(cx);
        let mut covered = Vec::new();
        let mut uncovered = Vec::new();
        for range in &coverage.ranges {
            let start =
                snapshot.clip_point(Point::new(range.start.line, range.start.column), Bias::Left);
            let end =
                snapshot.clip_point(Point::new(range.end.line, range.end.column), Bias::Right);
            let anchors = snapshot.anchor_before(start)..snapshot.anchor_after(end);
            if range.is_covered() {
                covered.push(anchors);
            } else {
                uncovered.push(anchors);
            }
        }
        self.highlight_gutter::<CoveredSourceRange>(covered, |cx| cx.theme().status().success, cx);
        self.highlight_gutter::<UncoveredSourceRange>(
            uncovered,
            |cx| cx.theme().status().error,
            cx,
        );
    }

    #[cfg(any(test, feature = "test-support"))]
    pub fn source_coverage_gutter_counts(&self) -> (usize, usize) {
        let covered = self
            .gutter_highlights
            .get(&std::any::TypeId::of::<CoveredSourceRange>())
            .map(|(_, ranges)| ranges.len())
            .unwrap_or_default();
        let uncovered = self
            .gutter_highlights
            .get(&std::any::TypeId::of::<UncoveredSourceRange>())
            .map(|(_, ranges)| ranges.len())
            .unwrap_or_default();
        (covered, uncovered)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use project::source_coverage::SourceCoveragePoint;
    use settings::WorktreeId;
    use util::rel_path::RelPath;

    use super::*;
    use crate::{EditorMode, MultiBuffer};

    fn range(line: u32, hit_count: u64) -> SourceCoverageRange {
        SourceCoverageRange {
            start: SourceCoveragePoint { line, column: 0 },
            end: SourceCoveragePoint { line, column: 1 },
            hit_count,
        }
    }

    #[test]
    fn source_coverage_large_visible_projection_is_range_bounded() {
        let ranges = (0..100_000)
            .map(|line| range(line, u64::from(line % 2)))
            .collect::<Vec<_>>();
        let summary = summarize_visible_source_coverage(&ranges, 50_000..50_100);
        assert_eq!(summary.covered, 50);
        assert_eq!(summary.uncovered, 50);
    }

    #[gpui::test]
    fn source_coverage_gutter_markers_replace_and_clear_without_stale_state(
        cx: &mut gpui::TestAppContext,
    ) {
        crate::editor_tests::init_test(cx, |_| {});
        let editor = cx.add_window(|window, cx| {
            let buffer = MultiBuffer::build_simple("covered\nuncovered\n", cx);
            Editor::new(EditorMode::full(), buffer, None, window, cx)
        });
        let file = SourceCoverageFile {
            path: project::ProjectPath {
                worktree_id: WorktreeId::from_usize(1),
                path: Arc::from(RelPath::from_unix_str("src/lib.rs").expect("valid fixture path")),
            },
            ranges: vec![range(0, 1), range(1, 0)],
            covered_lines: 1,
            uncovered_lines: 1,
            truncated: false,
        };
        editor
            .update(cx, |editor, _window, cx| {
                editor.apply_source_coverage(Some(&file), cx);
                assert_eq!(editor.source_coverage_gutter_counts(), (1, 1));
                editor.apply_source_coverage(None, cx);
                assert_eq!(editor.source_coverage_gutter_counts(), (0, 0));
            })
            .expect("update editor window");
    }
}
