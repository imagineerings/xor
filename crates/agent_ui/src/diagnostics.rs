use std::{collections::HashSet, env, fmt::Write, ops::RangeInclusive, path::Path};

use anyhow::Result;
use doctor::{
    Doctor, DoctorCheck, DoctorCheckReport, DoctorReport, DoctorStatus, ExtensionDirectoryCheck,
    SystemDependencyCheck,
};
use gpui::{
    App, AppContext as _, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement,
    ParentElement, Render, SharedString, Styled, Task, Window,
};
use language::{Anchor, BufferSnapshot, DiagnosticEntryRef, DiagnosticSeverity, ToOffset};
use project::{DiagnosticSummary, Project};
use rope::Point;
use text::OffsetRangeExt;
use ui::{
    Button, ButtonStyle, Color, Divider, DividerColor, Icon, IconName, IconSize, Label, LabelSize,
    prelude::*,
};
use util::ResultExt;
use util::paths::PathMatcher;

const PROVIDER_ENV_KEYS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GOOGLE_API_KEY",
    "GEMINI_API_KEY",
    "DEEPSEEK_API_KEY",
    "OPENROUTER_API_KEY",
];

pub struct GooseDiagnosticsView {
    report: DoctorReport,
    expanded_checks: HashSet<String>,
    focus_handle: FocusHandle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GooseDiagnosticsEvent {
    ChecksRun(DoctorReportSummary),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DoctorReportSummary {
    pub passed: usize,
    pub warnings: usize,
    pub failures: usize,
}

struct ProviderCredentialCheck;

impl DoctorCheck for ProviderCredentialCheck {
    fn name(&self) -> &str {
        "provider connectivity"
    }

    fn run(&self) -> DoctorCheckReport {
        let configured_keys = PROVIDER_ENV_KEYS
            .iter()
            .filter(|key| env::var_os(key).is_some())
            .copied()
            .collect::<Vec<_>>();

        if configured_keys.is_empty() {
            return DoctorCheckReport::warning(
                self.name(),
                "No provider credentials were found in the environment.",
                "Configure at least one provider credential or verify provider settings before starting remote model work.",
            );
        }

        DoctorCheckReport::pass(
            self.name(),
            format!(
                "{} provider credential source(s) are configured.",
                configured_keys.len()
            ),
        )
    }
}

impl GooseDiagnosticsView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut this = Self {
            report: DoctorReport::default(),
            expanded_checks: HashSet::default(),
            focus_handle: cx.focus_handle(),
        };
        this.run_checks(cx);
        this
    }

    pub fn run_checks(&mut self, cx: &mut Context<Self>) {
        self.report = run_goose_doctor_checks();
        let summary = summarize_doctor_report(&self.report);
        log_doctor_report(&self.report);
        cx.emit(GooseDiagnosticsEvent::ChecksRun(summary));
        cx.notify();
    }

    pub fn toggle_check_details(&mut self, name: &str, cx: &mut Context<Self>) {
        if !self.expanded_checks.insert(name.to_string()) {
            self.expanded_checks.remove(name);
        }
        cx.notify();
    }

    fn render_summary(&self) -> impl IntoElement {
        let summary = summarize_doctor_report(&self.report);
        h_flex()
            .gap_2()
            .child(summary_pill(
                "Passed",
                summary.passed,
                Color::Success,
                IconName::Check,
            ))
            .child(summary_pill(
                "Warnings",
                summary.warnings,
                Color::Warning,
                IconName::Warning,
            ))
            .child(summary_pill(
                "Failed",
                summary.failures,
                Color::Error,
                IconName::XCircle,
            ))
    }

