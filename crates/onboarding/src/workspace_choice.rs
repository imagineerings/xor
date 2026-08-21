use fs::Fs;
use gpui::{App, FocusHandle, IntoElement};
use settings::{Settings, update_settings_file};
use ui::{TintColor, prelude::*};
use workspace::{WorkspacePresentation, WorkspaceSettings, effective_workspace_presentation};

const SHARED_DATA_EXPLANATION: &str =
    "Both presentations use the same underlying projects and data.";

#[derive(Clone, Copy)]
struct WorkspaceChoice {
    id: &'static str,
    label: &'static str,
    description: &'static str,
    aria_label: &'static str,
    presentation: WorkspacePresentation,
}

const EDITOR_WORKSPACE_CHOICE: WorkspaceChoice = WorkspaceChoice {
    id: "onboarding-editor-workspace",
    label: "Editor Workspace",
    description: "Zed's current editor experience.",
    aria_label: "Editor Workspace. Zed's current editor experience. Both presentations use the same underlying projects and data.",
    presentation: WorkspacePresentation::Editor,
};

#[cfg(feature = "multiplayer-tools")]
const WORKSPACE_CHOICES: [WorkspaceChoice; 2] = [
    EDITOR_WORKSPACE_CHOICE,
    WorkspaceChoice {
        id: "onboarding-collaborative-workspace",
        label: "Collaborative Workspace",
        description: "A workspace where humans and agents build together.",
        aria_label: "Collaborative Workspace. A workspace where humans and agents build together. Both presentations use the same underlying projects and data.",
        presentation: WorkspacePresentation::Collaborative,
    },
];

#[cfg(not(feature = "multiplayer-tools"))]
const WORKSPACE_CHOICES: [WorkspaceChoice; 1] = [EDITOR_WORKSPACE_CHOICE];

pub(crate) fn render_workspace_choice(tab_index: &mut isize, cx: &mut App) -> impl IntoElement {
    render_workspace_choice_with_focus_handles(tab_index, [None, None], cx)
}

fn render_workspace_choice_with_focus_handles(
    tab_index: &mut isize,
    focus_handles: [Option<FocusHandle>; 2],
    cx: &mut App,
) -> impl IntoElement {
    let selected_presentation =
        effective_workspace_presentation(WorkspaceSettings::get_global(cx).workspace_presentation);

    v_flex()
        .gap_2()
        .child(Label::new("Workspace"))
        .child(
            Label::new(SHARED_DATA_EXPLANATION)
                .color(Color::Muted)
                .size(LabelSize::Small),
        )
        .child(
            h_flex().items_start().gap_2().children(
                WORKSPACE_CHOICES
                    .into_iter()
                    .zip(focus_handles)
                    .map(|(choice, focus_handle)| {
                        let choice_tab_index = *tab_index;
                        *tab_index += 1;

                        v_flex()
                            .debug_selector(move || choice.id.to_owned())
                            .min_w_0()
                            .w_full()
                            .gap_1()
                            .child(
                                Button::new(choice.id, choice.label)
                                    .full_width()
                                    .style(ButtonStyle::OutlinedGhost)
                                    .size(ButtonSize::Medium)
                                    .selected_style(ButtonStyle::Tinted(TintColor::Accent))
                                    .toggle_state(selected_presentation == choice.presentation)
                                    .aria_label(choice.aria_label)
                                    .tab_index(choice_tab_index)
                                    .when_some(focus_handle, |button, focus_handle| {
                                        button.track_focus(&focus_handle)
                                    })
                                    .on_click(move |_, _, cx| {
                                        persist_workspace_presentation(choice.presentation, cx);
                                    }),
                            )
                            .child(
                                Label::new(choice.description)
                                    .color(Color::Muted)
                                    .size(LabelSize::Small),
                            )
                    }),
            ),
        )
}

