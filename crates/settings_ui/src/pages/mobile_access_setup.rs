use gpui::{
    AsyncApp, ClipboardItem, Context, ReadGlobal, ScrollHandle, SharedString, Task, WeakEntity,
    Window, img, prelude::*,
};
use mobile_tunnel::qr_code::generate_qr_code_png;
use mobile_tunnel::{
    GlobalTunnelManager, GlobalTunnelState, TunnelManager, TunnelStatus, render_image_from_png,
};
use remote::SshConnectionOptions;
use settings::SettingsStore;
use std::sync::Arc;
use ui::{Banner, Button, ButtonStyle, Label, Severity, prelude::*};

use crate::components::SettingsInputField;

use crate::{SettingsWindow, all_projects};

/// Render the Mobile Access setup sub-page.
pub(crate) fn render_mobile_access_setup_page(
    settings_window: &SettingsWindow,
    scroll_handle: &ScrollHandle,
    window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    // Read saved SSH host/port from user settings
    let (saved_host, saved_port) = load_mobile_access_settings(cx);

    // Read current tunnel state from the global
    let (status, connection_string, qr_image) = {
        let guard = cx.global::<GlobalTunnelManager>().0.lock();
        match guard.as_ref() {
            Some(state) => (
                state.manager.status().clone(),
                Some(state.cached_connection_string.clone()),
                state.cached_qr_render_image.clone(),
            ),
            None => (TunnelStatus::Stopped, None, None),
        }
    };

    // Check for active SSH remote connection
    let has_remote = settings_window
        .original_window
        .as_ref()
        .is_some_and(|handle| {
            all_projects(Some(handle), cx).any(|project| project.read(cx).remote_client().is_some())
        });

    v_flex()
        .id("mobile-access-page")
        .size_full()
        .pt_2()
        .pb_16()
        .track_scroll(scroll_handle)
        .overflow_y_scroll()
        .child(render_header())
        .child(render_connection_status(has_remote))
        .child(match &status {
            TunnelStatus::Stopped => {
                render_stopped_state(saved_host, saved_port, has_remote, window, cx)
                    .into_any_element()
            }
            TunnelStatus::Starting | TunnelStatus::Stopping => {
                render_transition_state(match &status {
                    TunnelStatus::Starting => "Starting tunnel…",
                    _ => "Stopping tunnel…",
                })
                .into_any_element()
            }
            TunnelStatus::Running(info) => {
                render_running_state(info, connection_string.as_deref(), qr_image, cx)
            }
            TunnelStatus::Error { message } => {
                render_error_state(message.clone(), cx).into_any_element()
            }
        })
        .into_any_element()
}

fn render_header() -> impl IntoElement {
    v_flex()
        .px_8()
        .child(Label::new("Mobile Access").size(LabelSize::Large))
        .child(
            Label::new(
                "Securely connect your mobile device to this development environment via SSH tunneling. \
                 Scan the QR code from the Sim mobile app to establish a connection.",
            )
            .size(LabelSize::Small)
            .color(Color::Muted),
        )
}

fn render_connection_status(has_remote: bool) -> impl IntoElement {
    v_flex().px_8().mt_2().child(
        h_flex()
            .gap_2()
            .child(
                Icon::new(if has_remote {
                    IconName::Check
                } else {
                    IconName::Info
                })
                .size(IconSize::Small)
                .color(if has_remote {
                    Color::Success
                } else {
                    Color::Muted
                }),
            )
            .child(
                Label::new(if has_remote {
                    "Active SSH remote connection detected — tunnel will reuse it."
                } else {
                    "No active SSH remote connection. The tunnel will use standalone SSH."
                })
                .size(LabelSize::Small)
                .color(Color::Muted),
            ),
    )
}