    fn render_check(
        &self,
        index: usize,
        check: &DoctorCheckReport,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let expanded = self.expanded_checks.contains(&check.name);
        let name = check.name.clone();
        let (icon, color, status_label) = status_display(check.status);

        v_flex()
            .id(("goose-doctor-check", index))
            .gap_2()
            .p_3()
            .border_1()
            .rounded_sm()
            .border_color(cx.theme().colors().border)
            .child(
                h_flex()
                    .justify_between()
                    .gap_2()
                    .child(
                        h_flex()
                            .min_w_0()
                            .gap_2()
                            .child(Icon::new(icon).size(IconSize::Small).color(color))
                            .child(
                                v_flex()
                                    .min_w_0()
                                    .child(Label::new(check.name.clone()))
                                    .child(
                                        Label::new(status_label)
                                            .size(LabelSize::Small)
                                            .color(color),
                                    ),
                            ),
                    )
                    .child(
                        Button::new(
                            ("toggle-goose-doctor-check", index),
                            if expanded { "Hide" } else { "Details" },
                        )
                        .style(ButtonStyle::Subtle)
                        .on_click(cx.listener(
                            move |this, _, _window, cx| {
                                this.toggle_check_details(&name, cx);
                            },
                        )),
                    ),
            )
            .when(expanded, |this| {
                this.child(
                    v_flex()
                        .gap_2()
                        .child(Label::new(check.message.clone()).size(LabelSize::Small))
                        .when_some(check.remediation.clone(), |this, remediation| {
                            this.child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        Label::new("Remediation")
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
                                    )
                                    .child(
                                        Label::new(remediation)
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
                                    ),
                            )
                        }),
                )
            })
    }
}

impl Render for GooseDiagnosticsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let check_rows = self
            .report
            .checks
            .iter()
            .enumerate()
            .map(|(index, check)| self.render_check(index, check, cx).into_any_element())
            .collect::<Vec<_>>();

        v_flex()
            .key_context("GooseDiagnosticsView")
            .track_focus(&self.focus_handle)
            .size_full()
            .gap_4()
            .p_4()
            .child(
                h_flex()
                    .justify_between()
                    .gap_3()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(Label::new("Diagnostics").size(LabelSize::Large))
                            .child(
                                Label::new("Startup health checks")
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            ),
                    )
                    .child(
                        Button::new("rerun-goose-doctor", "Re-run")
                            .style(ButtonStyle::Filled)
                            .start_icon(Icon::new(IconName::RotateCw).size(IconSize::Small))
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.run_checks(cx);
                            })),
                    ),
            )
            .child(self.render_summary())
            .child(Divider::horizontal().color(DividerColor::Border))
            .child(v_flex().gap_2().children(check_rows))
    }
}

impl Focusable for GooseDiagnosticsView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<GooseDiagnosticsEvent> for GooseDiagnosticsView {}

pub fn run_goose_doctor_checks() -> DoctorReport {
    Doctor::new()
        .with_check(SystemDependencyCheck::new("git"))
        .with_check(ExtensionDirectoryCheck::new(
            paths::extensions_dir().clone(),
        ))
        .with_check(ProviderCredentialCheck)
        .run()
}

pub fn summarize_doctor_report(report: &DoctorReport) -> DoctorReportSummary {
    report
        .checks
        .iter()
        .fold(DoctorReportSummary::default(), |mut summary, check| {
            match check.status {
                DoctorStatus::Pass => summary.passed += 1,
                DoctorStatus::Warning => summary.warnings += 1,
                DoctorStatus::Fail => summary.failures += 1,
            }
            summary
        })
}

fn log_doctor_report(report: &DoctorReport) {
    for check in &report.checks {
        match check.status {
            DoctorStatus::Pass => {
                log::info!("doctor check passed: {}: {}", check.name, check.message);
            }
            DoctorStatus::Warning => {
                log::warn!("doctor check warning: {}: {}", check.name, check.message);
            }
            DoctorStatus::Fail => {
                log::error!("doctor check failed: {}: {}", check.name, check.message);
            }
        }
    }
}

fn summary_pill(
    label: &'static str,
    count: usize,
    color: Color,
    icon: IconName,
) -> impl IntoElement {
    h_flex()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_sm()
        .border_1()
        .child(Icon::new(icon).size(IconSize::XSmall).color(color))
        .child(Label::new(format!("{label}: {count}")).size(LabelSize::Small))
}

fn status_display(status: DoctorStatus) -> (IconName, Color, SharedString) {
    match status {
        DoctorStatus::Pass => (IconName::Check, Color::Success, "Passing".into()),
        DoctorStatus::Warning => (IconName::Warning, Color::Warning, "Warning".into()),
        DoctorStatus::Fail => (IconName::XCircle, Color::Error, "Failed".into()),
    }
}

