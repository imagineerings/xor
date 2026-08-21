#!/usr/bin/env python3
"""Regenerate evidence catalogs for the vendored Comfy-Desktop source tree."""

from __future__ import annotations

import csv
import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path


ROOT = Path(__file__).resolve().parents[4]
DESKTOP = ROOT / "projects/comfy/Comfy-Desktop"
OUT = Path(__file__).resolve().parent


def rel(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def write_csv(name: str, fieldnames: list[str], rows: list[dict[str, object]]) -> None:
    with (OUT / name).open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(
            stream,
            fieldnames=fieldnames,
            extrasaction="ignore",
            lineterminator="\n",
        )
        writer.writeheader()
        for row in rows:
            writer.writerow({key: row.get(key, "") for key in fieldnames})


def read(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def feature_id(number: int) -> str:
    return f"COMFY-DESKTOP-{number:03d}"


def feature_identity(domain: str, name: str, source: str) -> str:
    return "\x1f".join((domain, name, source))


def load_feature_id_map() -> dict[str, str]:
    map_path = OUT / "desktop-feature-id-map.json"
    if map_path.exists():
        value = json.loads(map_path.read_text(encoding="utf-8"))
        if not isinstance(value, dict) or not all(isinstance(key, str) and isinstance(identifier, str) for key, identifier in value.items()):
            raise RuntimeError("desktop-feature-id-map.json is not a string-to-string map")
        return value
    catalog_path = OUT / "desktop-features.csv"
    if not catalog_path.exists():
        return {}
    with catalog_path.open(newline="", encoding="utf-8") as handle:
        return {
            feature_identity(row["domain"], row["name"], row["source_evidence"]): row["feature_id"]
            for row in csv.DictReader(handle)
        }


def new_feature_id(identity: str) -> str:
    return f"COMFY-DESKTOP-{hashlib.sha256(identity.encode('utf-8')).hexdigest()[:12].upper()}"


FEATURE_GROUPS: list[tuple[str, list[tuple[str, str, str, str, str]]]] = [
    ("source-and-installation", [
        ("Six-source runtime registry", "active", "src/main/sources/index.ts", "src/main/sources/index.test.ts", "The chooser enumerates standalone, portable, git, cloud, remote, and legacy-desktop source plugins with their visibility rules."),
        ("Managed standalone source", "active", "src/main/sources/standalone/index.ts", "src/main/sources/standalone/index.test.ts", "A user can create and manage an isolated Python ComfyUI installation."),
        ("Portable source adoption", "platform-specific", "src/main/sources/portable.ts", "src/main/sources/portable.probe.test.ts", "On Windows development builds, a user can probe and track a portable ComfyUI distribution without installing it."),
        ("Git source plugin", "developer-only", "src/main/sources/git.ts", "src/main/lib/git.test.ts", "A hidden source can launch a source checkout while skipping the managed install workflow."),
        ("Remote URL source", "active", "src/main/sources/remote.ts", "src/main/sources/common/urlSource.test.ts", "A user can save and open an external ComfyUI HTTP(S) endpoint."),
        ("Comfy Cloud source", "cloud/paid", "src/main/sources/cloud.ts", "src/main/lib/cloudUrl.test.ts", "A seeded cloud entry opens the hosted Comfy endpoint subject to capacity and account-tier gates."),
        ("Legacy Desktop source", "deprecated/dead", "src/main/sources/desktop.ts", "src/main/sources/desktop.test.ts", "A hidden legacy v1 source exists only to discover and migrate an older Desktop installation."),
        ("Dependent source field options", "active", "src/main/lib/ipc/registerAppHandlers.ts", "src/renderer/src/views/InstallWizardModal.test.ts", "Changing source selections recomputes permitted field options and preserves only valid dependent values."),
        ("Installation record construction", "active", "src/main/lib/ipc/registerAppHandlers.ts", "src/main/installations.test.ts", "Validated wizard selections are converted to a durable typed installation record."),
        ("GPU discovery", "active", "src/main/lib/gpu.ts", "src/main/lib/gpu.test.ts", "The installer detects supported GPU adapters and reports a normalized recommendation."),
        ("Hardware compatibility validation", "active", "src/main/lib/hardwareTap.ts", "src/main/lib/hardwareTap.test.ts", "The selected install variant is checked against the host platform and detected hardware before installation."),
        ("NVIDIA driver validation", "platform-specific", "src/main/lib/gpu.ts", "src/main/lib/gpu.test.ts", "NVIDIA selections surface an absent or incompatible driver as an actionable warning rather than silently continuing."),
        ("Install-path validation", "active", "src/main/lib/ipc/registerAppHandlers.ts", "src/main/lib/paths.test.ts", "The wizard rejects unsafe, occupied, unwritable, or structurally invalid destination paths with specific issues."),
        ("Disk-space inspection", "active", "src/main/lib/disk.ts", "src/renderer/src/components/PathDiskInfo.test.ts", "The UI reports total and available bytes for a candidate destination and blocks insufficient-space operations."),
        ("Stable ComfyUI release selection", "conditional", "src/main/lib/comfyui-releases.ts", "src/main/lib/comfyui-releases.test.ts", "The release picker lists stable tags newest-first and degrades to an empty list when the network is unavailable."),
        ("Install variant selection", "active", "src/main/sources/standalone/index.ts", "src/renderer/src/lib/variants.test.ts", "The user chooses a platform/GPU-specific environment variant and receives variant-specific install steps."),
        ("Express installation", "active", "src/renderer/src/views/QuickInstallModal.vue", "src/renderer/src/views/InstallWizardModal.test.ts", "A recommended default configuration can be installed with one confirmation and is labeled express in telemetry."),
        ("Configured installation wizard", "active", "src/renderer/src/views/InstallWizardModal.vue", "src/renderer/src/views/InstallWizardModal.test.ts", "A multi-step wizard collects source, release, variant, path, name, shared-directory, and launch settings."),
        ("Existing-install probe", "active", "src/main/lib/ipc/registerInstallationHandlers.ts", "src/main/sources/standalone/install.probe.test.ts", "A selected directory is probed for one or more recognizable installations and reports precise incompatibilities."),
        ("Track existing installation", "active", "src/main/lib/ipc/registerInstallationHandlers.ts", "src/renderer/src/views/TrackModal.test.ts", "A probed external installation can be added without copying or reinstalling its contents."),
        ("Nested installation-root discovery", "active", "src/main/sources/common/nestedRoot.ts", "src/main/sources/common/nestedRoot.test.ts", "Probe follows supported nested-root layouts without escaping the chosen directory."),
        ("Multiple installation inventory", "active", "src/main/installations.ts", "src/main/installations.test.ts", "Multiple named installations persist independently and appear in chooser and picker views."),
        ("Installation reordering", "active", "src/main/lib/ipc/registerInstallationHandlers.ts", "src/renderer/src/composables/useInstallList.test.ts", "Drag or keyboard reordering persists a stable installation order."),
        ("Installation rename and configuration update", "active", "src/main/lib/actions.ts", "src/renderer/src/components/settings/ComfyUISettingsContent.test.ts", "Editable fields update the durable record and dependent runtime configuration with errors surfaced to the user."),
        ("Installation deletion", "active", "src/main/lib/delete.ts", "src/renderer/src/composables/useInstallContextMenu.test.ts", "Deletion requires destructive confirmation, stops active work, and distinguishes record-only from on-disk removal."),
        ("Installation copy", "active", "src/main/lib/copy.ts", "src/renderer/src/composables/useListAction.test.ts", "An installation can be copied to a validated destination with progress, cancellation, and a distinct tracked record."),
        ("Shared versus isolated model paths", "active", "src/main/sources/standalone/envPaths.ts", "src/main/sources/standalone/envPaths.test.ts", "Each installation independently chooses shared model storage while preserving isolation for its environment."),
        ("Shared versus isolated input/output paths", "active", "src/main/sources/standalone/envPaths.ts", "src/main/sources/standalone/envPaths.test.ts", "Each installation independently chooses shared media folders and receives matching Comfy configuration."),
        ("Manager configuration", "conditional", "src/main/lib/managerConfig.ts", "src/main/lib/managerConfig.test.ts", "The desktop writes compatible ComfyUI-Manager configuration and tolerates an absent manager."),
        ("Comfy command-line argument editor", "active", "src/main/lib/comfy-args.ts", "src/main/lib/comfy-args.test.ts", "Registered Comfy arguments are discovered, edited with typed widgets, validated, serialized, and passed to launch."),
        ("Environment-variable editor", "active", "src/renderer/src/components/EnvVarsEditor.vue", "src/main/sources/getTerminalEnv.test.ts", "Per-install environment variables persist and are merged into child processes without exposing reserved variables."),
        ("Model search path configuration", "active", "src/main/lib/models.ts", "src/main/lib/models.test.ts", "Primary and additional model directories are normalized into Comfy-compatible search-path configuration."),
        ("Input/output directory configuration", "active", "src/main/settings.ts", "src/main/settings.test.ts", "Global media roots persist and propagate into managed installations."),
        ("Installation size calculation", "active", "src/main/lib/ipc/registerAppHandlers.ts", "src/main/lib/ipc/installDirState.test.ts", "The UI can request and cancel a recursive installation-size scan."),
        ("Unique installation naming", "active", "src/main/lib/ipc/registerInstallationHandlers.ts", "src/main/installations.test.ts", "Name collisions are resolved deterministically before a record is created."),
    ]),
    ("onboarding-and-migration", [
        ("First-use cohort detection", "active", "src/main/lib/firstUseDetection.ts", "src/renderer/src/panel/useFirstUseChain.test.ts", "Startup distinguishes new, returning, cloud-only, and legacy-desktop cohorts from persisted installations."),
        ("Cloud-versus-local first-use choice", "active", "src/renderer/src/views/FirstUseTakeover.vue", "src/renderer/src/views/FirstUseTakeover.test.ts", "A new user chooses cloud or local setup in a keyboard-operable takeover before normal navigation."),
        ("First-use fork experiment", "experimental", "src/renderer/src/views/FirstUseTakeover.vue", "src/main/lib/experiments.test.ts", "The desktop-first-use-fork-default flag changes the default onboarding branch while recording exposure."),
        ("Terms acceptance", "active", "src/renderer/src/components/TermsModal.vue", "src/renderer/src/components/TermsModal.test.ts", "Installation cannot proceed until required terms are accepted; cancellation returns without creating an install."),
        ("Skip onboarding", "active", "src/main/menu.ts", "src/renderer/src/views/FirstUseTakeover.test.ts", "A post-consent menu action marks first use complete and dismisses the takeover."),
        ("Onboarding coachmark", "active", "src/main/popups/titleCoachmark.ts", "src/main/popups/titleCoachmark.test.ts", "A central-pill coachmark is shown once, can be dismissed, and persists dismissal state."),
        ("Legacy Desktop detection", "platform-specific", "src/main/lib/desktopDetect.ts", "src/main/lib/desktopDetect.test.ts", "Windows and macOS known locations are probed for a legacy Desktop footprint without modifying it."),
        ("Legacy Desktop adoption preview", "platform-specific", "src/main/lib/desktopAdopt.ts", "src/main/lib/desktopAdopt.test.ts", "The user sees detected paths and migration consequences before adoption begins."),
        ("Legacy Desktop adoption", "platform-specific", "src/main/lib/desktopAdopt.ts", "src/main/lib/desktopAdopt.test.ts", "Confirmed adoption creates a modern tracked installation and migrates supported settings/content with progress."),
        ("Adoption prompt handshake", "active", "src/main/lib/ipc/sessionActions/migrate.ts", "src/renderer/src/composables/useAdoptPromptBridge.test.ts", "Main waits for renderer acknowledgement and an explicit user response before destructive migration steps."),
        ("Standalone layout migration", "conditional", "src/main/lib/standaloneMigration.ts", "src/main/lib/migrate.test.ts", "Older managed directory layouts are detected and migrated idempotently."),
        ("Local-install migration preview", "conditional", "src/main/lib/localMigration.ts", "src/main/lib/snapshots.test.ts", "A source installation can be previewed as a migration snapshot before creating a replacement."),
        ("Snapshot-based new installation", "active", "src/main/lib/ipc/registerSnapshotHandlers.ts", "src/renderer/src/views/LoadSnapshotModal.test.ts", "A compatible snapshot file can seed a new installation after preview, version, name, and variant validation."),
        ("OEM workflow import", "conditional", "src/main/lib/oem.ts", "src/main/lib/oem.test.ts", "A signed/validated OEM manifest can import designated workflows once per manifest version."),
        ("Interrupted migration recovery", "conditional", "src/main/lib/opMarker.ts", "src/main/lib/opMarker.test.ts", "An operation marker survives process exit and startup offers deterministic recovery or cleanup."),
    ]),
    ("launch-process-and-lifecycle", [
        ("Startup auto-launch policy", "active", "src/main/index.ts", "src/main/lib/lastSession.test.ts", "Startup honors none, last, or a selected installation only after first-use completion."),
        ("Last-session restoration", "active", "src/main/lib/lastSession.ts", "src/main/lib/lastSession.test.ts", "Window/install bindings are restored from a durable session file and invalid entries fall back to the chooser."),
        ("Hidden restore reveal handshake", "active", "src/main/index.ts", "src/renderer/src/panel/useFirstUseChain.test.ts", "A restored window remains hidden until takeover or dashboard readiness, with a bounded fallback reveal."),
        ("Managed ComfyUI launch", "active", "src/main/lib/ipc/sessionActions/launch.ts", "src/main/lib/process.test.ts", "Desktop launches the selected Python environment with normalized arguments, directories, environment, and log capture."),
        ("Remote endpoint launch", "active", "src/main/sources/remote.ts", "src/main/sources/common/urlSource.test.ts", "A remote entry opens its validated endpoint without spawning a local inference process."),
        ("Cloud endpoint launch", "cloud/paid", "src/main/sources/cloud.ts", "src/main/lib/cloudUrl.test.ts", "A cloud entry resolves the correct authenticated web endpoint and applies capacity policy."),
        ("External legacy process launch", "deprecated/dead", "src/main/sources/desktop.ts", "src/main/sources/desktop.test.ts", "A legacy installation uses its native launcher/process contract when migration has not yet occurred."),
        ("Launch phase progress", "active", "src/main/lib/launchProgress.ts", "src/main/lib/launchProgress.test.ts", "The renderer receives ordered weighted phases and never regresses visible overall progress."),
        ("Launch cancellation", "active", "src/main/lib/ipc/registerSessionHandlers.ts", "src/renderer/src/composables/useActionGuard.test.ts", "Cancelling launch aborts downloads/subprocess waits, releases reservations, and returns the installation to stopped."),
        ("Launch boot timeout", "active", "src/main/lib/ipc/sessionActions/launch.ts", "src/main/lib/process.test.ts", "A local server that is not ready within 300 seconds fails with actionable retained logs and cleanup."),
        ("TCP readiness probe", "active", "src/main/lib/ipc/sessionActions/launch.ts", "src/main/lib/process.test.ts", "The web view attaches only after the selected port accepts connections."),
        ("Port reservation", "active", "src/main/lib/process.ts", "src/main/lib/process.test.ts", "A cross-process lock reserves the candidate port before launch and is released on all terminal paths."),
        ("Port-owner conflict inspection", "active", "src/main/lib/file-lock-info.ts", "src/main/lib/file-lock-info.test.ts", "A conflicting process is identified and displayed without killing it automatically."),
        ("Port conflict resolution", "active", "src/main/lib/ipc/registerSessionHandlers.ts", "src/renderer/src/composables/useDialogs.test.ts", "The user can cancel, kill the identified process, or choose another port; the choice is rechecked for races."),
        ("Automatic alternate-port selection", "active", "src/main/lib/ipc/sessionActions/launch.ts", "src/main/lib/process.test.ts", "When policy permits, launch retries up to three candidate ports and persists the chosen port."),
        ("Manager reboot loop", "conditional", "src/main/lib/ipc/sessionActions/launch.ts", "src/main/lib/managerConfig.test.ts", "A Manager-requested reboot is honored up to five times, then fails visibly instead of looping forever."),
        ("Instance launch state broadcast", "active", "src/main/lib/ipc/broadcast.ts", "src/main/lib/ipc/broadcast.test.ts", "All windows observe launching, started, failed, stopping, and stopped transitions and late windows hydrate current state."),
        ("Stop ComfyUI", "active", "src/main/lib/ipc/shared.ts", "src/renderer/src/composables/useStopAction.test.ts", "Stopping terminates the complete child-process tree and resolves only after state and port cleanup."),
        ("Restart ComfyUI", "active", "src/main/popups/titlePopup.ts", "src/main/popups/titlePopup.test.ts", "Restart confirms disruptive work, stops the old process, and relaunches the same installation."),
        ("Quit with active operations", "active", "src/main/index.ts", "src/renderer/src/composables/useActionGuard.test.ts", "Quit consults active overlays/downloads, requires confirmation for destructive cancellation, and supports aborting quit."),
        ("OS session-ending shutdown", "platform-specific", "src/main/index.ts", "src/main/lib/updater.test.ts", "OS shutdown suppresses update installation and terminates managed processes without interactive deadlock."),
        ("Child-process environment isolation", "active", "src/main/lib/terminal.ts", "src/main/sources/getTerminalEnv.test.ts", "Each process receives its installation-specific Python/Git paths and sanitized inherited environment."),
        ("Sensitive launch argument redaction", "active", "src/main/lib/logged-process.ts", "src/main/lib/logged-process.test.ts", "Secrets and tokens are redacted from displayed/logged command lines while still reaching the child."),
        ("Python environment management", "active", "src/main/lib/pythonEnv.ts", "src/main/lib/pip.test.ts", "Managed environments use the bundled Python and report dependency failures at the UI boundary."),
        ("Bundled Git/pygit2 operations", "active", "src/main/lib/git.ts", "src/main/lib/git.test.ts", "Repository clone/fetch/checkout operations use packaged tooling, progress reporting, and cancellable error propagation."),
    ]),
    ("window-host-and-navigation", [
        ("Multiple top-level windows", "active", "src/main/host/createHostWindow.ts", "src/main/host/createHostWindow.test.ts", "Users can open multiple chooser or installation windows with independent bounds and shared installation state."),
        ("Installation focus instead of duplicate", "active", "src/main/index.ts", "src/main/host/registry.test.ts", "Opening an installation focuses its existing host unless the source explicitly permits a duplicate."),
        ("Unique browser partition support", "conditional", "src/main/host/attach.ts", "src/main/host/registry.test.ts", "Installations requiring a unique session receive a fresh host rather than being attached to an incompatible shared partition."),
        ("In-place chooser-to-install attach", "active", "src/main/host/attach.ts", "src/main/host/createHostWindow.test.ts", "A chooser window can be claimed and converted into the launched installation without a visible window swap."),
        ("Attach preview rollback", "active", "src/main/host/attachHostPreview.ts", "src/main/host/createHostWindow.test.ts", "Cancelled or failed launches restore the chooser title-bar identity and release the claim."),
        ("Return to dashboard", "active", "src/main/host/detach.ts", "src/main/host/detach.test.ts", "An install-backed host can detach in place after stop/confirmation while retaining its bounds."),
        ("Close-window renderer consultation", "active", "src/main/host/createHostWindow.ts", "src/main/host/createHostWindow.test.ts", "Main acknowledges renderer-owned cancellation prompts before deciding whether the window may close."),
        ("Window bounds persistence", "active", "src/main/lib/windowState.ts", "src/main/host/createHostWindow.test.ts", "Install-backed window bounds persist across move/resize and are clamped onto available displays at restore."),
        ("Panel navigation stack", "active", "src/main/host/panelView.ts", "src/main/host/panelView.test.ts", "Settings, downloads, console, and root surfaces navigate through a bounded panel stack with close/back semantics."),
        ("Title-bar identity", "active", "src/renderer/src/comfyTitleBar/useTitleBarIdentity.ts", "src/renderer/src/comfyTitleBar/TitleBarApp.test.ts", "The custom title bar reflects chooser/install identity, source category, connection state, and current panel."),
        ("Instance picker", "active", "src/renderer/src/comfyTitlePopup/InstancePickerView.vue", "src/renderer/src/comfyTitlePopup/InstancePickerView.test.ts", "The central pill opens a live installation picker with focus, launch, new-window, manage, update, snapshot, and delete actions."),
        ("Deep-link navigation", "active", "src/renderer/src/composables/useDeepLinkRouter.ts", "src/renderer/src/composables/useDeepLinkRouter.test.ts", "Validated comfy:// links route to settings or installation actions without allowing arbitrary navigation."),
        ("Second-instance behavior", "active", "src/main/index.ts", "e2e/specs/window-management.spec.ts", "A packaged second launch focuses an existing window instead of creating a second application process."),
        ("macOS activate behavior", "platform-specific", "src/main/index.ts", "e2e/specs/window-management.spec.ts", "Dock activation raises existing windows or creates a chooser when none remain."),
        ("Window-all-closed behavior", "platform-specific", "src/main/index.ts", "src/main/host/registry.test.ts", "The app exits only when no managed sessions still require lifecycle ownership."),
    ]),
    ("terminal-logs-crash-and-diagnostics", [
        ("Per-install integrated terminal", "active", "src/main/lib/terminal.ts", "src/main/lib/terminal.test.ts", "Each installation owns a reusable PTY with renderer subscription, scrollback, input, and resize."),
        ("Terminal keyboard semantics", "platform-specific", "src/renderer/src/views/ConsoleModal.vue", "e2e/specs/terminal.spec.ts", "Copy, paste, and SIGINT follow macOS, Windows, and Linux terminal conventions."),
        ("Terminal restart", "active", "src/main/lib/terminal.ts", "src/main/lib/terminal.test.ts", "A dead or wedged shell can be killed and recreated with a fresh restore snapshot."),
        ("Terminal popout", "active", "src/main/lib/terminalPopoutWindow.ts", "src/main/lib/popoutWindows.test.ts", "Terminal output can move to a dedicated window without creating a second PTY."),
        ("Durable Comfy log buffer", "active", "src/main/lib/logsBroadcast.ts", "src/main/lib/bootPhaseBuffer.test.ts", "Per-install log output is retained, replayed to late subscribers, and bounded."),
        ("Logs popout", "active", "src/main/lib/logsPopoutWindow.ts", "src/main/lib/popoutWindows.test.ts", "A dedicated logs window subscribes to the same retained stream and cleans up on close."),
        ("Application log rotation", "active", "src/main/lib/logRotation.ts", "src/main/lib/logRotation.test.ts", "Desktop rotates application logs at bounded size/count and preserves recent diagnostics."),
        ("Comfy process log rotation", "active", "src/main/lib/logRotation.ts", "src/main/lib/logRotation.test.ts", "Per-install comfyui.log files rotate without losing the active stream."),
        ("Crash stderr-tail retention", "active", "src/main/lib/stderrTail.ts", "src/main/lib/stderrTail.test.ts", "A crash retains a bounded stderr tail and rehydrates it in windows opened after the event."),
        ("Exit-code diagnosis", "active", "src/main/lib/exitCodeInfo.ts", "src/main/lib/exitCodeInfo.test.ts", "Known POSIX signals and Windows NTSTATUS values are translated into actionable crash detail."),
        ("VC++ runtime diagnosis", "platform-specific", "src/main/lib/vcRuntimeAudit.ts", "src/main/lib/vcRuntimeAudit.test.ts", "Relevant Windows launch crashes include a Visual C++ runtime audit and remediation hint."),
        ("Renderer crash recovery", "active", "src/main/host/createHostWindow.ts", "src/main/host/createHostWindow.test.ts", "A crashed renderer is reported and reloaded while the managed backend continues when safe."),
        ("Failed navigation retry", "active", "src/main/host/createHostWindow.ts", "src/main/host/createHostWindow.test.ts", "A failed Comfy navigation retries after two seconds and remains visibly failed if the backend is unavailable."),
        ("Crash dump collection", "conditional", "src/main/lib/crashDumps.ts", "src/main/lib/crashDumps.test.ts", "Crash dump locations and recent dumps can be gathered for diagnostics without uploading them automatically."),
        ("System information diagnostics", "active", "src/main/lib/ipc/registerAppHandlers.ts", "src/main/lib/gpu.test.ts", "The diagnostics surface reports app, OS, CPU, memory, GPU, and relevant installation context."),
        ("Feedback link", "active", "src/renderer/src/composables/useSendFeedback.ts", "src/renderer/src/lib/supportUrl.test.ts", "Title-bar and menu feedback actions open a support URL containing non-secret diagnostic context."),
    ]),
    ("updates-snapshots-and-downloads", [
        ("Startup app-update check", "active", "src/main/lib/updater.ts", "src/main/lib/updater.test.ts", "Desktop checks for application updates at startup regardless of the legacy autoUpdate setting."),
        ("Periodic app-update checks", "active", "src/main/lib/updater.ts", "src/main/lib/updater.test.ts", "Desktop repeats update checks every ten minutes without overlapping checks."),
        ("Manual app-update check", "active", "src/main/lib/updater.ts", "src/main/lib/updater.test.ts", "The user can request a check and receives available, current, or failed state."),
        ("App-update download", "conditional", "src/main/lib/updater.ts", "src/main/lib/updater.test.ts", "A supported package downloads an update with progress, error state, and retry."),
        ("App-update installation", "conditional", "src/main/lib/updater.ts", "src/main/lib/updater.test.ts", "A downloaded update installs only after explicit/allowed relaunch and never during OS session shutdown."),
        ("Windows staged startup update", "platform-specific", "src/main/lib/updater.ts", "src/main/lib/updater.test.ts", "A downloaded Windows update may install at next startup behind a splash with loop-prevention settings."),
        ("Linux system-managed updates", "platform-specific", "src/main/lib/updater.ts", "src/main/lib/updater.test.ts", "DEB/system-managed installs report self-update unavailable; AppImage capability is detected separately."),
        ("Update title-bar pill", "active", "src/renderer/src/comfyTitleBar/useUpdatePills.ts", "src/renderer/src/comfyTitleBar/TitleBarApp.test.ts", "Available, downloading, and ready app-update states are visible and actionable in every host window."),
        ("ComfyUI release cache", "active", "src/main/lib/release-cache.ts", "src/main/lib/release-cache.test.ts", "Stable release metadata persists in schema version 1, is fresh for one hour, and refreshes every fifteen minutes."),
        ("Installation version check", "active", "src/main/sources/standalone/updateOrchestrator.ts", "src/main/sources/standalone/updateOrchestrator.test.ts", "Managed installs compare current, stable, and latest channels and expose an update only when newer."),
        ("Transactional ComfyUI update", "active", "src/main/sources/standalone/updateOrchestrator.ts", "src/main/sources/standalone/updateOrchestrator.integration.test.ts", "Core checkout and dependency changes form one visible operation and roll back when dependency sync fails."),
        ("Custom-node update", "conditional", "src/main/sources/standalone/actions.ts", "src/main/sources/standalone/actions.integration.test.ts", "Installed custom nodes can be updated with progress and per-node failure reporting."),
        ("Pre-update snapshot", "active", "src/main/sources/standalone/updateOrchestrator.ts", "src/main/sources/standalone/updateOrchestrator.test.ts", "A snapshot is created before a managed update and remains available after success or rollback."),
        ("Interrupted update recovery", "active", "src/main/lib/opMarker.ts", "src/main/lib/opMarker.test.ts", "Startup recognizes an unfinished update marker and offers deterministic repair/rollback."),
        ("Manual snapshot creation", "active", "src/main/lib/snapshots/store.ts", "src/main/lib/snapshots/store.test.ts", "A user can create a version-1 snapshot of core revision, packages, and custom nodes."),
        ("Automatic boot/restart snapshots", "active", "src/main/lib/snapshots/store.ts", "src/main/lib/snapshots/store.test.ts", "Lifecycle-triggered snapshots are labeled boot or restart and bounded by retention policy."),
        ("Snapshot list and detail", "active", "src/main/lib/snapshots/tabData.ts", "src/main/lib/snapshots.test.ts", "The UI lists snapshots and shows exact package/repository metadata for a selected file."),
        ("Snapshot diff", "active", "src/main/lib/snapshots/diff.ts", "src/main/lib/snapshots.test.ts", "A snapshot compares against previous or current state with added, removed, and changed entries."),
        ("Snapshot restore", "active", "src/main/lib/snapshots/restore.ts", "src/main/lib/snapshots.test.ts", "Restore requires confirmation, stops active Comfy, applies revisions/dependencies, reports partial failures, and creates a post-restore snapshot."),
        ("Snapshot deletion", "active", "src/main/lib/snapshots/store.ts", "src/main/lib/snapshots/store.test.ts", "A selected snapshot is deleted only after destructive confirmation."),
        ("Single snapshot export", "active", "src/main/lib/snapshots/exportImport.ts", "src/main/lib/snapshots.test.ts", "A snapshot exports as a versioned comfyui-desktop-2 envelope through an OS save dialog."),
        ("All-snapshot export", "active", "src/main/lib/snapshots/exportImport.ts", "src/main/lib/snapshots.test.ts", "All snapshots export as a versioned multi-snapshot envelope without overwriting silently."),
        ("Snapshot import preview", "active", "src/main/lib/snapshots/exportImport.ts", "src/main/lib/snapshots.test.ts", "File chooser or drag-and-drop validates type/version and previews contents before import."),
        ("Snapshot import conflict diff", "active", "src/main/lib/snapshots/exportImport.ts", "src/main/lib/snapshots.test.ts", "Existing filenames/content are compared and conflicts are shown before confirmation."),
        ("Snapshot import confirmation", "active", "src/main/lib/snapshots/exportImport.ts", "src/main/lib/snapshots.test.ts", "Confirmed compatible snapshots import atomically and report imported count and restore candidate."),
        ("Model download request", "active", "src/main/lib/comfyDownloadManager.ts", "src/main/lib/comfyDownloadManager.test.ts", "A hosted Comfy page can request .safetensors, .sft, .ckpt, .pth, or .pt into an allowed model directory."),
        ("Asset download request", "active", "src/main/lib/comfyDownloadManager.ts", "src/main/lib/comfyDownloadManager.test.ts", "A hosted page can download a sanitized asset path inside an allowed input/output root."),
        ("Download path containment", "active", "src/main/lib/comfyDownloadManager.ts", "src/main/lib/comfyDownloadManager.test.ts", "Traversal, unsupported extensions, and destinations outside configured roots are rejected before network access."),
        ("Download filename collision handling", "active", "src/main/lib/comfyDownloadManager.ts", "src/main/lib/comfyDownloadManager.test.ts", "Existing names receive deterministic unique suffixes and partial files never replace good content."),
        ("Download progress states", "active", "src/main/lib/comfyDownloadManager.ts", "src/main/lib/comfyDownloadManager.test.ts", "Downloads transition through pending, downloading, paused, completed, error, or cancelled with bytes, rate, and ETA."),
        ("Pause and resume download", "active", "src/main/lib/comfyDownloadManager.ts", "src/main/lib/comfyDownloadManager.test.ts", "Active downloads pause without losing partial data and resume from a supported byte offset."),
        ("Cancel download", "active", "src/main/lib/comfyDownloadManager.ts", "src/main/lib/comfyDownloadManager.test.ts", "Cancellation aborts network work, removes temporary artifacts, and leaves a dismissible terminal record."),
        ("Retry failed download", "active", "src/main/lib/comfyDownloadManager.ts", "src/main/lib/comfyDownloadManager.test.ts", "Retry reuses the original validated request and returns to pending/downloading state."),
        ("Downloads tray", "active", "src/renderer/src/comfyTitlePopup/DownloadsView.vue", "src/renderer/src/comfyTitlePopup/DownloadsView.test.ts", "The title bar shows active and ten recent downloads with per-state actions and clear-finished."),
        ("Downloads full view", "active", "src/renderer/src/comfyTitlePopup/DownloadsFullView.vue", "src/renderer/src/comfyTitlePopup/DownloadsView.test.ts", "A persistent modal monitors large downloads without disappearing when title-bar focus changes."),
        ("Download image thumbnails", "active", "src/main/lib/ipc/registerDownloadHandlers.ts", "src/renderer/src/composables/useThumbnailPrefetch.test.ts", "Completed image assets provide bounded 64-pixel data-URL thumbnails; unreadable/non-images return null."),
        ("Taskbar download progress", "platform-specific", "src/main/lib/comfyDownloadManager.ts", "src/main/lib/comfyDownloadManager.test.ts", "Supported platforms aggregate active downloads into native taskbar/dock progress."),
        ("Starter-template downloads", "conditional", "src/main/sources/standalone/templateDownloadTask.ts", "src/main/sources/standalone/templateDownloadCore.test.ts", "Curated starter templates download required models/assets with pool size three, two retries, and disk headroom checks."),
        ("Skip starter-template wait", "active", "src/main/lib/ipc/registerInstallationHandlers.ts", "src/main/sources/standalone/templateDownloadGate.test.ts", "The user can finish installation while template downloads continue in the global tray without restarting."),
        ("Download authentication-token isolation", "active", "src/main/lib/downloadAttribution.ts", "src/main/lib/downloadAttribution.test.ts", "Attribution/auth tokens are stored separately, passed only to the request, and never broadcast to renderers or logs."),
    ]),
    ("settings-persistence-cloud-security-and-ui", [
        ("Atomic global settings persistence", "active", "src/main/settings.ts", "src/main/settings.test.ts", "settings.json writes through temporary and backup files and recovers from a damaged primary."),
        ("Atomic installation persistence", "active", "src/main/installations.ts", "src/main/installations.test.ts", "installations.json writes atomically, retains a backup, and migrates legacy fields on load."),
        ("Settings section registry", "active", "src/main/lib/ipc/registerSettingsHandlers.ts", "src/renderer/src/comfyTitlePopup/GlobalSettingsView.test.ts", "General, telemetry, update, cache, advanced, shared-directory, and install-location fields expose typed defaults and validation."),
        ("Theme setting", "active", "src/main/lib/theme.ts", "src/renderer/src/composables/useTheme.test.ts", "Light, dark, or system theme resolves centrally and updates host, popups, panel, and hosted Comfy surfaces."),
        ("Locale setting", "active", "src/main/lib/i18n.ts", "src/renderer/src/lib/localeCoverage.test.ts", "English and Chinese catalogs expose identical 1,176-key shapes and locale changes apply across desktop surfaces."),
        ("Model-directory settings", "active", "src/main/settings.ts", "src/main/lib/models.test.ts", "One primary and additional model roots persist, are normalized, and reject invalid duplicates."),
        ("Cache settings", "active", "src/main/settings.ts", "src/main/settings.test.ts", "The cache root and retained-download count persist and feed cache cleanup policy."),
        ("Close-confirmation setting", "active", "src/main/settings.ts", "src/main/host/createHostWindow.test.ts", "A configurable confirmation gates closing a window and supports cancel without stopping its session."),
        ("Auto-update settings", "active", "src/main/settings.ts", "src/main/lib/updater.test.ts", "Auto-download/install preferences affect update actions while checks continue independently."),
        ("Launch-on-startup setting", "active", "src/main/settings.ts", "src/main/lib/lastSession.test.ts", "None, last, or installation-ID startup policy persists and invalid IDs degrade to the chooser."),
        ("Chinese mirror settings", "conditional", "src/main/settings.ts", "src/main/lib/github-mirror.test.ts", "PyPI/GitHub mirror choices and first-use prompt state persist and affect only supported downloads."),
        ("Browser window-state persistence", "active", "src/main/lib/windowState.ts", "src/main/host/createHostWindow.test.ts", "Window bounds and display placement persist without storing transient popup geometry."),
        ("Last-session persistence", "active", "src/main/lib/lastSession.ts", "src/main/lib/lastSession.test.ts", "Last host/install bindings survive clean restart and malformed data is ignored safely."),
        ("Windows data-location marker", "platform-specific", "src/main/lib/paths.ts", "src/main/lib/paths.test.ts", "New installs on the system drive use LOCALAPPDATA, non-system drive installs colocate data, and a marker preserves the choice."),
        ("Linux XDG migration", "platform-specific", "src/main/lib/paths.ts", "src/main/lib/paths.test.ts", "Existing Linux userData state migrates once into XDG config/data/cache locations."),
        ("Firebase desktop authentication bridge", "cloud/paid", "src/main/auth/firebaseBridge/index.ts", "src/main/auth/firebaseBridge/server.test.ts", "Google/GitHub sign-in returns through a loopback bridge and injects only validated Firebase state into the shared partition."),
        ("Authentication callback validation", "cloud/paid", "src/main/auth/firebaseBridge/intercept.ts", "src/main/auth/firebaseBridge/intercept.test.ts", "Only exact configured HTTPS handler hosts/paths and supported providers enter the auth bridge."),
        ("Authentication loopback hardening", "cloud/paid", "src/main/auth/firebaseBridge/server.ts", "src/main/auth/firebaseBridge/server.test.ts", "The callback binds 127.0.0.1, caps request bodies at 64 KiB, uses no-store responses, and rejects invalid state."),
        ("Cloud capacity feature flag", "cloud/paid", "src/main/lib/cloudCapacity.ts", "src/renderer/src/composables/useCloudCapacity.test.ts", "normal, degraded, or disabled capacity changes cloud launch confirmation/blocking; paid users relax disabled to degraded."),
        ("Cloud user-tier cache", "cloud/paid", "src/main/lib/userTier.ts", "src/main/lib/userTier.test.ts", "Free, paid, or unknown tier persists in a bounded cache and is refreshed from authenticated state."),
        ("Telemetry consent", "active", "src/main/lib/telemetry.ts", "src/main/lib/telemetry.test.ts", "Unknown, enabled, and disabled consent states gate event emission and can be changed later."),
        ("Telemetry sanitization", "active", "src/main/lib/telemetry.ts", "src/main/lib/telemetry.test.ts", "Tokens, credentials, user paths, and high-risk free text are removed or normalized before transmission."),
        ("Telemetry rate limiting", "active", "src/main/lib/telemetry.ts", "src/main/lib/telemetry.test.ts", "Identical events are bounded to 60 per minute and the session is bounded to 5,000 events."),
        ("Experiment flag cache and exposure", "experimental", "src/main/lib/experiments.ts", "src/main/lib/experiments.test.ts", "Experiment variants cache locally, use deterministic fallback, and record exposure once per context."),
        ("Comfy feature-flag negotiation", "conditional", "src/main/lib/comfy-feature-flags.ts", "src/main/lib/comfy-feature-flags.test.ts", "Desktop queries python main.py --list-feature-flags and injects only recognized flags into launch arguments."),
        ("Navigation allowlist", "active", "src/main/host/createHostWindow.ts", "src/main/lib/allowedPopups.test.ts", "Untrusted navigations are denied or opened externally; only explicitly allowed auth/checkout popup origins may create children."),
        ("Checkout popup isolation", "cloud/paid", "src/main/host/createHostWindow.ts", "src/main/lib/allowedPopups.test.ts", "Checkout runs in a styled child with backdrop, Escape/close handling, and spoof-safe comfy.org return detection."),
        ("Context-isolated preload bridges", "active", "src/preload/index.ts", "src/preload/index.test.ts", "Renderers receive narrow validated APIs with nodeIntegration disabled and contextIsolation enabled."),
        ("Title-bar popup bridge", "active", "src/preload/comfyTitlePopupPreload.ts", "src/main/popups/titlePopup.test.ts", "One reused popup bridge validates menu, picker, downloads, settings, and lifecycle messages before main receives them."),
        ("System confirmation modal", "active", "src/main/popups/systemModal.ts", "src/renderer/src/components/ui/BasePrompt.test.ts", "Shell-level confirms dim the full host, expose primary/secondary/danger actions, trap focus, and close on Escape as cancellation."),
        ("Native application menus", "platform-specific", "src/main/menu.ts", "src/main/menu.test.ts", "macOS and non-macOS menus expose platform-standard app/edit/window/view actions and development-only reload tools."),
        ("Custom title-bar menu", "active", "src/main/popups/titlePopup.ts", "src/main/popups/titlePopup.test.ts", "The file menu exposes dashboard/new window/new instance/load snapshot/settings/feedback/zoom/close/quit based on current state."),
        ("Context menu", "active", "src/main/lib/contextMenu.ts", "src/main/lib/contextMenu.test.ts", "Link, image, and editable contexts expose only applicable open/copy/save/cut/paste/select actions."),
        ("Comfy-view reload and zoom shortcuts", "active", "src/main/host/createHostWindow.ts", "src/main/host/createHostWindow.test.ts", "F5/CmdOrCtrl+R reload; CmdOrCtrl plus, minus, and zero adjust/reset zoom; CmdOrCtrl+W is intercepted by host close policy."),
        ("Keyboard-accessible menus and lists", "active", "src/renderer/src/components/ui/BaseMenu.vue", "src/renderer/src/components/ui/BaseActionSheet.test.ts", "Arrow keys move roving focus, Enter/Space activate, Escape closes, and tab order remains inside open modal surfaces."),
        ("Keyboard-accessible card grids", "active", "src/renderer/src/components/VariantCardGrid.vue", "src/renderer/src/components/TemplatePickerStep.test.ts", "Arrow keys navigate variant/template cards and Enter/Space select the focused card."),
        ("Snapshot drag and drop", "active", "src/renderer/src/views/LoadSnapshotModal.vue", "src/renderer/src/views/LoadSnapshotModal.test.ts", "Dropping one JSON snapshot previews it; invalid, multiple, or non-file drops show validation errors."),
        ("Zoom status banner", "active", "src/renderer/src/components/ZoomBanner.vue", "src/main/lib/zoom.test.ts", "Non-default zoom is announced visually with a reset action and synchronized title-menu state."),
        ("Update and download status indicators", "active", "src/renderer/src/comfyTitleBar/useUpdatePills.ts", "src/renderer/src/comfyTitleBar/TitleBarApp.test.ts", "The title bar communicates installation update, app update, active download, and failure states without relying on color alone."),
        ("Native file/folder choosers", "active", "src/main/lib/ipc/registerAppHandlers.ts", "e2e/specs/dialogs.spec.ts", "Open/save/folder choices use native dialogs, return cancellation explicitly, and remember the last save directory where specified."),
        ("External path reveal", "active", "src/main/lib/ipc/registerAppHandlers.ts", "src/main/lib/contextMenu.test.ts", "Open path, reveal file, and show download in folder validate local targets and surface OS failures."),
        ("Developer update shortcuts", "developer-only", "src/main/lib/devShortcuts.ts", "src/main/lib/devShortcuts.test.ts", "Development builds register CmdOrCtrl+Alt+U/I to cycle application/installation update states for UI testing."),
        ("E2E control hooks", "developer-only", "src/main/lib/e2eHooks.ts", "e2e/fixtures.ts", "Test builds expose deterministic dialogs, state overrides, and renderer hooks without enabling them in production."),
        ("Dead tray menu", "deprecated/dead", "src/main/index.ts", "src/main/menu.test.ts", "Show App and Quit tray actions remain implemented but tray creation is disabled; legacy tray close preference is sanitized away."),
        ("Orphan draggable-list utility", "infrastructure-only", "src/renderer/src/lib/draggableList.ts", "src/renderer/src/lib/draggableList.test.ts", "A tested pointer drag-list helper has no production importer and is not treated as an active user capability."),
    ]),
    ("platform-packaging-and-recovery", [
        ("Windows NSIS installation", "platform-specific", "electron-builder.yml", "scripts/installer.nsh", "Windows ships an assisted per-user installer with optional destination/desktop shortcut and current/all-users switches."),
        ("Windows VC++ redistributable bootstrap", "platform-specific", "scripts/installer.nsh", "src/main/lib/vcRuntimeAudit.test.ts", "The installer runs the VC++ redistributable with UAC retry and explicit retry/ignore/abort handling."),
        ("macOS DMG and ZIP packaging", "platform-specific", "electron-builder.yml", "scripts/notarize.js", "macOS builds signed/notarized DMG and ZIP artifacts with the arm64 bootstrap payload."),
        ("Linux AppImage packaging", "platform-specific", "electron-builder.yml", "scripts/after-install.sh", "Linux x64 ships an AppImage with bundled bootstrap resources and AppImage-aware update capability."),
        ("Linux Debian packaging", "platform-specific", "electron-builder.yml", "scripts/after-install.sh", "The DEB installs/removes desktop integration and treats application updates as system-managed."),
        ("Linux AppArmor integration", "platform-specific", "scripts/after-install.sh", "scripts/after-remove.sh", "Ubuntu systems receive/remove a scoped AppArmor profile needed for Chromium user namespaces."),
        ("Platform-specific data roots", "platform-specific", "src/main/lib/paths.ts", "src/main/lib/paths.test.ts", "Windows, macOS, and Linux resolve configuration, data, cache, logs, models, input, and output roots according to platform policy."),
        ("Single-instance lock", "active", "src/main/index.ts", "e2e/specs/window-management.spec.ts", "Packaged launches acquire a single-instance lock; update relaunch preserves the lock handoff semantics."),
        ("Startup cache recovery", "active", "src/main/lib/safe-file.ts", "src/main/settings.test.ts", "Missing/corrupt atomic JSON files recover from backup or defaults with a visible diagnostic instead of crashing startup."),
        ("Partial download recovery", "active", "src/main/lib/download.ts", "src/main/lib/download.test.ts", "Sidecar metadata and partial content permit safe resume or cleanup after process interruption."),
        ("Offline launch behavior", "conditional", "src/main/lib/fetch.ts", "src/main/lib/fetch.test.ts", "Network-dependent release/cloud/template checks fail independently while an already-installed local Comfy remains launchable."),
        ("Mirror fallback", "conditional", "src/main/lib/r2Mirror.ts", "src/main/lib/r2Mirror.test.ts", "Configured mirrors fall back to canonical sources with bounded retries and preserve the final causal error."),
        ("Relaunch fallback page", "active", "src/main/lib/relaunchPage.ts", "src/main/lib/relaunchPage.test.ts", "When relaunch cannot proceed immediately, a local explanatory page offers retry/quit without remote script execution."),
        ("Download/operation cleanup on quit", "active", "src/main/index.ts", "src/main/lib/comfyDownloadManager.test.ts", "Confirmed quit cancels active work, removes temporary files/locks, stops process trees, and closes popouts."),
        ("Runtime observation unavailable in snapshot", "uncertain", "package.json", "", "No Electron runtime was launched in this read-only parity pass because the snapshot has no installed dependencies or bundled Node runtime on PATH."),
    ]),
]


def build_features() -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    feature_ids = load_feature_id_map()
    for domain, features in FEATURE_GROUPS:
        for name, availability, source, test, behavior in features:
            evidence_level = "test-backed" if test and (DESKTOP / test).exists() else "code-inferred"
            source_evidence = rel(DESKTOP / source)
            identity = feature_identity(domain, name, source_evidence)
            identifier = feature_ids.setdefault(identity, new_feature_id(identity))
            rows.append({
                "feature_id": identifier,
                "product": "Comfy-Desktop",
                "domain": domain,
                "name": name,
                "classification": "independently-testable-capability",
                "availability": availability,
                "evidence_level": evidence_level,
                "confidence": "high" if evidence_level == "test-backed" else "medium",
                "source_evidence": source_evidence,
                "test_evidence": f"{rel(DESKTOP / test)}" if test and (DESKTOP / test).exists() else "none located",
                "actor_trigger": "User, hosted Comfy surface, lifecycle event, or operating system invokes the named capability under the source-defined preconditions.",
                "observable_behavior": behavior,
                "failure_cancellation_recovery": "Preserve the source-defined validation, visible error, cancellation cleanup, retry, and restart behavior; unresolved edge detail remains an explicit test target.",
                "persistence_side_effects": "See desktop-persistence.csv, desktop-settings.csv, desktop-ipc.csv, and the cited source for exact durable and external effects.",
                "zed_status": "uncertain",
                "zed_evidence": "Current target evidence is maintained in native-zed-evidence.csv and master-feature-catalog.json; this row's Zed status is synchronized from the master native-architecture audit.",
                "parity_gap": "If Zed lacks the stated observable behavior and compatibility contract, implement it or record an explicit deferral decision.",
                "acceptance": f"With deterministic fixtures and the same preconditions, Zed shall reproduce: {behavior}",
                "validation": "Source contract/unit test plus side-by-side Zed protocol or GPUI interaction test; add failure injection when the capability performs I/O or process work.",
                "open_questions": "Runtime-only details are unverified unless separately marked observed by the lead audit.",
            })
    identifiers = [str(row["feature_id"]) for row in rows]
    if len(identifiers) != len(set(identifiers)):
        raise RuntimeError("desktop feature identity map contains colliding IDs")
    (OUT / "desktop-feature-id-map.json").write_text(
        json.dumps(dict(sorted(feature_ids.items())), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
    )
    return rows


FEATURES = build_features()


def map_feature(value: str) -> str:
    value = value.lower()
    rules = [
        (("snapshot",), 121), (("app-update",), 107), (("thumbnail",), 142),
        (("download-asset", "asset-download"), 133), (("download-model", "model-download"), 132), (("download",), 136),
        (("terminal",), 91), (("logs", "log-"), 95), (("telemetry",), 167),
        (("capacity",), 165), (("tier",), 166), (("firebase", "auth"), 162), (("cloud",), 6),
        (("first-use", "firstuse"), 36), (("experiment",), 170), (("feature-flag",), 171),
        (("locale",), 151), (("theme",), 150), (("setting",), 149),
        (("gpu", "hardware", "nvidia"), 10), (("source",), 1), (("install", "probe"), 22),
        (("launch", "running", "stopping", "comfy-start", "stop-comfy", "port"), 54),
        (("panel",), 84), (("modal",), 176), (("title", "popup"), 175), (("menu",), 177),
        (("zoom",), 180), (("window", "host"), 76), (("crash", "exit"), 99),
        (("update",), 107), (("relaunch", "quit"), 70), (("open", "browse", "path"), 187),
    ]
    for needles, number in rules:
        if any(needle in value for needle in needles):
            return feature_id(number)
    return feature_id(174)


def extract_ipc() -> None:
    usages: dict[str, list[tuple[str, str, int]]] = defaultdict(list)
    patterns = [
        ("main-handle", re.compile(r"ipcMain\.(?:handle|handleOnce)\(\s*['\"]([^'\"]+)['\"]")),
        ("main-on", re.compile(r"ipcMain\.on\(\s*['\"]([^'\"]+)['\"]")),
        ("renderer-invoke", re.compile(r"ipcRenderer\.invoke\(\s*['\"]([^'\"]+)['\"]")),
        ("renderer-send", re.compile(r"ipcRenderer\.(?:send|sendSync)\(\s*['\"]([^'\"]+)['\"]")),
        ("renderer-listen", re.compile(r"ipcRenderer\.(?:on|once|removeListener)\(\s*['\"]([^'\"]+)['\"]")),
        ("main-send", re.compile(r"(?:webContents|sender|event\.sender)\.send\(\s*['\"]([^'\"]+)['\"]")),
    ]
    for path in sorted((DESKTOP / "src").rglob("*.ts")):
        if ".test." in path.name:
            continue
        text = read(path)
        for kind, pattern in patterns:
            for match in pattern.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                usages[match.group(1)].append((kind, rel(path), line))
    ipc_types = read(DESKTOP / "src/types/ipc.ts")
    match = re.search(r"export const PICKER_SETTINGS_CHANNELS\s*=\s*\{(.*?)\}\s*as const", ipc_types, re.S)
    picker_channels: dict[str, str] = {}
    if match:
        base_line = ipc_types.count("\n", 0, match.start()) + 1
        for key, value in re.findall(r"(\w+)\s*:\s*['\"]([^'\"]+)['\"]", match.group(1)):
            picker_channels[key] = value
            usages[value].append(("constant-mediated", rel(DESKTOP / "src/types/ipc.ts"), base_line))
    for path in [DESKTOP / "src/main/popups/pickerSettingsHandlers.ts", DESKTOP / "src/preload/comfyTitlePopupPreload.ts"]:
        text = read(path)
        for match in re.finditer(r"ipcMain\.(handle|on|handleOnce)\(\s*CH\.(\w+)", text, re.S):
            value = picker_channels.get(match.group(2))
            if value:
                kind = "main-handle" if match.group(1) != "on" else "main-on"
                usages[value].append((kind, rel(path), text.count("\n", 0, match.start()) + 1))
        for match in re.finditer(r"ipcRenderer\.(invoke|send|sendSync|on|once)\(\s*CH\.(\w+)", text, re.S):
            value = picker_channels.get(match.group(2))
            if value:
                operation = match.group(1)
                kind = "renderer-invoke" if operation == "invoke" else "renderer-listen" if operation in {"on", "once"} else "renderer-send"
                usages[value].append((kind, rel(path), text.count("\n", 0, match.start()) + 1))
    production_main = [path for path in (DESKTOP / "src/main").rglob("*.ts") if ".test." not in path.name]
    for channel in list(usages):
        quoted = re.compile(rf"['\"]{re.escape(channel)}['\"]")
        existing_paths = {(kind, path, line) for kind, path, line in usages[channel]}
        for path in production_main:
            text = read(path)
            for match in quoted.finditer(text):
                line = text.count("\n", 0, match.start()) + 1
                if not any(existing_path == rel(path) and existing_line == line for _, existing_path, existing_line in existing_paths):
                    usages[channel].append(("main-reference", rel(path), line))
    rows = []
    for channel, channel_usages in sorted(usages.items()):
        kinds = sorted({item[0] for item in channel_usages})
        if "main-handle" in kinds or "renderer-invoke" in kinds:
            mechanism = "request-response"
        elif "main-on" in kinds or "renderer-send" in kinds:
            mechanism = "one-way renderer-to-main"
        else:
            mechanism = "main-to-renderer event"
        rows.append({
            "channel": channel,
            "mechanism": mechanism,
            "directions_observed": ";".join(kinds),
            "feature_id": map_feature(channel),
            "availability": "developer-only" if "e2e" in channel or "dev" in channel else "active",
            "registration_and_use": "; ".join(f"{path}:{line} ({kind})" for kind, path, line in channel_usages),
            "payload_result_schema": "Typed by src/types/ipc.ts or the corresponding preload/handler; preserve rejection and event-unsubscribe behavior.",
            "security_validation": "Renderer arguments are narrowed by context-isolated preload and revalidated by main for paths, URLs, IDs, actions, and privileged operations where applicable.",
            "evidence_level": "code-inferred",
            "notes": "Computed channels are marked constant-mediated; dynamic callback pairing is described in desktop-preload-apis.csv.",
        })
    write_csv("desktop-ipc.csv", ["channel", "mechanism", "directions_observed", "feature_id", "availability", "registration_and_use", "payload_result_schema", "security_validation", "evidence_level", "notes"], rows)


def interface_body(text: str, name: str) -> tuple[str, int]:
    match = re.search(rf"export interface {re.escape(name)}\s*\{{", text)
    if not match:
        return "", 0
    depth = 1
    cursor = match.end()
    while cursor < len(text) and depth:
        if text[cursor] == "{":
            depth += 1
        elif text[cursor] == "}":
            depth -= 1
        cursor += 1
    return text[match.end():cursor - 1], text.count("\n", 0, match.start()) + 1


def extract_preload_apis() -> None:
    interfaces = [
        ("window.api", DESKTOP / "src/types/ipc.ts", "ElectronApi", "panel/chooser renderer"),
        ("window.__comfyDesktop2", DESKTOP / "src/types/comfyDesktopBridge.ts", "ComfyDesktop2Bridge", "hosted Comfy renderer"),
        ("window.__comfyDesktop2.Terminal", DESKTOP / "src/types/comfyDesktopBridge.ts", "ComfyDesktop2TerminalBridge", "hosted Comfy terminal integration"),
        ("window.__comfyDesktop2.Logs", DESKTOP / "src/types/comfyDesktopBridge.ts", "ComfyDesktop2LogsBridge", "hosted Comfy logs integration"),
        ("window.__comfyDesktop2.Telemetry", DESKTOP / "src/types/comfyDesktopBridge.ts", "ComfyDesktop2TelemetryBridge", "hosted Comfy telemetry integration"),
        ("window.__comfyTitleBar", DESKTOP / "src/preload/comfyTitleBarPreload.ts", "ComfyTitleBarBridge", "title bar"),
        ("window.__comfyTitlePopup", DESKTOP / "src/preload/comfyTitlePopupPreload.ts", "ComfyTitlePopupBridge", "title popup"),
        ("window.__comfySystemModal", DESKTOP / "src/preload/comfySystemModalPreload.ts", "ComfySystemModalBridge", "system modal"),
        ("window.__comfyTitleTooltip", DESKTOP / "src/preload/comfyTitleTooltipPreload.ts", "ComfyTitleTooltipBridge", "title tooltip/coachmark"),
    ]
    rows = []
    for surface, path, interface, consumer in interfaces:
        text = read(path)
        body, interface_line = interface_body(text, interface)
        for match in re.finditer(r"(?m)^  ([A-Za-z_$][\w$]*)\??\s*(?:\(|:)", body):
            name = match.group(1)
            line = interface_line + body.count("\n", 0, match.start()) + 1
            rows.append({
                "surface": surface,
                "member": name,
                "interface": interface,
                "consumer": consumer,
                "feature_id": map_feature(name),
                "source": f"{rel(path)}:{line}",
                "contract": "Property/call/callback contract is the exact TypeScript member declaration at the cited source; callback members return an unsubscribe where typed.",
                "trust_boundary": "contextBridge exposure; no direct ipcRenderer or Node primitive is exposed.",
                "evidence_level": "code-inferred",
            })
    write_csv("desktop-preload-apis.csv", ["surface", "member", "interface", "consumer", "feature_id", "source", "contract", "trust_boundary", "evidence_level"], rows)


SETTINGS = [
    ("cacheDir", "platform cache path", "Storage", "active", "Cache root; changing it affects subsequent downloads."),
    ("maxCachedDownloads", "1", "Storage", "active", "Maximum retained completed download artifacts."),
    ("onAppClose", "quit", "General", "deprecated/dead", "Legacy tray value is sanitized to quit because tray creation is disabled."),
    ("modelsDirs", "shared models path", "Models", "active", "Ordered model roots; first entry is primary."),
    ("inputDir", "shared input path", "Shared directories", "active", "Default Comfy input root."),
    ("outputDir", "shared output path", "Shared directories", "active", "Default Comfy output root."),
    ("installDir", "platform installs path", "Installation", "active", "Default new-install parent."),
    ("language", "OS locale", "General", "active", "Desktop locale override."),
    ("theme", "system", "General", "active", "light/dark/system theme."),
    ("autoUpdate", "legacy", "Updates", "deprecated/dead", "Retained for migration; no longer gates update checks."),
    ("autoInstallUpdates", "true", "Updates", "active", "Automatically downloads/installs supported app updates."),
    ("autoLaunchOnStartup", "none", "General", "active", "none, last, or installation ID."),
    ("confirmBeforeClosingWindow", "false", "General", "active", "Ask before closing an application window."),
    ("pypiMirror", "canonical", "Advanced", "conditional", "Python package index override."),
    ("useChineseMirrors", "false", "Advanced", "conditional", "Enable regional mirrors."),
    ("chineseMirrorsPrompted", "false", "Internal", "infrastructure-only", "One-time regional mirror prompt marker."),
    ("telemetryEnabled", "unset", "Telemetry", "active", "Tri-state telemetry consent."),
    ("firstUseCompleted", "false", "Internal", "active", "Onboarding completion marker."),
    ("hideCloudFromPicker", "false", "Advanced", "conditional", "Suppress cloud entry in picker without deleting it."),
    ("oemManagedModelDirs", "[]", "Internal", "conditional", "OEM-controlled model roots."),
    ("oemWorkflowImportVersion", "unset", "Internal", "conditional", "Last imported OEM workflow manifest version."),
    ("lastSaveDialogDir", "unset", "Internal", "active", "Most recent snapshot/export save directory."),
    ("skipTemplatePickerStep", "false", "Installation", "active", "Skip starter-template selection during installation."),
    ("pendingDownloadedUpdateVersion", "unset", "Internal", "platform-specific", "Windows staged update version."),
    ("lastStartupUpdateAttemptVersion", "unset", "Internal", "platform-specific", "Windows startup update loop breaker."),
    ("installUpdatesOnStartup", "true on Windows", "Updates", "platform-specific", "Allow staged startup update."),
    ("showInstallerUI", "true on Windows", "Updates", "platform-specific", "Show installer UI for startup update."),
]

REMOVED_SETTINGS = [
    ("primaryInstallId", "discarded", "Legacy migration", "The loader removes the key from memory and rewrites settings.json without it; standalone swap-installations scripts still read it as a legacy compatibility input.", "projects/comfy/Comfy-Desktop/src/main/settings.test.ts:115-137", "test-backed"),
    ("pinnedInstallIds", "discarded", "Legacy migration", "The loader removes the obsolete pinned-install inventory from memory and rewrites settings.json without it.", "projects/comfy/Comfy-Desktop/src/main/settings.test.ts:115-137", "test-backed"),
    ("maxCachedFiles", "discarded; maxCachedDownloads defaults to 1", "Legacy migration", "The loader discards the predecessor value rather than translating it, then rewrites settings.json with the current maxCachedDownloads default.", "projects/comfy/Comfy-Desktop/src/main/settings.test.ts:141-155", "test-backed"),
    ("closeDirectlyOnLastWindow", "discarded", "Legacy migration", "The loader removes the obsolete last-window quit toggle; confirmBeforeClosingWindow is the current independent close guard and remains false by default.", "No focused existing test; executable removal loop at projects/comfy/Comfy-Desktop/src/main/settings.ts:285-299", "code-inferred"),
]


def write_settings() -> None:
    rows = []
    source = rel(DESKTOP / "src/main/settings.ts")
    for key, default, section, availability, behavior in SETTINGS:
        rows.append({"key": key, "default_or_fallback": default, "section": section, "availability": availability, "feature_id": map_feature(key), "persistence": "settings.json (atomic .tmp/.bak strategy)", "behavior": behavior, "source": source, "tests": rel(DESKTOP / "src/main/settings.test.ts"), "evidence_level": "test-backed"})
    for key, default, section, behavior, tests, evidence_level in REMOVED_SETTINGS:
        rows.append({"key": key, "default_or_fallback": default, "section": section, "availability": "deprecated/dead", "feature_id": feature_id(149), "persistence": "settings.json load-time removal followed by atomic rewrite", "behavior": behavior, "source": f"{source}:285-299", "tests": tests, "evidence_level": evidence_level})
    write_csv("desktop-settings.csv", ["key", "default_or_fallback", "section", "availability", "feature_id", "persistence", "behavior", "source", "tests", "evidence_level"], rows)


PERSISTENCE = [
    ("settings.json", "Global settings", "JSON plus .tmp/.bak", "src/main/settings.ts", "Atomic write, backup recovery, legacy key/value migration."),
    ("installations.json", "Installation inventory", "JSON plus .tmp/.bak", "src/main/installations.ts", "Atomic write; migrates useSharedPaths and seeds cloud entry."),
    ("window-state.json", "Window bounds", "JSON", "src/main/lib/windowState.ts", "Per-install/chooser bounds restored and clamped to displays."),
    ("last-session.json", "Last window/install session", "JSON", "src/main/lib/lastSession.ts", "Used by startup restore; malformed/unknown installs ignored."),
    ("data-location.json", "Windows data-root choice", "JSON", "src/main/lib/paths.ts", "Pins system-drive/local or colocated data mode."),
    ("release-cache.json", "ComfyUI release metadata", "schemaVersion 1 JSON", "src/main/lib/release-cache.ts", "One-hour freshness and periodic refresh."),
    ("fetch-cache.json", "HTTP metadata cache", "JSON", "src/main/lib/cache.ts", "Offline/freshness cache for bounded remote reads."),
    ("experiment-flags.json", "Experiment variants", "JSON", "src/main/lib/experiments.ts", "Cached flag values and exposure context."),
    ("cloud-user-tier.json", "Cloud tier", "JSON", "src/main/lib/userTier.ts", "free/paid/unknown cache."),
    ("device-id.txt", "Anonymous device identifier", "text", "src/main/lib/deviceId.ts", "Generated once; migration/alias guards prevent identity churn."),
    ("identity-migration-completed", "Identity migration completion guard", "ISO-8601 timestamp marker", "src/main/lib/deviceId.ts", "Written only after the legacy PostHog alias callback succeeds; suppresses future alias attempts. Best-effort write failure can retry the migration on a later boot."),
    ("first-launch-completed", "First-launch telemetry guard", "ISO-8601 timestamp marker", "src/main/lib/deviceId.ts", "consumeFirstLaunch returns true when absent and writes the marker. Best-effort write failure may over-count a later launch rather than lose the first-launch anchor."),
    ("pending-identity-alias.txt", "Telemetry identity migration", "text", "src/main/lib/deviceId.ts", "Deferred alias survives offline startup."),
    ("pending-download-token.txt", "Download attribution", "text", "src/main/lib/downloadAttribution.ts", "Secret is consumed without renderer broadcast."),
    ("download-token-attributed", "Attribution completion marker", "marker", "src/main/lib/downloadAttribution.ts", "Prevents repeated attribution."),
    ("shared_model_paths.yaml", "Shared model paths", "YAML", "src/main/lib/models.ts", "Desktop-owned model path source."),
    ("extra_model_paths.yaml", "Per-install Comfy model paths", "YAML", "src/main/lib/models.ts", "Comfy-compatible generated path mapping."),
    ("instance-model-paths/<installation-id>.yaml", "Isolated per-install model paths", "Comfy extra_model_paths YAML", "src/main/lib/models.ts", "Generated with safe-file replacement when an installation opts out of shared models; passed to ComfyUI at launch and removed best-effort when the installation is deleted."),
    ("port-locks/port-<port>.json", "Port reservations", "JSON { pid, installationName, timestamp }", "src/main/lib/process.ts", "Written best-effort when ComfyUI is spawned; a missing/dead PID makes reads remove the stale lock; explicit process teardown removes it best-effort."),
    ("cloud-entered-completed", "First cloud-entry guard", "ISO-8601 timestamp marker", "src/main/lib/cloudEntry.ts", "Written on the first cloud dom-ready entry and drives first_time plus has_launched_cloud. Best-effort write failure may over-count a later launch."),
    (".comfyui-desktop-2", "Installation marker", "marker", "src/main/sources/standalone/install.ts", "Identifies managed/adopted roots."),
    (".comfyui-op-in-progress.json", "Interrupted operation", "JSON", "src/main/lib/opMarker.ts", "Recovery/rollback prompt at next startup."),
    (".launcher/snapshots/*.json", "Installation snapshots", "Snapshot schema version 1", "src/main/lib/snapshots/types.ts", "manual/boot/restart/pre-update/post-update/post-restore state."),
    (".launcher/snapshots/manifest.json", "Snapshot index", "JSON", "src/main/lib/snapshots/store.ts", "Ordering/retention metadata."),
    ("snapshot export", "Portable snapshot envelope", "type comfyui-desktop-2-snapshot; version 1 JSON", "src/main/lib/snapshots/exportImport.ts", "Single or multi-snapshot import/export contract."),
    ("manifest.json", "Managed install manifest", "JSON", "src/main/sources/standalone/install.ts", "Release, variant, component, and layout metadata."),
    (".comfy_environment", "Environment marker", "text/marker", "src/main/sources/standalone/install.ts", "Identifies managed Python environment."),
    ("*.dl-meta", "Partial download metadata", "JSON sidecar", "src/main/lib/download.ts", "Resume validation and interrupted download cleanup."),
    ("comfyui.log", "Per-install runtime log", "rotating text", "src/main/lib/logRotation.ts", "Bounded retained process diagnostics."),
    ("app.log", "Desktop runtime log", "rotating text", "src/main/lib/appLog.ts", "Bounded application diagnostics."),
    ("OEM/manifest.json", "OEM policy", "version 1 JSON", "src/main/lib/oem.ts", "ProgramData-managed directories/workflow import on Windows."),
    ("localStorage:lastCheckedAt", "Update UI timestamp", "renderer localStorage", "src/renderer/src/comfyTitlePopup/GlobalSettingsView.vue", "Mirrored to main popup snapshot for display."),
    ("persist:shared", "Shared browser session", "Electron partition", "src/main/host/createHostWindow.ts", "Carries shared auth/cookies across compatible installs."),
    ("persist:<installation-id>", "Unique browser session", "Electron partition", "src/main/host/createHostWindow.ts", "Isolates sources that request unique browser state."),
    ("firebaseLocalStorageDb/firebaseLocalStorage", "Firebase auth", "IndexedDB", "src/main/auth/firebaseBridge/inject.ts", "Validated auth state injected into shared partition."),
    ("sessionStorage:__comfyDesktopPostSignin", "Post-sign-in handoff", "sessionStorage", "src/main/auth/firebaseBridge/inject.ts", "One-navigation auth completion marker."),
]


def write_persistence() -> None:
    rows = []
    for artifact, purpose, format_, source, lifecycle in PERSISTENCE:
        rows.append({"artifact": artifact, "purpose": purpose, "format_schema": format_, "feature_id": map_feature(artifact + purpose), "availability": "active", "source": rel(DESKTOP / source), "write_read_migration_recovery": lifecycle, "compatibility_contract": "Preserve filename/location semantics where external Comfy or upgrade compatibility depends on them; otherwise migrate atomically.", "evidence_level": "code-inferred"})
    write_csv("desktop-persistence.csv", ["artifact", "purpose", "format_schema", "feature_id", "availability", "source", "write_read_migration_recovery", "compatibility_contract", "evidence_level"], rows)


SOURCE_PLUGINS = [
    ("standalone", "Local", "active", "Windows/macOS/Linux", "false", "Managed isolated Python ComfyUI install/update."),
    ("portable", "Local", "platform-specific", "Windows", "hidden in packaged builds", "Adopt existing portable tree; skip managed install."),
    ("git", "Local", "developer-only", "Windows/macOS/Linux", "hidden", "Developer source checkout; skip managed install."),
    ("cloud", "Cloud", "cloud/paid", "Windows/macOS/Linux", "false unless hideCloudFromPicker", "Seeded Comfy Cloud endpoint with capacity/tier gating."),
    ("remote", "Remote", "active", "Windows/macOS/Linux", "false", "Externally managed ComfyUI URL."),
    ("desktop", "Legacy", "deprecated/dead", "Windows/macOS", "hidden", "Legacy Desktop v1 discovery/migration only."),
]


def write_static_surface_catalogs() -> None:
    source_rows = []
    source_feature_ids = {"standalone": 2, "portable": 3, "git": 4, "remote": 5, "cloud": 6, "desktop": 7}
    for identifier, category, availability, platforms, visibility, behavior in SOURCE_PLUGINS:
        source_rows.append({"source_id": identifier, "category": category, "availability": availability, "platforms": platforms, "visibility": visibility, "feature_id": feature_id(source_feature_ids[identifier]), "behavior": behavior, "registry_source": rel(DESKTOP / "src/main/sources/index.ts"), "implementation_source": rel(DESKTOP / f"src/main/sources/{identifier}.ts") if (DESKTOP / f"src/main/sources/{identifier}.ts").exists() else rel(DESKTOP / "src/main/sources/standalone/index.ts"), "evidence_level": "code-inferred"})
    write_csv("desktop-source-plugins.csv", ["source_id", "category", "availability", "platforms", "visibility", "feature_id", "behavior", "registry_source", "implementation_source", "evidence_level"], source_rows)

    menu_items = [
        ("title-popup", "Open Dashboard", "active", "normal mode", "Return/focus dashboard"),
        ("title-popup", "New Instance", "active", "normal mode", "Open install wizard"),
        ("title-popup", "Add Existing Instance", "active", "normal mode", "Open probe/track modal"),
        ("title-popup", "Load Snapshot", "active", "normal mode", "Open snapshot import/new-install flow"),
        ("title-popup", "Desktop Settings", "active", "normal mode", "Open global settings"),
        ("title-popup", "Send Beta Feedback", "active", "normal mode", "Open support URL"),
        ("title-popup", "Reset Zoom", "conditional", "non-default zoom", "Reset host zoom"),
        ("title-popup", "Close Window", "active", "normal mode", "Enter close consultation/confirmation"),
        ("title-popup", "Quit Desktop", "active", "normal mode", "Enter app quit guards"),
        ("title-popup", "Skip Onboarding", "conditional", "post-consent takeover", "Mark first use complete"),
        ("context", "Open Link in Browser", "conditional", "link target", "Open validated URL externally"),
        ("context", "Copy Link Address", "conditional", "link target", "Copy URL"),
        ("context", "Save Image", "conditional", "image target", "Save image via download path"),
        ("context", "Copy Image", "conditional", "image target", "Copy image"),
        ("context", "Cut", "conditional", "editable selection", "Native cut"),
        ("context", "Copy", "conditional", "selection", "Native copy"),
        ("context", "Paste", "conditional", "editable target", "Native paste"),
        ("context", "Select All", "conditional", "editable/selectable target", "Native select-all"),
        ("mac-app", "About", "platform-specific", "macOS", "Native about panel"),
        ("mac-app", "Check for Updates", "platform-specific", "macOS", "Manual app update check"),
        ("mac-app", "Services", "platform-specific", "macOS", "Native services submenu"),
        ("mac-app", "Hide", "platform-specific", "macOS", "Hide app"),
        ("mac-app", "Hide Others", "platform-specific", "macOS", "Hide other apps"),
        ("mac-app", "Show All", "platform-specific", "macOS", "Unhide windows"),
        ("mac-app", "Quit", "platform-specific", "macOS", "Quit with desktop guards"),
        ("mac-edit", "Undo", "platform-specific", "macOS", "Native undo role"),
        ("mac-edit", "Redo", "platform-specific", "macOS", "Native redo role"),
        ("mac-edit", "Cut", "platform-specific", "macOS", "Native cut role"),
        ("mac-edit", "Copy", "platform-specific", "macOS", "Native copy role"),
        ("mac-edit", "Paste", "platform-specific", "macOS", "Native paste role"),
        ("mac-edit", "Paste and Match Style", "platform-specific", "macOS", "Native paste-and-match-style role"),
        ("mac-edit", "Delete", "platform-specific", "macOS", "Native delete role"),
        ("mac-edit", "Select All", "platform-specific", "macOS", "Native select-all role"),
        ("mac-edit", "Start Speaking", "platform-specific", "macOS", "Native start-speaking role in the edit menu"),
        ("mac-edit", "Stop Speaking", "platform-specific", "macOS", "Native stop-speaking role in the edit menu"),
        ("mac-window", "Minimize", "platform-specific", "macOS", "Minimize window"),
        ("mac-window", "Zoom", "platform-specific", "macOS", "Native zoom window"),
        ("mac-window", "Toggle Full Screen", "platform-specific", "macOS", "Fullscreen host and relayout views"),
        ("mac-window", "Bring All to Front", "platform-specific", "macOS", "Raise all windows"),
        ("nonmac-view", "Toggle Full Screen", "platform-specific", "Windows/Linux", "Fullscreen host and relayout views"),
        ("development-view", "Reload", "developer-only", "development", "Reload focused renderer"),
        ("development-view", "Force Reload", "developer-only", "development", "Reload ignoring cache"),
        ("development-view", "Toggle Developer Tools", "developer-only", "development", "Open Chromium devtools"),
        ("tray", "Show App", "deprecated/dead", "tray disabled", "Would show existing host"),
        ("tray", "Quit", "deprecated/dead", "tray disabled", "Would quit desktop"),
    ]
    rows = [{"surface": surface, "action": action, "availability": availability, "condition": condition, "feature_id": map_feature(action), "observable_effect": effect, "source": rel(DESKTOP / ("src/main/lib/contextMenu.ts" if surface == "context" else "src/main/menu.ts" if "mac" in surface or "view" in surface or surface == "tray" else "src/main/popups/titlePopup.ts")), "evidence_level": "code-inferred"} for surface, action, availability, condition, effect in menu_items]
    write_csv("desktop-menu-actions.csv", ["surface", "action", "availability", "condition", "feature_id", "observable_effect", "source", "evidence_level"], rows)

    shell_actions = [
        ("title-bar", "Open file menu", "active", "Always", "Open the reused title popup with state-dependent file actions", 178, "src/renderer/src/comfyTitleBar/TitleBarApp.vue"),
        ("title-bar", "Refresh instance", "conditional", "Install-backed remote/cloud or refreshable view", "Reload or reconnect the active hosted instance", 103, "src/renderer/src/comfyTitleBar/TitleBarApp.vue"),
        ("title-bar", "Open instance picker", "active", "Central installation pill", "Open the live picker seeded with the active installation", 86, "src/renderer/src/comfyTitleBar/TitleBarApp.vue"),
        ("title-bar", "Open installation update", "conditional", "Update tag visible", "Open Update tab and initiate the installation update action", 116, "src/renderer/src/comfyTitleBar/TitleBarApp.vue"),
        ("title-bar", "Open app update", "conditional", "App update available/downloading/ready", "Open or invoke the action appropriate to the updater state", 114, "src/renderer/src/comfyTitleBar/TitleBarApp.vue"),
        ("title-bar", "Reset zoom", "conditional", "Zoom differs from 100 percent", "Reset active hosted-view zoom", 184, "src/renderer/src/comfyTitleBar/TitleBarApp.vue"),
        ("title-bar", "Send feedback", "active", "Feedback button enabled", "Record source and open the support URL externally", 106, "src/renderer/src/comfyTitleBar/TitleBarApp.vue"),
        ("title-bar", "Open downloads tray", "active", "Always; status changes with downloads", "Open active/recent downloads popup", 140, "src/renderer/src/comfyTitleBar/TitleBarApp.vue"),
        ("chooser", "Search installations", "active", "Chooser visible", "Filter tiles by fuzzy name/source/status text while preserving empty-state behavior", 22, "src/renderer/src/views/ChooserView.vue"),
        ("chooser", "Create new installation", "active", "Chooser CTA", "Open the install takeover/wizard", 18, "src/renderer/src/views/ChooserView.vue"),
        ("chooser", "Pick installation", "active", "Tile click/keyboard activation", "Focus a running host or begin launch with guarded handoff", 77, "src/renderer/src/views/ChooserView.vue"),
        ("chooser", "View retained error", "conditional", "Tile has crash/launch error", "Open the retained error detail surface", 99, "src/renderer/src/views/chooser/ChooserInstallTile.vue"),
        ("chooser", "View danger/status detail", "conditional", "Tile exposes a danger status", "Open the source-defined warning/detail surface", 24, "src/renderer/src/views/chooser/ChooserInstallTile.vue"),
        ("chooser-menu", "Manage", "conditional", "Manage callback available", "Open per-install configuration", 24, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Update", "conditional", "Installed update tag and stopped/no operation", "Open Update tab and run update-comfyui", 116, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Migrate", "conditional", "Migratable status and stopped/no operation", "Run migrate-to-standalone through managed progress", 47, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Restore snapshot", "conditional", "Installed local path and stopped/no operation", "Open snapshot management tab", 125, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Stop", "conditional", "Local-like installation running", "Confirm and stop the Python backend while retaining the host", 68, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Reveal in folder", "conditional", "Local-like installation has a path", "Open/reveal the installation path with platform-specific label", 187, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Share", "conditional", "Installed local snapshot-capable installation", "Export the latest snapshot; cancellation is silent and genuine failure alerts", 127, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Copy installation", "conditional", "Installed standalone and stopped/no operation", "Prompt for copy destination/name and run cancellable copy", 26, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Untrack", "conditional", "Installed non-adopted local-like installation", "Confirm and remove the registry record without deleting files", 20, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Delete", "conditional", "Installed local-like and stopped/no operation", "Danger-confirm and cancellably remove the installation files/record", 25, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Dismiss error", "conditional", "Retained error exists", "Clear the tile's retained error marker", 99, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Open by right-click", "active", "Applicable tile has actions", "Anchor the same action set at pointer coordinates", 179, "src/renderer/src/composables/useInstallContextMenu.ts"),
        ("chooser-menu", "Open by kebab", "active", "Applicable tile has actions", "Anchor the same action set below the More Actions button", 179, "src/renderer/src/composables/useInstallContextMenu.ts"),
    ]
    rows = [{"surface": surface, "action": action, "availability": availability, "condition": condition, "feature_id": feature_id(number), "observable_effect": effect, "source": rel(DESKTOP / source), "evidence_level": "code-inferred"} for surface, action, availability, condition, effect, number, source in shell_actions]
    write_csv("desktop-shell-actions.csv", ["surface", "action", "availability", "condition", "feature_id", "observable_effect", "source", "evidence_level"], rows)

    window_events = [
        ("app", "second-instance", "Packaged second launch", "Focus/restore an existing host rather than start a second app process", "No-op when the lock owner is quitting", 199, "src/main/index.ts"),
        ("app", "activate", "macOS dock activation", "Raise existing windows or create a chooser when none remain", "Preserve running child sessions", 89, "src/main/index.ts"),
        ("app", "before-quit", "Quit requested", "Run download/operation guards then stop sessions and close auxiliary windows", "Abort quit if user declines destructive cancellation", 70, "src/main/index.ts"),
        ("app", "window-all-closed", "Last host closes", "Quit only when no running session still requires ownership", "Retain app lifecycle while backend remains", 90, "src/main/index.ts"),
        ("app", "browser-window-created", "Any BrowserWindow created", "Attach OS session-ending and non-mac menu-suppression hooks", "Suppress updater install during OS shutdown", 71, "src/main/index.ts"),
        ("app", "browser-window-focus", "Host gains focus", "Refresh installation/directory state and MRU session identity", "Ignore destroyed/untracked hosts", 83, "src/main/index.ts"),
        ("host-window", "resize", "Bounds change", "Relayout title, panel, Comfy, popup, and overlay views; persist install bounds", "Clamp child bounds to content area", 83, "src/main/host/createHostWindow.ts"),
        ("host-window", "move", "Position changes", "Persist install-backed window origin", "Restore later clamps to visible displays", 83, "src/main/host/createHostWindow.ts"),
        ("host-window", "restore", "Window restored", "Relayout all WebContentsViews", "Preserve active panel/popup state", 76, "src/main/host/createHostWindow.ts"),
        ("host-window", "show", "Window becomes visible", "Relayout and synchronize title/panel state", "Startup restore waits for reveal handshake/backstop", 53, "src/main/host/createHostWindow.ts"),
        ("host-window", "focus", "Host gains focus", "Update MRU and last-session state", "No duplicate attach is created", 77, "src/main/host/createHostWindow.ts"),
        ("host-window", "close", "User/OS requests close", "Consult renderer overlays and configured confirm before teardown", "Cancellation leaves the window/session intact", 82, "src/main/host/createHostWindow.ts"),
        ("host-window", "closed", "Close succeeds", "Remove registry/listeners/views and update session persistence", "Owned child process follows close policy", 76, "src/main/host/createHostWindow.ts"),
        ("host-window", "enter-full-screen", "macOS fullscreen entered", "Relayout custom chrome and body views", "Preserve panel focus", 89, "src/main/host/createHostWindow.ts"),
        ("host-window", "leave-full-screen", "macOS fullscreen exited", "Relayout custom chrome and body views", "Restore saved normal bounds", 89, "src/main/host/createHostWindow.ts"),
        ("host-window", "query-session-end", "OS asks whether session may end", "Mark shutdown and prevent update-on-quit corruption path", "Do not wait on interactive updater UI", 71, "src/main/index.ts"),
        ("host-window", "session-end", "OS session ends", "Stop managed processes and exit cleanup path", "Suppress staged updater action", 71, "src/main/index.ts"),
        ("comfy-webcontents", "dom-ready", "Hosted DOM constructed", "Install desktop integrations/content scripts as applicable", "Reapply on navigation/reload", 174, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "did-finish-load", "Navigation succeeds", "Push current theme/state and mark hosted surface ready", "Late state is rehydrated", 61, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "did-fail-load", "Navigation fails", "Retain failure and retry after two seconds", "Further failure remains visible/retryable", 103, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "will-navigate", "A model-path relaunch has replaced ComfyUI with the local splash page", "Prevent every hosted navigation until the matching restarted process is ready", "A monotonic relaunch token prevents stale restart callbacks; the exact blocker is detached on supersession, successful restart, or window cleanup", 204, "src/main/index.ts"),
        ("comfy-webcontents", "render-process-gone", "Renderer crashes/exits", "Capture diagnostic context and reload when safe", "Backend ownership survives independent renderer recovery", 102, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "before-input-event", "Keyboard event", "Intercept close/reload/zoom shortcuts with platform modifiers", "Unmatched input continues to hosted Comfy", 180, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "zoom-changed", "Chromium zoom changes", "Synchronize zoom banner/title-menu reset state", "Clamp through desktop zoom policy", 184, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "page-title-updated", "Hosted page changes title", "Prevent uncontrolled OS title and push sanitized title identity", "Chooser/install identity remains authoritative", 85, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "ipc-message", "Trusted preload reports theme", "Validate sender/channel and update title-bar/overlay theme", "Ignore unrecognized sender/channel/payload", 150, "src/main/host/createHostWindow.ts"),
        ("firebase-bridge-webcontents", "console-message", "The injected cloud sign-in banner is active", "Open the exact internally generated loopback login URL only for the top-frame OPEN_LINK_SENTINEL message", "Ignore iframe and non-sentinel console messages; a replacement/finished sign-in detaches the listener and removes the injected banner", 162, "src/main/auth/firebaseBridge/index.ts"),
        ("comfy-webcontents", "did-create-window", "Allowed popup child created", "Wire checkout/auth child lifecycle and backdrop", "Denied origins never reach this state", 173, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "setWindowOpenHandler", "window.open requested", "Allow only popup allowlist, route safe externals to OS, deny others", "Return deny on malformed/untrusted URL", 172, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "will-prevent-unload", "Hosted page requests unload block", "Override for managed install lifecycle where source policy permits", "Desktop close guard remains authoritative", 82, "src/main/host/createHostWindow.ts"),
        ("comfy-webcontents", "context-menu", "Right click", "Build only applicable link/image/edit actions and suspend popup blur dismissal", "No menu opens when no action applies", 179, "src/main/lib/contextMenu.ts"),
        ("panel-webcontents", "did-finish-load", "Panel renderer ready", "Push current panel/install/startup state and reveal hidden restore when eligible", "Current state wins mid-load races", 84, "src/main/host/panelView.ts"),
        ("titlebar-webcontents", "did-finish-load", "Title renderer ready", "Push identity/theme/source/panel/download/update state", "Queued state is replayed", 85, "src/main/host/createHostWindow.ts"),
        ("checkout-window", "ready-to-show", "Child checkout painted", "Show child and backdrop without stale frame", "Parent remains interactive only where policy permits", 173, "src/main/host/createHostWindow.ts"),
        ("checkout-window", "before-input-event", "Escape or Cmd/Ctrl+W", "Close the checkout child", "Backdrop and overlay are removed", 173, "src/main/host/createHostWindow.ts"),
        ("checkout-window", "will-redirect", "Checkout navigates", "Close only on spoof-safe Comfy return URL", "Other allowed checkout navigation continues", 173, "src/main/host/createHostWindow.ts"),
        ("checkout-window", "did-navigate", "Checkout navigation commits", "Close only on spoof-safe Comfy return URL", "Auth/checkout state stays in shared partition", 173, "src/main/host/createHostWindow.ts"),
        ("checkout-window", "did-navigate-in-page", "A macOS auth/checkout child performs an in-page navigation to a passkey-incompatible origin", "Re-inject the passkey warning CSS and banner JavaScript after the navigation", "No-op outside macOS or for unmatched URL prefixes; CSS/script injection failures are isolated", 162, "src/main/host/createHostWindow.ts"),
        ("checkout-window", "resize", "Child resizes", "Reposition close control and parent backdrop", "Clamp overlays to content bounds", 173, "src/main/host/createHostWindow.ts"),
        ("checkout-window", "closed", "Child closes", "Destroy close overlay and dismiss backdrop", "Remove sender-scoped IPC listener", 173, "src/main/host/createHostWindow.ts"),
        ("embedded-popup-webcontents", "blur", "The popup opts into hideOnPopupBlur and blur dismissal is not temporarily suppressed", "Hide the popup and run its on-hide transition", "suppressBlurDismiss prevents dismissal while a child native/context menu owns focus; destruction makes the listener inert and cleans parent listeners", 175, "src/main/popups/embeddedPopupView.ts"),
        ("application-updater", "update-available", "The updater offers a parseable version newer than the running app", "Emit the deduplicated available event; with auto-install off expose available state, otherwise begin one silent auto-download intent", "Ignore missing/non-newer versions; a per-version guard prevents periodic-check re-entry and telemetry storms", 108, "src/main/lib/updater.ts"),
        ("application-updater", "update-downloaded", "A parseable newer application version finishes downloading", "Emit download completion, persist pendingDownloadedUpdateVersion, expose ready state, and prompt restart after a user-initiated download", "Ignore missing/non-newer versions; reset auto-download/user-intent guards and retain the staged marker for restart recovery", 110, "src/main/lib/updater.ts"),
        ("application-updater", "download-progress", "The updater supplies a progress record", "Narrow percent, transferred, total, and bytesPerSecond to number-or-null, transition user downloads to downloading, and broadcast app-update:download-progress", "Ignore non-record payloads; auto-on background downloads remain silent until ready and malformed individual fields become null", 110, "src/main/lib/updater.ts"),
    ]
    rows = [{"surface": surface, "event": event, "condition": condition, "observable_effect": effect, "failure_cleanup_recovery": recovery, "feature_id": feature_id(number), "source": rel(DESKTOP / source), "evidence_level": "code-inferred"} for surface, event, condition, effect, recovery, number, source in window_events]
    write_csv("desktop-window-events.csv", ["surface", "event", "condition", "observable_effect", "failure_cleanup_recovery", "feature_id", "source", "evidence_level"], rows)

    gestures = [
        ("Comfy view", "CmdOrCtrl+W", "Close through host guard; raw browser close is suppressed"),
        ("Comfy view", "F5", "Reload hosted Comfy view"),
        ("Comfy view", "CmdOrCtrl+R", "Reload hosted Comfy view"),
        ("Comfy view", "CmdOrCtrl++ or =", "Increase zoom by 0.5"),
        ("Comfy view", "CmdOrCtrl+-", "Decrease zoom by 0.5"),
        ("Comfy view", "CmdOrCtrl+0", "Reset zoom"),
        ("Terminal macOS", "Cmd+C / Cmd+V", "Copy selection / paste clipboard"),
        ("Terminal macOS", "Ctrl+C", "Send SIGINT"),
        ("Terminal Windows", "Ctrl+Shift+C / Ctrl+Shift+V", "Copy / paste"),
        ("Terminal Windows", "Ctrl+C", "Copy selection, otherwise send SIGINT"),
        ("Terminal Linux", "Ctrl+Shift+C / Ctrl+Shift+V", "Copy / paste"),
        ("Terminal Linux", "Ctrl+C", "Send SIGINT"),
        ("Modal/popover", "Escape", "Cancel or close topmost permitted surface"),
        ("Menu/list/tab", "Arrow keys", "Move roving focus/selection"),
        ("Menu/card/button", "Enter or Space", "Activate focused item"),
        ("Snapshot loader", "Drop one .json file", "Validate and preview snapshot"),
        ("Installation list", "Pointer drag", "Reorder cards and persist order"),
        ("Development", "CmdOrCtrl+Alt+U", "Cycle app-update state"),
        ("Development", "CmdOrCtrl+Alt+I", "Toggle installation-update state"),
    ]
    rows = [{"surface": surface, "input": input_, "behavior": behavior, "availability": "developer-only" if surface == "Development" else "platform-specific" if "Terminal" in surface else "active", "feature_id": map_feature(surface + behavior), "source": rel(DESKTOP / ("src/renderer/src/views/ConsoleModal.vue" if "Terminal" in surface else "src/main/lib/devShortcuts.ts" if surface == "Development" else "src/main/host/createHostWindow.ts" if surface == "Comfy view" else "src/renderer/src")), "evidence_level": "code-inferred"} for surface, input_, behavior in gestures]
    write_csv("desktop-keybindings-gestures.csv", ["surface", "input", "behavior", "availability", "feature_id", "source", "evidence_level"], rows)

    platforms = [
        ("Windows", "NSIS; win-unpacked", "win-x64", "LOCALAPPDATA or installation drive", "Ctrl conventions", "VC++ bootstrap/audit; port process kill; staged startup updater"),
        ("macOS", "DMG; ZIP", "mac-arm64", "Electron userData", "Command conventions", "notarization; app/dock menu; fullscreen events; update single-instance handoff"),
        ("Linux", "AppImage; DEB", "linux-x64", "XDG config/data/cache", "Ctrl conventions", "AppArmor hook; DEB system-managed updates; AppImage capability"),
    ]
    platform_feature_ids = [192, 194, 195]
    rows = [{"platform": p, "packages": package, "bootstrap": bootstrap, "data_location": data, "input_conventions": keys, "specific_behavior": behavior, "feature_id": feature_id(platform_feature_ids[index]), "source": rel(DESKTOP / "electron-builder.yml"), "evidence_level": "code-inferred"} for index, (p, package, bootstrap, data, keys, behavior) in enumerate(platforms)]
    write_csv("desktop-platform-matrix.csv", ["platform", "packages", "bootstrap", "data_location", "input_conventions", "specific_behavior", "feature_id", "source", "evidence_level"], rows)


def extract_flags_and_environment() -> None:
    records: dict[str, set[str]] = defaultdict(set)
    flag_records: dict[str, set[str]] = defaultdict(set)
    cli_records: dict[str, set[str]] = defaultdict(set)
    source_paths = sorted(
        path
        for pattern in ("*.ts", "*.vue")
        for path in (DESKTOP / "src").rglob(pattern)
        if ".test." not in path.name
    )
    for path in source_paths:
        text = read(path)
        for pattern in [r"process\.env\.([A-Za-z_][A-Za-z0-9_]*)", r"process\.env\[['\"]([^'\"]+)['\"]\]"]:
            for value in re.findall(pattern, text):
                records[value].add(rel(path))
        for value in re.findall(r"['\"]((?:COMFY|ELECTRON|NODE|PYTHON|CUDA|TORCH|HF|XDG|DD|POSTHOG)[A-Z0-9_]+)['\"]", text):
            records[value].add(rel(path))
        for value in re.findall(r"['\"](--[A-Za-z][A-Za-z0-9-]+)['\"]", text):
            cli_records[value].add(rel(path))
        for value in re.findall(r"['\"]((?:desktop|show|enable|disable)-[a-z0-9-]+)['\"]", text):
            if "desktop" in value or "signin" in value or "feature" in value:
                flag_records[value].add(rel(path))
    for key in ["PATH", "HOME", "USERPROFILE", "LOCALAPPDATA", "APPDATA", "PROGRAMDATA", "TEMP", "TMPDIR", "CI", "DISPLAY"]:
        records[key].add("platform/runtime convention")
    flag_records["desktop-first-use-fork-default"].add(rel(DESKTOP / "src/renderer/src/views/FirstUseTakeover.vue"))
    flag_records["desktop-cloud-capacity"].add(rel(DESKTOP / "src/main/lib/cloudCapacity.ts"))
    flag_records["show_signin_button"].add(rel(DESKTOP / "src/main/lib/ipc/sessionActions/launch.ts"))
    rows = []
    for key, sources in sorted(records.items()):
        rows.append({"kind": "environment-variable", "name": key, "availability": "developer-only" if key.startswith("E2E") or key == "CI" else "conditional", "feature_id": map_feature(key), "default": "unset/inherited unless source specifies otherwise", "behavior": "Read or supplied at the cited process/config boundary; preserve validation and secret redaction.", "source": "; ".join(sorted(sources)), "evidence_level": "code-inferred"})
    for key, sources in sorted(cli_records.items()):
        renderer_css = any("/renderer/" in source for source in sources) or key in {"--comfy-menu-bg", "--descrip-text"}
        rows.append({"kind": "CSS custom property" if renderer_css else "CLI-flag", "name": key, "availability": "developer-only" if "e2e" in key or "dev" in key else "conditional", "feature_id": map_feature(key), "default": "absent", "behavior": "Renderer CSS custom property consumed at the cited Vue/CSS boundary; it affects theme, layout, terminal, title-bar, menu, or chooser presentation rather than child-process argv." if renderer_css else "Recognized or emitted by desktop/Comfy child process at the cited boundary.", "source": "; ".join(sorted(sources)), "evidence_level": "code-inferred"})
    write_csv("desktop-cli-environment.csv", ["kind", "name", "availability", "feature_id", "default", "behavior", "source", "evidence_level"], rows)
    rows = []
    for key, sources in sorted(flag_records.items()):
        flag_feature_ids = {"desktop-first-use-fork-default": 38, "desktop-cloud-capacity": 165, "show_signin_button": 171}
        rows.append({"flag": key, "provider": "PostHog experiment/ops or negotiated Comfy feature registry", "availability": "experimental" if "first-use" in key else "conditional", "feature_id": feature_id(flag_feature_ids.get(key, 170)), "variants_default": "Source-defined; unavailable remote flags use a deterministic fallback.", "behavior": "Only recognized flags affect onboarding/cloud/Comfy launch; exposure is recorded where applicable.", "source": "; ".join(sorted(sources)), "evidence_level": "code-inferred"})
    write_csv("desktop-feature-flags.csv", ["flag", "provider", "availability", "feature_id", "variants_default", "behavior", "source", "evidence_level"], rows)


def flatten(value: object, prefix: str = "") -> dict[str, object]:
    result: dict[str, object] = {}
    if isinstance(value, dict):
        for key, child in value.items():
            child_prefix = f"{prefix}.{key}" if prefix else key
            result.update(flatten(child, child_prefix))
    elif isinstance(value, list):
        for index, child in enumerate(value):
            result.update(flatten(child, f"{prefix}[{index}]"))
    else:
        result[prefix] = value
    return result


def extract_localization() -> None:
    locale_dir = DESKTOP / "locales"
    locale_files = sorted(locale_dir.glob("*.json"))
    flattened = {path.stem: flatten(json.loads(read(path))) for path in locale_files}
    keys = sorted(set().union(*(values.keys() for values in flattened.values())))
    production_text = "\n".join(read(path) for path in (DESKTOP / "src").rglob("*.*") if path.suffix in {".ts", ".vue"} and ".test." not in path.name)
    rows = []
    for key in keys:
        present = [locale for locale, values in flattened.items() if key in values]
        rows.append({"key": key, "locales_present": ";".join(present), "locale_count": len(present), "reference_hint": "literal-reference" if key in production_text else "dynamic-or-unreferenced", "availability": "active" if len(present) == len(locale_files) else "uncertain", "feature_id": feature_id(151), "source": "; ".join(rel(path) for path in locale_files), "evidence_level": "code-inferred"})
    write_csv("desktop-localization.csv", ["key", "locales_present", "locale_count", "reference_hint", "availability", "feature_id", "source", "evidence_level"], rows)


def classify_source(path: Path) -> tuple[str, str, str]:
    value = path.as_posix().lower()
    if ".test." in value or value.endswith(".spec.ts") or "/e2e/" in value:
        classification = "test-only"
    elif path.suffix.lower() in {".svg", ".woff2", ".png", ".ico", ".icns", ".css", ".scss"}:
        classification = "asset"
    elif path.suffix.lower() in {".d.ts", ".html"}:
        classification = "generated-or-declaration"
    else:
        classification = "production"
    keyword_map = [
        (("snapshot",), 121), (("download",), 136), (("terminal",), 91), (("log",), 95),
        (("telemetry", "datadog", "experiment"), 167), (("update", "release"), 107),
        (("firebase", "auth", "cloud", "tier", "checkout"), 162), (("settings", "locale", "theme", "i18n"), 149),
        (("source", "install", "migration", "adopt", "oem"), 1), (("launch", "process", "port", "gpu", "python", "pip", "git"), 51),
        (("host", "window", "panel", "popup", "titlebar", "titlebar", "modal", "menu", "zoom"), 76),
        (("crash", "stderr", "exitcode", "vcruntime"), 99), (("preload", "ipc"), 174),
    ]
    identifiers = []
    for needles, number in keyword_map:
        if any(needle in value for needle in needles):
            identifiers.append(feature_id(number))
    if identifiers:
        reason = "Mapped by source subsystem to the named desktop capability cluster."
    elif classification == "test-only":
        if "/e2e/" in value:
            identifiers.append(feature_id(189))
            reason = "Mapped to the deterministic E2E shell-control capability; case titles retain the more specific exercised behavior."
        elif "/renderer/" in value:
            identifiers.append(feature_id(181))
            reason = "Mapped to shared renderer interaction/accessibility behavior; case titles retain the more specific exercised behavior."
        else:
            identifiers.append(feature_id(174))
            reason = "Mapped to the shared desktop service/bridge contract; case titles retain the more specific exercised behavior."
    else:
        reason = "Classified as shared UI/build/type infrastructure; retained explicitly rather than silently excluded."
        if classification == "production":
            classification = "infrastructure-only"
    return classification, ";".join(dict.fromkeys(identifiers)), reason


def source_and_test_coverage() -> None:
    paths = [
        path
        for path in DESKTOP.rglob("*")
        if path.is_file() and not any(part in {"node_modules", "out", "dist"} for part in path.parts)
    ]
    rows = []
    for path in sorted(set(path for path in paths if path.is_file())):
        classification, ids, reason = classify_source(path)
        rows.append({"path": rel(path), "classification": classification, "feature_ids": ids, "coverage_disposition": "mapped" if ids else "explicitly-classified", "reason": reason})
    write_csv("desktop-source-coverage.csv", ["path", "classification", "feature_ids", "coverage_disposition", "reason"], rows)

    test_rows = []
    for path in sorted(path for path in paths if path.is_file() and path.suffix in {".ts", ".tsx", ".js", ".mjs"} and (".test." in path.name or path.name.endswith(".spec.ts"))):
        text = read(path)
        classification, ids, reason = classify_source(path)
        titles = []
        for match in re.finditer(r"(?:it|test|describe)\s*(?:\.\w+)?\s*\(\s*['\"]([^'\"]+)['\"]", text):
            titles.append(match.group(1))
        test_rows.append({"test_file": rel(path), "suite_or_case_count": len(titles), "feature_ids": ids, "coverage_disposition": "mapped" if ids else "explicitly-classified", "mapping_reason": reason, "evidence_level": "test-backed", "titles": " | ".join(titles), "runtime_result": "not-run: dependencies/runtime unavailable in source snapshot"})
    write_csv("desktop-tests.csv", ["test_file", "suite_or_case_count", "feature_ids", "coverage_disposition", "mapping_reason", "evidence_level", "titles", "runtime_result"], test_rows)


def main() -> None:
    write_csv("desktop-features.csv", list(FEATURES[0].keys()), FEATURES)
    extract_ipc()
    extract_preload_apis()
    write_settings()
    write_persistence()
    write_static_surface_catalogs()
    extract_flags_and_environment()
    extract_localization()
    source_and_test_coverage()


if __name__ == "__main__":
    main()
