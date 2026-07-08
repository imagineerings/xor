use gpui::{
    App, Context, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};
use scheduler::Priority;
use ui::{
    Button, ButtonStyle, Color, Divider, DividerColor, Icon, IconButton, IconName, IconSize, Label,
    LabelSize, Switch, ToggleState, Tooltip, prelude::*,
};
use util::ResultExt as _;

use crate::components::SettingsInputField;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScheduleId(u64);

impl ScheduleId {
    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub id: ScheduleId,
    pub config: ScheduleConfig,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleConfig {
    pub name: SharedString,
    pub cron_expression: SharedString,
    pub task: ScheduledTask,
    pub enabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScheduledTask {
    RunRecipe { recipe_name: SharedString },
    SendMessage { prompt: SharedString },
    RunDiagnostics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduledTaskKind {
    RunRecipe,
    SendMessage,
    RunDiagnostics,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SchedulingSettingsEvent {
    ScheduleCreated(ScheduleId),
    ScheduleDeleted(ScheduleId),
    ScheduleToggled { id: ScheduleId, enabled: bool },
}

pub struct SchedulingSettings {
    schedules: Vec<Schedule>,
    next_schedule_id: u64,
    form: ScheduleForm,
    last_error: Option<SharedString>,
    pending_delete_schedule_id: Option<ScheduleId>,
    executor_priority: Priority,
    focus_handle: FocusHandle,
}

#[derive(Clone, Debug)]
struct ScheduleForm {
    name: String,
    cron_expression: String,
    task_kind: ScheduledTaskKind,
    task_input: String,
}

impl Default for ScheduleForm {
    fn default() -> Self {
        Self {
            name: String::new(),
            cron_expression: "0 9 * * *".to_string(),
            task_kind: ScheduledTaskKind::RunRecipe,
            task_input: String::new(),
        }
    }
}

impl SchedulingSettings {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            schedules: Vec::new(),
            next_schedule_id: 1,
            form: ScheduleForm::default(),
            last_error: None,
            pending_delete_schedule_id: None,
            executor_priority: Priority::Low,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn schedules(&self) -> &[Schedule] {
        &self.schedules
    }

    pub fn last_error(&self) -> Option<&SharedString> {
        self.last_error.as_ref()
    }

    pub fn set_form_name(&mut self, name: impl Into<String>, cx: &mut Context<Self>) {
        self.form.name = name.into();
        self.last_error = None;
        cx.notify();
    }

    pub fn set_form_cron_expression(
        &mut self,
        cron_expression: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.form.cron_expression = cron_expression.into();
        self.last_error = None;
        cx.notify();
    }

    pub fn set_form_task_kind(&mut self, task_kind: ScheduledTaskKind, cx: &mut Context<Self>) {
        self.form.task_kind = task_kind;
        self.last_error = None;
        cx.notify();
    }

    pub fn set_form_task_input(&mut self, task_input: impl Into<String>, cx: &mut Context<Self>) {
        self.form.task_input = task_input.into();
        self.last_error = None;
        cx.notify();
    }

    pub fn create_schedule_from_form(&mut self, cx: &mut Context<Self>) {
        match self.build_schedule_config() {
            Ok(config) => {
                let id = ScheduleId(self.next_schedule_id);
                self.next_schedule_id += 1;
                self.schedules.push(Schedule { id, config });
                self.form = ScheduleForm::default();
                self.last_error = None;
                self.pending_delete_schedule_id = None;
                cx.emit(SchedulingSettingsEvent::ScheduleCreated(id));
                cx.notify();
            }
            Err(message) => {
                self.last_error = Some(message.into());
                cx.notify();
            }
        }
    }

    pub fn request_delete_schedule(&mut self, id: ScheduleId, cx: &mut Context<Self>) {
        self.pending_delete_schedule_id = Some(id);
        cx.notify();
    }

    pub fn cancel_delete_schedule(&mut self, id: ScheduleId, cx: &mut Context<Self>) {
        if self.pending_delete_schedule_id == Some(id) {
            self.pending_delete_schedule_id = None;
            cx.notify();
        }
    }

    pub fn delete_schedule(&mut self, id: ScheduleId, cx: &mut Context<Self>) {
        let previous_len = self.schedules.len();
        self.schedules.retain(|schedule| schedule.id != id);
        if self.schedules.len() != previous_len {
            if self.pending_delete_schedule_id == Some(id) {
                self.pending_delete_schedule_id = None;
            }
            cx.emit(SchedulingSettingsEvent::ScheduleDeleted(id));
            cx.notify();
        }
    }

    pub fn set_schedule_enabled(&mut self, id: ScheduleId, enabled: bool, cx: &mut Context<Self>) {
        if let Some(schedule) = self.schedules.iter_mut().find(|schedule| schedule.id == id)
            && schedule.config.enabled != enabled
        {
            schedule.config.enabled = enabled;
            cx.emit(SchedulingSettingsEvent::ScheduleToggled { id, enabled });
            cx.notify();
        }
    }

    fn build_schedule_config(&self) -> Result<ScheduleConfig, &'static str> {
        let name = self.form.name.trim();
        if name.is_empty() {
            return Err("Schedule name is required.");
        }

        let cron_expression = self.form.cron_expression.trim();
        if !is_supported_cron_expression(cron_expression) {
            return Err(
                "Use a five-field cron expression or @hourly, @daily, @weekly, or @monthly.",
            );
        }

        let task = match self.form.task_kind {
            ScheduledTaskKind::RunRecipe => {
                let recipe_name = self.form.task_input.trim();
                if recipe_name.is_empty() {
                    return Err("Recipe name is required.");
                }
                ScheduledTask::RunRecipe {
                    recipe_name: recipe_name.into(),
                }
            }
            ScheduledTaskKind::SendMessage => {
                let prompt = self.form.task_input.trim();
                if prompt.is_empty() {
                    return Err("Prompt is required.");
                }
                ScheduledTask::SendMessage {
                    prompt: prompt.into(),
                }
            }
            ScheduledTaskKind::RunDiagnostics => ScheduledTask::RunDiagnostics,
        };

        Ok(ScheduleConfig {
            name: name.into(),
            cron_expression: cron_expression.into(),
            task,
            enabled: true,
        })
    }

    fn render_schedule_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self
            .schedules
            .iter()
            .map(|schedule| self.render_schedule_row(schedule, cx).into_any_element())
            .collect::<Vec<_>>();

        v_flex()
            .gap_2()
            .child(Label::new("Schedules").size(LabelSize::Large))
            .when(self.schedules.is_empty(), |this| {
                this.child(
                    Label::new("No schedules created yet.")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
            })
            .children(rows)
    }

    fn render_schedule_row(&self, schedule: &Schedule, cx: &mut Context<Self>) -> impl IntoElement {
        let id = schedule.id;
        let enabled = schedule.config.enabled;
        let toggle_state = if enabled {
            ToggleState::Selected
        } else {
            ToggleState::Unselected
        };
        let this = cx.entity().downgrade();
        let confirming_delete = self.pending_delete_schedule_id == Some(id);

        h_flex()
            .id(("schedule-row", id.as_u64()))
            .justify_between()
            .items_center()
            .gap_3()
            .p_3()
            .rounded_sm()
            .border_1()
            .border_color(cx.theme().colors().border)
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Label::new(schedule.config.name.clone()))
                            .child(
                                Label::new(schedule.config.cron_expression.clone())
                                    .size(LabelSize::Small)
                                    .color(Color::Muted),
                            ),
                    )
                    .child(
                        Label::new(task_label(&schedule.config.task))
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Switch::new(("schedule-enabled", id.as_u64()), toggle_state).on_click({
                            let this = this.clone();
                            move |state, _window, cx| {
                                this.update(cx, |this, cx| {
                                    this.set_schedule_enabled(
                                        id,
                                        *state == ToggleState::Selected,
                                        cx,
                                    );
                                })
                                .log_err();
                            }
                        }),
                    )
                    .when(confirming_delete, |this| {
                        this.child(
                            Button::new(("confirm-delete-schedule", id.as_u64()), "Delete")
                                .style(ButtonStyle::Filled)
                                .on_click(cx.listener(move |this, _, _window, cx| {
                                    this.delete_schedule(id, cx);
                                })),
                        )
                        .child(
                            Button::new(("cancel-delete-schedule", id.as_u64()), "Cancel")
                                .style(ButtonStyle::Outlined)
                                .on_click(cx.listener(move |this, _, _window, cx| {
                                    this.cancel_delete_schedule(id, cx);
                                })),
                        )
                    })
                    .when(!confirming_delete, |this| {
                        this.child(
                            IconButton::new(("delete-schedule", id.as_u64()), IconName::Trash)
                                .icon_size(IconSize::Small)
                                .tooltip(Tooltip::text("Delete Schedule"))
                                .on_click(cx.listener(move |this, _, _window, cx| {
                                    this.request_delete_schedule(id, cx);
                                })),
                        )
                    }),
            )
    }