pub fn codeblock_fence_for_path(
    path: Option<&str>,
    row_range: Option<RangeInclusive<u32>>,
) -> String {
    let mut text = String::new();
    write!(text, "```").unwrap();

    if let Some(path) = path {
        if let Some(extension) = Path::new(path).extension().and_then(|ext| ext.to_str()) {
            write!(text, "{} ", extension).unwrap();
        }

        write!(text, "{path}").unwrap();
    } else {
        write!(text, "untitled").unwrap();
    }

    if let Some(row_range) = row_range {
        write!(text, ":{}-{}", row_range.start() + 1, row_range.end() + 1).unwrap();
    }

    text.push('\n');
    text
}

pub struct DiagnosticsOptions {
    pub include_errors: bool,
    pub include_warnings: bool,
    pub path_matcher: Option<PathMatcher>,
}

/// Collects project diagnostics into a formatted string.
///
/// Returns `None` if no matching diagnostics were found.
pub fn collect_diagnostics(
    project: Entity<Project>,
    options: DiagnosticsOptions,
    cx: &mut App,
) -> Task<Result<Option<String>>> {
    let path_style = project.read(cx).path_style(cx);
    let glob_is_exact_file_match = if let Some(path) = options
        .path_matcher
        .as_ref()
        .and_then(|pm| pm.sources().next())
    {
        project
            .read(cx)
            .find_project_path(Path::new(path), cx)
            .is_some()
    } else {
        false
    };

    let project_handle = project.downgrade();
    let diagnostic_summaries: Vec<_> = project
        .read(cx)
        .diagnostic_summaries(false, cx)
        .flat_map(|(path, _, summary)| {
            let worktree = project.read(cx).worktree_for_id(path.worktree_id, cx)?;
            let full_path = worktree.read(cx).root_name().join(&path.path);
            Some((path, full_path, summary))
        })
        .collect();

    cx.spawn(async move |cx| {
        let error_source = if let Some(path_matcher) = &options.path_matcher {
            debug_assert_eq!(path_matcher.sources().count(), 1);
            Some(path_matcher.sources().next().unwrap_or_default())
        } else {
            None
        };

        let mut text = String::new();
        if let Some(error_source) = error_source.as_ref() {
            writeln!(text, "diagnostics: {}", error_source).unwrap();
        } else {
            writeln!(text, "diagnostics").unwrap();
        }

        let mut found_any_diagnostics = false;
        let mut project_summary = DiagnosticSummary::default();
        for (project_path, path, summary) in diagnostic_summaries {
            if let Some(path_matcher) = &options.path_matcher
                && !path_matcher.is_match(&path)
            {
                continue;
            }

            let has_errors = options.include_errors && summary.error_count > 0;
            let has_warnings = options.include_warnings && summary.warning_count > 0;
            if !has_errors && !has_warnings {
                continue;
            }

            if options.include_errors {
                project_summary.error_count += summary.error_count;
            }
            if options.include_warnings {
                project_summary.warning_count += summary.warning_count;
            }

            let file_path = path.display(path_style).to_string();
            if !glob_is_exact_file_match {
                writeln!(&mut text, "{file_path}").unwrap();
            }

            if let Some(buffer) = project_handle
                .update(cx, |project, cx| project.open_buffer(project_path, cx))?
                .await
                .log_err()
            {
                let snapshot = cx.read_entity(&buffer, |buffer, _| buffer.snapshot());
                if collect_buffer_diagnostics(
                    &mut text,
                    &snapshot,
                    options.include_warnings,
                    options.include_errors,
                ) {
                    found_any_diagnostics = true;
                }
            }
        }

        if !found_any_diagnostics {
            return Ok(None);
        }

        let mut label = String::new();
        label.push_str("Diagnostics");
        if let Some(source) = error_source {
            write!(label, " ({})", source).unwrap();
        }

        if project_summary.error_count > 0 || project_summary.warning_count > 0 {
            label.push(':');

            if project_summary.error_count > 0 {
                write!(label, " {} errors", project_summary.error_count).unwrap();
                if project_summary.warning_count > 0 {
                    label.push(',');
                }
            }

            if project_summary.warning_count > 0 {
                write!(label, " {} warnings", project_summary.warning_count).unwrap();
            }
        }

        // Prepend the summary label to the output.
        text.insert_str(0, &format!("{label}\n"));

        Ok(Some(text))
    })
}