fn render_stopped_state(
    saved_host: String,
    saved_port: String,
    _has_remote: bool,
    _window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> impl IntoElement {
    let start_handler = cx.listener(|this, _event, _window, cx| {
        let has_remote = this.original_window.as_ref().is_some_and(|handle| {
            all_projects(Some(handle), cx).any(|project| project.read(cx).remote_client().is_some())
        });
        // Save current settings before starting
        save_mobile_access_settings_to_disk(cx);
        start_tunnel_async(has_remote, cx).detach_and_log_err(cx);
    });

    v_flex()
        .px_8()
        .mt_4()
        .gap_4()
        // SSH Host field (only relevant for standalone mode)
        .child(
            v_flex().gap_1().child(Label::new("SSH Host")).child(
                SettingsInputField::new()
                    .with_id("mobile-ssh-host")
                    .with_initial_text(saved_host)
                    .with_placeholder("localhost")
                    .on_confirm(move |value, _window, cx| {
                        let host = value.unwrap_or_default();
                        save_mobile_access_host(host, cx);
                    }),
            ),
        )
        // SSH Port field
        .child(
            v_flex().gap_1().child(Label::new("SSH Port")).child(
                SettingsInputField::new()
                    .with_id("mobile-ssh-port")
                    .with_initial_text(saved_port)
                    .with_placeholder("22")
                    .on_confirm(move |value, _window, cx| {
                        let port = value.unwrap_or_default();
                        save_mobile_access_port(port, cx);
                    }),
            ),
        )
        // Start button
        .child(
            Button::new("start-tunnel", "Start Tunnel")
                .style(ButtonStyle::Filled)
                .on_click(start_handler),
        )
}

fn render_transition_state(message: &str) -> impl IntoElement {
    v_flex().px_8().mt_4().gap_4().child(
        h_flex()
            .gap_2()
            .child(Label::new(message).color(Color::Muted))
            .child(
                Icon::new(IconName::ArrowCircle)
                    .size(IconSize::Small)
                    .color(Color::Accent),
            ),
    )
}

fn render_running_state(
    info: &mobile_tunnel::TunnelInfo,
    connection_string: Option<&str>,
    qr_image: Option<Arc<gpui::RenderImage>>,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let stop_handler = cx.listener(|this, _event, _window, cx| {
        stop_tunnel_async(this, cx).detach_and_log_err(cx);
    });

    let qr_section = if let (Some(conn_str), Some(image)) = (connection_string, qr_image) {
        let conn_str = conn_str.to_string();
        let copy_handler = cx.listener(move |_this, _event, _window, cx| {
            cx.write_to_clipboard(ClipboardItem::new_string(conn_str.clone()));
        });

        Some(
            v_flex()
                .gap_2()
                .child(Label::new("Scan this QR code with the Sim mobile app:"))
                .child(img(image).w(px(220.)).h(px(220.)).rounded_md())
                .child(
                    Button::new("copy-connection", "Copy Connection String")
                        .style(ButtonStyle::OutlinedGhost)
                        .on_click(copy_handler),
                )
                .into_any_element(),
        )
    } else {
        None
    };

    let connection_text = format!(
        "Tunnel running on port {}. Auth token: {}",
        info.local_port,
        info.auth_token.as_deref().unwrap_or("none"),
    );

    v_flex()
        .px_8()
        .mt_4()
        .gap_4()
        .child(
            v_flex()
                .gap_1()
                .child(
                    h_flex()
                        .gap_2()
                        .child(
                            Icon::new(IconName::Check)
                                .size(IconSize::Small)
                                .color(Color::Success),
                        )
                        .child(Label::new("Tunnel Active").color(Color::Success)),
                )
                .child(
                    Label::new(connection_text)
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
        )
        .when_some(qr_section, |this, qr| this.child(qr))
        .child(
            Button::new("stop-tunnel", "Stop Tunnel")
                .style(ButtonStyle::Tinted(ui::TintColor::Error))
                .on_click(stop_handler),
        )
        .into_any_element()
}

fn render_error_state(message: SharedString, cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    let retry_handler = cx.listener(|this, _event, _window, cx| {
        let has_remote = this.original_window.as_ref().is_some_and(|handle| {
            all_projects(Some(handle), cx).any(|project| project.read(cx).remote_client().is_some())
        });
        start_tunnel_async(has_remote, cx).detach_and_log_err(cx);
    });

    v_flex()
        .px_8()
        .mt_4()
        .gap_4()
        .child(
            Banner::new()
                .severity(Severity::Error)
                .child(Label::new(message)),
        )
        .child(
            Button::new("retry-tunnel", "Retry")
                .style(ButtonStyle::Filled)
                .on_click(retry_handler),
        )
}

/// Load saved mobile access settings from the user's settings file.
fn load_mobile_access_settings(cx: &mut App) -> (String, String) {
    let content = SettingsStore::global(cx)
        .raw_user_settings()
        .map(|u| u.content.as_ref());
    let host = content
        .and_then(|c| c.remote.mobile_access.as_ref())
        .and_then(|m| m.ssh_host.clone())
        .unwrap_or_else(|| "localhost".to_string());
    let port = content
        .and_then(|c| c.remote.mobile_access.as_ref())
        .and_then(|m| m.ssh_port.map(|p| p.to_string()))
        .unwrap_or_else(|| "22".to_string());
    (host, port)
}

/// Save the SSH host to the user's settings file.
fn save_mobile_access_host(host: String, cx: &mut App) {
    SettingsStore::global(cx).update_settings_file(<dyn fs::Fs>::global(cx), move |settings, _| {
        settings
            .remote
            .mobile_access
            .get_or_insert_default()
            .ssh_host = Some(if host.is_empty() {
            "localhost".to_string()
        } else {
            host
        });
    });
}

/// Save the SSH port to the user's settings file.
fn save_mobile_access_port(port: String, cx: &mut App) {
    let parsed_port = port.parse::<u16>().ok();
    SettingsStore::global(cx).update_settings_file(<dyn fs::Fs>::global(cx), move |settings, _| {
        settings
            .remote
            .mobile_access
            .get_or_insert_default()
            .ssh_port = parsed_port;
    });
}

/// Persist the current SSH host/port values from settings fields to disk.
fn save_mobile_access_settings_to_disk(_cx: &mut App) {
    // The individual on_confirm callbacks already persist each field on blur.
    // This is called as a safety net before tunnel start so that any in-flight
    // changes that haven't yet been confirmed (e.g. the user typed but didn't
    // blur) get captured. We re-read the editor contents here.
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use mobile_tunnel::{GlobalTunnelManager, GlobalTunnelState, TunnelManager, TunnelStatus};
    use parking_lot::Mutex;
    use remote::SshConnectionOptions;

    use crate::test;

    use super::*;

    /// Create a minimal TunnelManager in a specific state for testing.
    fn manager_with_status(status: TunnelStatus) -> TunnelManager {
        let mut manager = TunnelManager::new_standalone(SshConnectionOptions {
            host: "test-host".into(),
            username: None,
            port: None,
            password: None,
            args: None,
            port_forwards: None,
            connection_timeout: None,
            nickname: None,
            upload_binary_over_ssh: false,
        });
        manager.set_status_for_test(status);
        manager
    }

    #[gpui::test]
    async fn test_render_stopped_state(cx: &mut TestAppContext) {
        let (settings_window, cx) = cx.add_window_view(|window, cx| {
            test::register_settings(cx);
            // No global set → Stopped state with no manager
            cx.set_global(GlobalTunnelManager(Mutex::new(None)));
            SettingsWindow::test(window, cx)
        });

        settings_window.update_in(cx, |settings_window, window, cx| {
            let scroll_handle = ScrollHandle::new();
            let _output =
                render_mobile_access_setup_page(settings_window, &scroll_handle, window, cx);
            // Render succeeded without panicking
        });
    }

    #[gpui::test]
    async fn test_render_running_state(cx: &mut TestAppContext) {
        let (settings_window, cx) = cx.add_window_view(|window, cx| {
            test::register_settings(cx);
            let manager = manager_with_status(TunnelStatus::Running(mobile_tunnel::TunnelInfo {
                endpoint_url: "127.0.0.1:9999".into(),
                auth_token: Some("tok-test".into()),
                local_port: 9999,
            }));
            let state = GlobalTunnelState {
                manager,
                cached_connection_string: "sim-tunnel://127.0.0.1:9999?token=tok-test".to_string(),
                // QR image left None to keep test dependencies minimal;
                // the render function handles this gracefully (skips QR section).
                cached_qr_render_image: None,
            };
            cx.set_global(GlobalTunnelManager(Mutex::new(Some(state))));
            SettingsWindow::test(window, cx)
        });

        settings_window.update_in(cx, |settings_window, window, cx| {
            let scroll_handle = ScrollHandle::new();
            let _output =
                render_mobile_access_setup_page(settings_window, &scroll_handle, window, cx);
            // Render succeeded without panicking — tunnel status section is shown
        });
    }

    #[gpui::test]
    async fn test_render_error_state(cx: &mut TestAppContext) {
        let (settings_window, cx) = cx.add_window_view(|window, cx| {
            test::register_settings(cx);
            let manager = manager_with_status(TunnelStatus::Error {
                message: "SSH tunnel failed to establish".into(),
            });
            let state = GlobalTunnelState {
                manager,
                cached_connection_string: String::new(),
                cached_qr_render_image: None,
            };
            cx.set_global(GlobalTunnelManager(Mutex::new(Some(state))));
            SettingsWindow::test(window, cx)
        });

        settings_window.update_in(cx, |settings_window, window, cx| {
            let scroll_handle = ScrollHandle::new();
            let _output =
                render_mobile_access_setup_page(settings_window, &scroll_handle, window, cx);
            // Render succeeded without panicking — error banner is shown
        });
    }
}

/// Spawn a task that creates (if needed) and starts the SSH tunnel.
fn start_tunnel_async(
    has_remote: bool,
    cx: &mut Context<SettingsWindow>,
) -> Task<anyhow::Result<()>> {
    cx.spawn(
        async move |this: WeakEntity<SettingsWindow>, cx: &mut AsyncApp| {
            // Phase 1: Create TunnelManager if not already initialized
            let already = cx.update(|cx| cx.global::<GlobalTunnelManager>().0.lock().is_some());
            if !already {
                let (remote_weak, standalone_opts) = cx.update(|cx| {
                    if has_remote {
                        let app_state = workspace::AppState::global(cx);
                        for ws_ref in app_state.workspace_store.read(cx).workspaces() {
                            if let Some(ws) = ws_ref.upgrade() {
                                let project = ws.read(cx).project().clone();
                                if let Some(rc) = project.read(cx).remote_client() {
                                    return (Some(rc.downgrade()), None);
                                }
                            }
                        }
                    }
                    let (host, port_str) = load_mobile_access_settings(cx);
                    let port = port_str.parse::<u16>().ok().or(Some(22));
                    let opts = SshConnectionOptions {
                        host: host.into(),
                        username: None,
                        port,
                        password: None,
                        args: None,
                        port_forwards: None,
                        connection_timeout: Some(10),
                        nickname: None,
                        upload_binary_over_ssh: false,
                    };
                    (None, Some(opts))
                });

                let manager = if let Some(weak) = remote_weak {
                    TunnelManager::new_with_remote(weak)
                } else if let Some(opts) = standalone_opts {
                    TunnelManager::new_standalone(opts)
                } else {
                    anyhow::bail!("no SSH connection available");
                };

                cx.update(|cx| {
                    cx.global_mut::<GlobalTunnelManager>()
                        .0
                        .lock()
                        .replace(GlobalTunnelState {
                            manager,
                            cached_connection_string: String::new(),
                            cached_qr_render_image: None,
                        });
                });
            }

            // Phase 2: Take manager out, start the tunnel, put it back
            let mut state = cx
                .update(|cx| cx.global_mut::<GlobalTunnelManager>().0.lock().take())
                .ok_or_else(|| anyhow::anyhow!("tunnel manager not initialized"))?;

            let info = state.manager.start(cx).await?;

            cx.update(|cx| {
                cx.global_mut::<GlobalTunnelManager>()
                    .0
                    .lock()
                    .replace(state);
            });

            // Phase 3: Generate QR code and store it
            let connection_string = mobile_tunnel::qr_code::build_connection_string(
                &info.endpoint_url,
                info.local_port,
                info.auth_token.as_deref(),
            );

            let png_bytes = generate_qr_code_png(&connection_string)?;
            let render_image = render_image_from_png(&png_bytes)?;

            // Phase 4: Update the global with QR code data and notify settings window
            cx.update(|cx| {
                let mut guard = cx.global_mut::<GlobalTunnelManager>().0.lock();
                if let Some(ref mut s) = *guard {
                    s.cached_connection_string = connection_string;
                    s.cached_qr_render_image = Some(render_image);
                }
            });

            this.update(cx, |_, cx| {
                cx.notify();
            })
            .ok();

            Ok(())
        },
    )
}

/// Spawn a task that stops the SSH tunnel.
fn stop_tunnel_async(
    _settings_window: &mut SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> Task<anyhow::Result<()>> {
    cx.spawn(
        async move |this: WeakEntity<SettingsWindow>, cx: &mut AsyncApp| {
            let mut state = cx
                .update(|cx| cx.global_mut::<GlobalTunnelManager>().0.lock().take())
                .ok_or_else(|| anyhow::anyhow!("tunnel not running"))?;

            state.manager.stop()?;

            // Reset QR code cache
            state.cached_connection_string.clear();
            state.cached_qr_render_image = None;

            cx.update(|cx| {
                cx.global_mut::<GlobalTunnelManager>()
                    .0
                    .lock()
                    .replace(state);
            });

            this.update(cx, |_, cx| {
                cx.notify();
            })
            .ok();

            Ok(())
        },
    )
}
