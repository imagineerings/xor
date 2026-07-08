use std::path::PathBuf;

use crate::{
    SIM_DIAGNOSTIC_PATH_ESCAPE_CODE, SIM_DIAGNOSTIC_UNAPPROVED_ROOT_CODE,
    SimDiagnosticEndpointStability, SimDiagnosticLogEntry, SimDiagnosticLogStream,
    SimDiagnosticRecentFile, SimDiagnosticRecentFileKind, SimDiagnosticRootKind,
    SimDiagnosticsAdapter, SimTerminalMetadata,
};

#[test]
fn diagnostics_adapter_exposes_raw_formatted_logs_and_terminal_metadata() {
    let report = SimDiagnosticsAdapter::new()
        .with_terminal(SimTerminalMetadata {
            columns: 120,
            rows: 40,
        })
        .logs([
            SimDiagnosticLogEntry::new(SimDiagnosticLogStream::Stdout, "server ready"),
            SimDiagnosticLogEntry::new(SimDiagnosticLogStream::Stderr, "model warning"),
        ]);

    assert_eq!(report.raw, "server ready\nmodel warning");
    assert_eq!(
        report.formatted,
        "[stdout] server ready\n[stderr] model warning"
    );
    assert_eq!(
        report.terminal,
        Some(SimTerminalMetadata {
            columns: 120,
            rows: 40
        })
    );
    assert_eq!(
        report.stability,
        SimDiagnosticEndpointStability::InternalUnstable
    );
}

#[test]
fn diagnostics_adapter_exposes_only_approved_folder_roots() {
    let report = SimDiagnosticsAdapter::new()
        .with_root(SimDiagnosticRootKind::Input, "/sim/input")
        .with_root(SimDiagnosticRootKind::Output, "/sim/output")
        .with_root(SimDiagnosticRootKind::Temp, "/sim/temp")
        .approved_folders();

    let folders = report
        .folders
        .iter()
        .map(|folder| (folder.root, folder.path.clone()))
        .collect::<Vec<_>>();

    assert_eq!(
        folders,
        vec![
            (SimDiagnosticRootKind::Input, PathBuf::from("/sim/input")),
            (SimDiagnosticRootKind::Output, PathBuf::from("/sim/output")),
            (SimDiagnosticRootKind::Temp, PathBuf::from("/sim/temp")),
        ]
    );
    assert_eq!(
        report.stability,
        SimDiagnosticEndpointStability::InternalUnstable
    );
}

#[test]
fn diagnostics_adapter_filters_recent_files_to_approved_roots() {
    let report = SimDiagnosticsAdapter::new()
        .with_root(SimDiagnosticRootKind::Input, "/sim/input")
        .with_root(SimDiagnosticRootKind::Output, "/sim/output")
        .recent_files([
            SimDiagnosticRecentFile::new(
                SimDiagnosticRootKind::Input,
                "prompt.png",
                SimDiagnosticRecentFileKind::Input,
            ),
            SimDiagnosticRecentFile::new(
                SimDiagnosticRootKind::Output,
                "renders/result.png",
                SimDiagnosticRecentFileKind::Output,
            ),
        ]);

    assert_eq!(report.diagnostics, Vec::new());
    assert_eq!(report.files.len(), 2);
    assert_eq!(report.files[0].path, PathBuf::from("/sim/input/prompt.png"));
    assert_eq!(
        report.files[1].path,
        PathBuf::from("/sim/output/renders/result.png")
    );
}

#[test]
fn diagnostics_adapter_rejects_unapproved_and_escaping_recent_files() {
    let report = SimDiagnosticsAdapter::new()
        .with_root(SimDiagnosticRootKind::Input, "/sim/input")
        .recent_files([
            SimDiagnosticRecentFile::new(
                SimDiagnosticRootKind::Temp,
                "scratch.bin",
                SimDiagnosticRecentFileKind::Temp,
            ),
            SimDiagnosticRecentFile::new(
                SimDiagnosticRootKind::Input,
                "../secret.txt",
                SimDiagnosticRecentFileKind::Input,
            ),
            SimDiagnosticRecentFile::new(
                SimDiagnosticRootKind::Input,
                "/absolute/path.png",
                SimDiagnosticRecentFileKind::Input,
            ),
        ]);

    let codes = report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        codes,
        vec![
            SIM_DIAGNOSTIC_UNAPPROVED_ROOT_CODE,
            SIM_DIAGNOSTIC_PATH_ESCAPE_CODE,
            SIM_DIAGNOSTIC_PATH_ESCAPE_CODE,
        ]
    );
    assert!(report.files.is_empty());
}