    fn render_create_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let weak_self = cx.entity().downgrade();

        v_flex()
            .gap_3()
            .child(Label::new("Create Schedule").size(LabelSize::Large))
            .child(form_field(
                "Name",
                SettingsInputField::new()
                    .with_id("schedule-name")
                    .with_initial_text(self.form.name.clone())
                    .with_placeholder("Nightly release check")
                    .display_confirm_button()
                    .on_confirm({
                        let weak_self = weak_self.clone();
                        move |value, _window, cx| {
                            weak_self
                                .update(cx, |this, cx| {
                                    this.set_form_name(value.unwrap_or_default(), cx);
                                })
                                .log_err();
                        }
                    }),
            ))
            .child(form_field(
                "Cron",
                SettingsInputField::new()
                    .with_id("schedule-cron")
                    .with_initial_text(self.form.cron_expression.clone())
                    .with_placeholder("0 9 * * *")
                    .display_confirm_button()
                    .on_confirm({
                        let weak_self = weak_self.clone();
                        move |value, _window, cx| {
                            weak_self
                                .update(cx, |this, cx| {
                                    this.set_form_cron_expression(value.unwrap_or_default(), cx);
                                })
                                .log_err();
                        }
                    }),
            ))
            .child(self.render_task_kind_selector(cx))
            .when(
                self.form.task_kind != ScheduledTaskKind::RunDiagnostics,
                |this| {
                    this.child(form_field(
                        task_input_label(self.form.task_kind),
                        SettingsInputField::new()
                            .with_id("schedule-task-input")
                            .with_initial_text(self.form.task_input.clone())
                            .with_placeholder(task_input_placeholder(self.form.task_kind))
                            .display_confirm_button()
                            .on_confirm({
                                let weak_self = weak_self.clone();
                                move |value, _window, cx| {
                                    weak_self
                                        .update(cx, |this, cx| {
                                            this.set_form_task_input(value.unwrap_or_default(), cx);
                                        })
                                        .log_err();
                                }
                            }),
                    ))
                },
            )
            .when_some(self.last_error.clone(), |this, error| {
                this.child(
                    h_flex()
                        .gap_2()
                        .child(Icon::new(IconName::Warning).color(Color::Warning))
                        .child(
                            Label::new(error)
                                .size(LabelSize::Small)
                                .color(Color::Warning),
                        ),
                )
            })
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        Label::new(format!("Scheduler priority: {:?}", self.executor_priority))
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    )
                    .child(
                        Button::new("create-schedule", "Create")
                            .style(ButtonStyle::Filled)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.create_schedule_from_form(cx);
                            })),
                    ),
            )
    }

    fn render_task_kind_selector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let kinds = [
            (0_u64, ScheduledTaskKind::RunRecipe, "Recipe"),
            (1, ScheduledTaskKind::SendMessage, "Message"),
            (2, ScheduledTaskKind::RunDiagnostics, "Diagnostics"),
        ];

        v_flex()
            .gap_1()
            .child(
                Label::new("Task")
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .child(
                h_flex()
                    .gap_2()
                    .children(kinds.into_iter().map(|(index, kind, label)| {
                        let selected = self.form.task_kind == kind;
                        Button::new(("schedule-task-kind", index), label)
                            .style(if selected {
                                ButtonStyle::Filled
                            } else {
                                ButtonStyle::Outlined
                            })
                            .on_click(cx.listener(move |this, _, _window, cx| {
                                this.set_form_task_kind(kind, cx);
                            }))
                    })),
            )
    }
}