/// Collects diagnostics from a buffer snapshot into the text output.
///
/// Returns `true` if any diagnostics were written.
fn collect_buffer_diagnostics(
    text: &mut String,
    snapshot: &BufferSnapshot,
    include_warnings: bool,
    include_errors: bool,
) -> bool {
    let mut found_any = false;
    for (_, group) in snapshot.diagnostic_groups(None) {
        let entry = &group.entries[group.primary_ix];
        if collect_diagnostic(text, entry, snapshot, include_warnings, include_errors) {
            found_any = true;
        }
    }
    found_any
}

/// Formats a single diagnostic entry as a code excerpt with the diagnostic message.
///
/// Returns `true` if the diagnostic was written (i.e. it matched severity filters).
fn collect_diagnostic(
    text: &mut String,
    entry: &DiagnosticEntryRef<'_, Anchor>,
    snapshot: &BufferSnapshot,
    include_warnings: bool,
    include_errors: bool,
) -> bool {
    const EXCERPT_EXPANSION_SIZE: u32 = 2;
    const MAX_MESSAGE_LENGTH: usize = 2000;

    let ty = match entry.diagnostic.severity {
        DiagnosticSeverity::WARNING => {
            if !include_warnings {
                return false;
            }
            "warning"
        }
        DiagnosticSeverity::ERROR => {
            if !include_errors {
                return false;
            }
            "error"
        }
        _ => return false,
    };

    let range = entry.range.to_point(snapshot);
    let diagnostic_row_number = range.start.row + 1;

    let start_row = range.start.row.saturating_sub(EXCERPT_EXPANSION_SIZE);
    let end_row = (range.end.row + EXCERPT_EXPANSION_SIZE).min(snapshot.max_point().row) + 1;
    let excerpt_range =
        Point::new(start_row, 0).to_offset(snapshot)..Point::new(end_row, 0).to_offset(snapshot);

    text.push_str("```");
    if let Some(language_name) = snapshot.language().map(|l| l.code_fence_block_name()) {
        text.push_str(&language_name);
    }
    text.push('\n');

    let mut buffer_text = String::new();
    for chunk in snapshot.text_for_range(excerpt_range) {
        buffer_text.push_str(chunk);
    }

    for (i, line) in buffer_text.lines().enumerate() {
        let line_number = start_row + i as u32 + 1;
        writeln!(text, "{}", line).unwrap();

        if line_number == diagnostic_row_number {
            text.push_str("//");
            let marker_start = text.len();
            write!(text, " {}: ", ty).unwrap();
            let padding = text.len() - marker_start;

            let message = util::truncate(&entry.diagnostic.message, MAX_MESSAGE_LENGTH)
                .replace('\n', format!("\n//{:padding$}", "").as_str());

            writeln!(text, "{message}").unwrap();
        }
    }

    writeln!(text, "```").unwrap();
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarizes_doctor_report_by_status() {
        let report = DoctorReport {
            checks: vec![
                DoctorCheckReport::pass("git", "available"),
                DoctorCheckReport::warning("extensions", "missing", "create directory"),
                DoctorCheckReport::fail("provider", "unreachable", "check credentials"),
            ],
        };

        assert_eq!(
            summarize_doctor_report(&report),
            DoctorReportSummary {
                passed: 1,
                warnings: 1,
                failures: 1
            }
        );
    }

    #[test]
    fn status_display_matches_doctor_status() {
        assert_eq!(status_display(DoctorStatus::Pass).1, Color::Success);
        assert_eq!(status_display(DoctorStatus::Warning).1, Color::Warning);
        assert_eq!(status_display(DoctorStatus::Fail).1, Color::Error);
    }
}