fn persist_workspace_presentation(presentation: WorkspacePresentation, cx: &mut App) {
    let fs = <dyn Fs>::global(cx);
    update_settings_file(fs, cx, move |settings, _cx| {
        settings.workspace.workspace_presentation = Some(presentation);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs::FakeFs;
    #[cfg(not(feature = "multiplayer-tools"))]
    use gpui::UpdateGlobal;
    use gpui::{
        Context, FocusHandle, KeyDownEvent, KeyUpEvent, Keystroke, PlatformInput, Render,
        Subscription, TestAppContext, VisualTestContext, Window,
    };
    use settings::SettingsStore;
    use std::sync::Arc;

    struct WorkspaceChoiceTestView {
        focus_handle: FocusHandle,
        editor_focus_handle: FocusHandle,
        collaborative_focus_handle: FocusHandle,
        _settings_subscription: Subscription,
    }

    impl WorkspaceChoiceTestView {
        fn new(cx: &mut Context<Self>) -> Self {
            Self {
                focus_handle: cx.focus_handle(),
                editor_focus_handle: cx.focus_handle(),
                collaborative_focus_handle: cx.focus_handle(),
                _settings_subscription: cx
                    .observe_global::<SettingsStore>(move |_, cx| cx.notify()),
            }
        }
    }

    impl Render for WorkspaceChoiceTestView {
        fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            let mut tab_index = 0;
            div().track_focus(&self.focus_handle).size_full().child(
                render_workspace_choice_with_focus_handles(
                    &mut tab_index,
                    [
                        Some(self.editor_focus_handle.clone()),
                        Some(self.collaborative_focus_handle.clone()),
                    ],
                    cx,
                ),
            )
        }
    }

    fn activate_focused_button(key: &str, cx: &mut VisualTestContext) {
        let keystroke = Keystroke::parse(key).expect("activation key should parse");
        cx.update(|window, cx| {
            window.dispatch_event(
                PlatformInput::KeyDown(KeyDownEvent {
                    keystroke: keystroke.clone(),
                    is_held: false,
                    prefer_character_input: false,
                }),
                cx,
            );
            window.dispatch_event(PlatformInput::KeyUp(KeyUpEvent { keystroke }), cx);
        });
        cx.run_until_parked();
    }

    #[gpui::test]
    async fn workspace_choice_labels_selection_keyboard_activation_and_persisted_value(
        cx: &mut TestAppContext,
    ) {
        let fs = FakeFs::new(cx.executor());
        let settings_fs: Arc<dyn Fs> = fs.clone();
        cx.update(|cx| {
            let settings_store = SettingsStore::test(cx);
            cx.set_global(settings_store);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
            <dyn Fs>::set_global(settings_fs.clone(), cx);
        });

        let (view, cx) = cx.add_window_view(|_window, cx| WorkspaceChoiceTestView::new(cx));
        cx.run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear());
        assert!(cx.debug_bounds("onboarding-editor-workspace").is_some());
        #[cfg(feature = "multiplayer-tools")]
        assert!(
            cx.debug_bounds("onboarding-collaborative-workspace")
                .is_some()
        );
        #[cfg(not(feature = "multiplayer-tools"))]
        assert!(
            cx.debug_bounds("onboarding-collaborative-workspace")
                .is_none()
        );

        #[cfg(feature = "multiplayer-tools")]
        assert_eq!(
            WORKSPACE_CHOICES.map(|choice| choice.label),
            ["Editor Workspace", "Collaborative Workspace"]
        );
        #[cfg(not(feature = "multiplayer-tools"))]
        assert_eq!(
            WORKSPACE_CHOICES.map(|choice| choice.label),
            ["Editor Workspace"]
        );
        assert_eq!(
            SHARED_DATA_EXPLANATION,
            "Both presentations use the same underlying projects and data."
        );
        assert_eq!(
            cx.update(|_, cx| WorkspaceSettings::get_global(cx).workspace_presentation),
            WorkspacePresentation::Editor
        );

        #[cfg(feature = "multiplayer-tools")]
        {
            view.update_in(cx, |view, window, cx| {
                window.focus(&view.collaborative_focus_handle, cx);
            });
            cx.update(|window, cx| window.draw(cx).clear());
            activate_focused_button("space", cx);

            assert_eq!(
                cx.update(|_, cx| WorkspaceSettings::get_global(cx).workspace_presentation),
                WorkspacePresentation::Collaborative
            );
            let settings_text = SettingsStore::load_settings(&settings_fs)
                .await
                .expect("workspace choice settings should be readable");
            assert!(settings_text.contains(r#""workspace_presentation": "collaborative""#));
        }

        #[cfg(not(feature = "multiplayer-tools"))]
        cx.update(|_, cx| {
            SettingsStore::update_global(cx, |store, cx| {
                store.update_user_settings(cx, |settings| {
                    settings.workspace.workspace_presentation =
                        Some(WorkspacePresentation::Collaborative);
                });
            });
        });

        #[cfg(not(feature = "multiplayer-tools"))]
        {
            cx.run_until_parked();
            cx.update(|window, cx| window.draw(cx).clear());
            assert_eq!(
                cx.update(|_, cx| WorkspaceSettings::get_global(cx).workspace_presentation),
                WorkspacePresentation::Collaborative
            );
            assert_eq!(
                cx.update(|_, cx| effective_workspace_presentation(
                    WorkspaceSettings::get_global(cx).workspace_presentation
                )),
                WorkspacePresentation::Editor
            );
        }

        view.update_in(cx, |view, window, cx| {
            window.focus(&view.editor_focus_handle, cx);
        });
        cx.update(|window, cx| window.draw(cx).clear());
        activate_focused_button("enter", cx);

        assert_eq!(
            cx.update(|_, cx| WorkspaceSettings::get_global(cx).workspace_presentation),
            WorkspacePresentation::Editor
        );
        let settings_text = SettingsStore::load_settings(&settings_fs)
            .await
            .expect("workspace choice settings should be readable");
        assert!(settings_text.contains(r#""workspace_presentation": "editor""#));
    }
}