impl Render for SchedulingSettings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("SchedulingSettings")
            .track_focus(&self.focus_handle)
            .size_full()
            .gap_4()
            .p_6()
            .child(self.render_schedule_list(cx))
            .child(Divider::horizontal().color(DividerColor::Border))
            .child(self.render_create_form(cx))
    }
}

impl Focusable for SchedulingSettings {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<SchedulingSettingsEvent> for SchedulingSettings {}

fn form_field(label: &'static str, input: impl IntoElement) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(Label::new(label).size(LabelSize::Small).color(Color::Muted))
        .child(input)
}

fn task_label(task: &ScheduledTask) -> SharedString {
    match task {
        ScheduledTask::RunRecipe { recipe_name } => format!("Run recipe: {recipe_name}").into(),
        ScheduledTask::SendMessage { prompt } => format!("Send message: {prompt}").into(),
        ScheduledTask::RunDiagnostics => "Run diagnostics".into(),
    }
}

fn task_input_label(kind: ScheduledTaskKind) -> &'static str {
    match kind {
        ScheduledTaskKind::RunRecipe => "Recipe",
        ScheduledTaskKind::SendMessage => "Prompt",
        ScheduledTaskKind::RunDiagnostics => "Task",
    }
}

fn task_input_placeholder(kind: ScheduledTaskKind) -> &'static str {
    match kind {
        ScheduledTaskKind::RunRecipe => "Release Risk Check",
        ScheduledTaskKind::SendMessage => "Summarize project status",
        ScheduledTaskKind::RunDiagnostics => "",
    }
}

fn is_supported_cron_expression(expression: &str) -> bool {
    let expression = expression.trim();
    if matches!(expression, "@hourly" | "@daily" | "@weekly" | "@monthly") {
        return true;
    }

    let fields = expression.split_whitespace().collect::<Vec<_>>();
    fields.len() == 5 && fields.iter().all(|field| !field.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_supported_cron_expressions() {
        assert!(is_supported_cron_expression("0 9 * * *"));
        assert!(is_supported_cron_expression("@daily"));
        assert!(!is_supported_cron_expression("0 9 * *"));
        assert!(!is_supported_cron_expression(""));
    }

    #[test]
    fn task_labels_describe_scheduled_work() {
        let label = task_label(&ScheduledTask::RunRecipe {
            recipe_name: "Release Risk Check".into(),
        });

        assert_eq!(label.as_ref(), "Run recipe: Release Risk Check");
    }
}
