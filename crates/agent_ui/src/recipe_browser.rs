use std::sync::Arc;

use anyhow::Result;
use editor::{Editor, EditorEvent};
use gpui::{
    App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, ParentElement, Render,
    SharedString, Styled, Subscription, Window,
};
use recipe::{
    BuiltinRecipeSource, ExecutionContext, Recipe, RecipeEngine, RecipeManifest, RecipeOutput,
    RecipeSourceType,
};
use ui::prelude::*;
use ui::{
    Button, ButtonStyle, Color, Divider, DividerColor, Icon, IconButton, IconName, IconSize, Label,
    LabelSize, Tooltip,
};

#[derive(Clone, Debug)]
pub enum RecipeBrowserEvent {
    RecipeRun {
        recipe: Recipe,
        output: RecipeOutput,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecipeBrowserRunState {
    Idle,
    Confirming { recipe_name: SharedString },
    Running { recipe_name: SharedString },
    Finished { summary: SharedString },
    Failed { message: SharedString },
}

pub struct RecipeBrowser {
    engine: Arc<RecipeEngine>,
    recipes: Vec<RecipeManifest>,
    filtered_recipe_indices: Vec<usize>,
    selected_recipe_index: Option<usize>,
    run_state: RecipeBrowserRunState,
    search_editor: Entity<Editor>,
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl RecipeBrowser {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::with_engine(
            Arc::new(RecipeEngine::new().with_source(BuiltinRecipeSource::sim_defaults())),
            window,
            cx,
        )
    }

    pub fn with_engine(
        engine: Arc<RecipeEngine>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let search_editor = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Search recipes...", window, cx);
            editor
        });
        let search_subscription = cx.subscribe(
            &search_editor,
            |this: &mut Self, _, event: &EditorEvent, cx| {
                if matches!(
                    event,
                    EditorEvent::BufferEdited | EditorEvent::Edited { .. }
                ) {
                    this.filter_recipes(cx);
                }
            },
        );

        let mut browser = Self {
            engine,
            recipes: Vec::new(),
            filtered_recipe_indices: Vec::new(),
            selected_recipe_index: None,
            run_state: RecipeBrowserRunState::Idle,
            search_editor,
            focus_handle,
            _subscriptions: vec![search_subscription],
        };
        browser.load_recipes(cx);
        browser
    }

    pub fn load_recipes(&mut self, cx: &mut Context<Self>) {
        match self.engine.discover_all() {
            Ok(recipes) => {
                self.recipes = recipes;
                self.filtered_recipe_indices = (0..self.recipes.len()).collect();
                self.selected_recipe_index = self.filtered_recipe_indices.first().copied();
                self.run_state = RecipeBrowserRunState::Idle;
            }
            Err(error) => {
                self.recipes.clear();
                self.filtered_recipe_indices.clear();
                self.selected_recipe_index = None;
                self.run_state = RecipeBrowserRunState::Failed {
                    message: format!("Failed to load recipes: {error}").into(),
                };
            }
        }
        cx.notify();
    }

    pub fn select_recipe(&mut self, recipe_index: usize, cx: &mut Context<Self>) {
        if self.filtered_recipe_indices.contains(&recipe_index) {
            self.selected_recipe_index = Some(recipe_index);
            self.run_state = RecipeBrowserRunState::Idle;
            cx.notify();
        }
    }

    pub fn request_run_selected_recipe(&mut self, cx: &mut Context<Self>) {
        let Some(recipe) = self.selected_recipe() else {
            return;
        };
        self.run_state = RecipeBrowserRunState::Confirming {
            recipe_name: recipe.name.clone().into(),
        };
        cx.notify();
    }

    pub fn cancel_run_confirmation(&mut self, cx: &mut Context<Self>) {
        if matches!(self.run_state, RecipeBrowserRunState::Confirming { .. }) {
            self.run_state = RecipeBrowserRunState::Idle;
            cx.notify();
        }
    }

    pub fn confirm_run_selected_recipe(&mut self, cx: &mut Context<Self>) {
        let Some(recipe_name) = self.selected_recipe().map(|recipe| recipe.name.clone()) else {
            return;
        };
        self.run_state = RecipeBrowserRunState::Running {
            recipe_name: recipe_name.clone().into(),
        };

        let result = self.run_recipe(&recipe_name);
        match result {
            Ok((recipe, output)) => {
                self.run_state = RecipeBrowserRunState::Finished {
                    summary: output.summary.clone().into(),
                };
                cx.emit(RecipeBrowserEvent::RecipeRun { recipe, output });
            }
            Err(error) => {
                self.run_state = RecipeBrowserRunState::Failed {
                    message: error.to_string().into(),
                };
            }
        }
        cx.notify();
    }

    pub fn recipes(&self) -> &[RecipeManifest] {
        &self.recipes
    }

    pub fn filtered_recipes(&self) -> impl Iterator<Item = &RecipeManifest> {
        self.filtered_recipe_indices
            .iter()
            .filter_map(|index| self.recipes.get(*index))
    }

    pub fn selected_recipe(&self) -> Option<&RecipeManifest> {
        self.selected_recipe_index
            .and_then(|index| self.recipes.get(index))
    }

    pub fn run_state(&self) -> &RecipeBrowserRunState {
        &self.run_state
    }

    fn filter_recipes(&mut self, cx: &mut Context<Self>) {
        let query = self.search_editor.read(cx).text(cx).to_string();
        self.filtered_recipe_indices = self
            .recipes
            .iter()
            .enumerate()
            .filter_map(|(index, recipe)| recipe_matches_query(recipe, &query).then_some(index))
            .collect();

        if !self
            .selected_recipe_index
            .is_some_and(|selected| self.filtered_recipe_indices.contains(&selected))
        {
            self.selected_recipe_index = self.filtered_recipe_indices.first().copied();
        }
        cx.notify();
    }

    fn run_recipe(&self, recipe_name: &str) -> Result<(Recipe, RecipeOutput)> {
        let recipe = self.engine.load(recipe_name)?;
        let mut context = ExecutionContext::default();
        let output = self.engine.execute(&recipe, &mut context)?;
        Ok((recipe, output))
    }

    fn render_recipe_list(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_recipes = !self.filtered_recipe_indices.is_empty();
        v_flex()
            .w(px(280.))
            .min_w(px(220.))
            .h_full()
            .border_r_1()
            .border_color(cx.theme().colors().border)
            .child(self.render_search_bar(cx))
            .child(
                v_flex()
                    .size_full()
                    .when(!has_recipes, |this| {
                        this.child(
                            v_flex()
                                .p_4()
                                .gap_1()
                                .child(Label::new("No recipes found"))
                                .child(
                                    Label::new("Try a different search.")
                                        .size(LabelSize::Small)
                                        .color(Color::Muted),
                                ),
                        )
                    })
                    .children(
                        self.filtered_recipe_indices
                            .iter()
                            .map(|index| self.render_recipe_card(*index, cx).into_any_element()),
                    ),
            )
    }

    fn render_search_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_query = !self.search_editor.read(cx).text(cx).is_empty();
        h_flex()
            .h_10()
            .px_3()
            .gap_2()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                Icon::new(IconName::MagnifyingGlass)
                    .size(IconSize::Small)
                    .color(Color::Muted),
            )
            .child(self.search_editor.clone())
            .when(has_query, |this| {
                this.child(
                    IconButton::new("clear-recipe-search", IconName::Close)
                        .icon_size(IconSize::Small)
                        .tooltip(Tooltip::text("Clear Search"))
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.search_editor.update(cx, |editor, cx| {
                                editor.set_text("", window, cx);
                            });
                            this.filter_recipes(cx);
                        })),
                )
            })
    }

    fn render_recipe_card(&self, recipe_index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(recipe) = self.recipes.get(recipe_index) else {
            return div().into_any_element();
        };
        let selected = self.selected_recipe_index == Some(recipe_index);
        let title = recipe.name.clone();
        let description = recipe.description.clone();

        v_flex()
            .id(("recipe-card", recipe_index))
            .m_1()
            .p_2()
            .gap_1()
            .rounded_sm()
            .border_1()
            .border_color(if selected {
                cx.theme().colors().border_variant
            } else {
                cx.theme().colors().border
            })
            .bg(if selected {
                cx.theme().colors().element_selected
            } else {
                cx.theme().colors().elevated_surface_background
            })
            .hover(|style| style.bg(cx.theme().colors().element_hover))
            .on_click(cx.listener(move |this, _, _window, cx| {
                this.select_recipe(recipe_index, cx);
            }))
            .child(Label::new(title))
            .child(
                Label::new(description)
                    .size(LabelSize::Small)
                    .color(Color::Muted),
            )
            .when(!recipe.tags.is_empty(), |this| {
                this.child(
                    h_flex()
                        .gap_1()
                        .flex_wrap()
                        .children(recipe.tags.iter().take(3).map(|tag| {
                            Label::new(format!("#{tag}"))
                                .size(LabelSize::XSmall)
                                .color(Color::Muted)
                        })),
                )
            })
            .into_any_element()
    }

    fn render_recipe_detail(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.selected_recipe() {
            Some(recipe) => self
                .render_selected_recipe_detail(recipe, cx)
                .into_any_element(),
            None => v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(Label::new("Select a recipe"))
                .into_any_element(),
        }
    }

    fn render_selected_recipe_detail(
        &self,
        recipe: &RecipeManifest,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let run_disabled = matches!(
            self.run_state,
            RecipeBrowserRunState::Running { .. } | RecipeBrowserRunState::Confirming { .. }
        );

        v_flex()
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
                            .child(Label::new(recipe.name.clone()).size(LabelSize::Large))
                            .child(
                                Label::new(recipe.description.clone())
                                    .color(Color::Muted)
                                    .size(LabelSize::Small),
                            ),
                    )
                    .child(
                        Button::new("run-selected-recipe", "Run")
                            .start_icon(Icon::new(IconName::PlayFilled))
                            .style(ButtonStyle::Filled)
                            .disabled(run_disabled)
                            .on_click(cx.listener(|this, _, _window, cx| {
                                this.request_run_selected_recipe(cx);
                            })),
                    ),
            )
            .child(self.render_run_state(cx))
            .child(Divider::horizontal().color(DividerColor::Border))
            .child(recipe_metadata_rows(recipe))
            .child(self.render_variables(recipe))
            .child(self.render_tags(recipe))
    }

    fn render_run_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        match &self.run_state {
            RecipeBrowserRunState::Idle => div().into_any_element(),
            RecipeBrowserRunState::Confirming { recipe_name } => h_flex()
                .p_2()
                .gap_2()
                .rounded_sm()
                .bg(cx.theme().colors().editor_foreground.opacity(0.06))
                .child(
                    Label::new(format!("Run {recipe_name}?"))
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
                .child(
                    Button::new("confirm-run-recipe", "Run")
                        .style(ButtonStyle::Filled)
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.confirm_run_selected_recipe(cx);
                        })),
                )
                .child(
                    Button::new("cancel-run-recipe", "Cancel")
                        .style(ButtonStyle::Outlined)
                        .on_click(cx.listener(|this, _, _window, cx| {
                            this.cancel_run_confirmation(cx);
                        })),
                )
                .into_any_element(),
            RecipeBrowserRunState::Running { recipe_name } => {
                Label::new(format!("Running {recipe_name}..."))
                    .color(Color::Muted)
                    .into_any_element()
            }
            RecipeBrowserRunState::Finished { summary } => Label::new(summary.clone())
                .color(Color::Success)
                .into_any_element(),
            RecipeBrowserRunState::Failed { message } => Label::new(message.clone())
                .color(Color::Error)
                .into_any_element(),
        }
    }

    fn render_variables(&self, recipe: &RecipeManifest) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(Label::new("Variables"))
            .when(recipe.variables.is_empty(), |this| {
                this.child(
                    Label::new("None")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
            })
            .children(recipe.variables.iter().map(|variable| {
                h_flex()
                    .gap_2()
                    .child(Icon::new(IconName::Info).size(IconSize::XSmall))
                    .child(Label::new(variable.clone()).size(LabelSize::Small))
            }))
    }

    fn render_tags(&self, recipe: &RecipeManifest) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(Label::new("Tags"))
            .when(recipe.tags.is_empty(), |this| {
                this.child(
                    Label::new("None")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                )
            })
            .when(!recipe.tags.is_empty(), |this| {
                this.child(
                    h_flex()
                        .gap_1()
                        .flex_wrap()
                        .children(recipe.tags.iter().map(|tag| {
                            Label::new(format!("#{tag}"))
                                .size(LabelSize::Small)
                                .color(Color::Muted)
                        })),
                )
            })
    }
}

impl Render for RecipeBrowser {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .key_context("RecipeBrowser")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().colors().surface_background)
            .child(self.render_recipe_list(cx))
            .child(self.render_recipe_detail(cx))
    }
}

impl Focusable for RecipeBrowser {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<RecipeBrowserEvent> for RecipeBrowser {}

fn recipe_metadata_rows(recipe: &RecipeManifest) -> impl IntoElement {
    v_flex()
        .gap_2()
        .child(metadata_row("Source", source_label(&recipe.source)))
        .child(metadata_row("Version", recipe.version.clone()))
        .when_some(recipe.author.clone(), |this, author| {
            this.child(metadata_row("Author", author))
        })
}

fn metadata_row(label: &'static str, value: impl Into<SharedString>) -> impl IntoElement {
    h_flex()
        .gap_2()
        .child(Label::new(label).size(LabelSize::Small).color(Color::Muted))
        .child(Label::new(value.into()).size(LabelSize::Small))
}

fn source_label(source: &RecipeSourceType) -> SharedString {
    match source {
        RecipeSourceType::Builtin => "Built-in".into(),
        RecipeSourceType::Local { path } => format!("Local: {}", path.display()).into(),
        RecipeSourceType::GitHub { owner, repo, path } => {
            format!("GitHub: {owner}/{repo}/{path}").into()
        }
        RecipeSourceType::Deeplink { uri } => format!("Deeplink: {uri}").into(),
    }
}

fn recipe_matches_query(recipe: &RecipeManifest, query: &str) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }

    recipe.name.to_lowercase().contains(&query)
        || recipe.description.to_lowercase().contains(&query)
        || recipe.version.to_lowercase().contains(&query)
        || recipe
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(&query))
        || recipe
            .variables
            .iter()
            .any(|variable| variable.to_lowercase().contains(&query))
        || recipe
            .author
            .as_ref()
            .is_some_and(|author| author.to_lowercase().contains(&query))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn manifest() -> RecipeManifest {
        RecipeManifest {
            name: "Release Risk Check".to_string(),
            description: "Evaluate release readiness".to_string(),
            version: "1.0.0".to_string(),
            source: RecipeSourceType::Local {
                path: PathBuf::from("/recipes/release.yaml"),
            },
            tags: vec!["release".to_string(), "risk".to_string()],
            author: Some("Sim".to_string()),
            variables: vec!["target_branch".to_string()],
        }
    }

    #[test]
    fn recipe_search_matches_name_description_tags_and_variables() {
        let recipe = manifest();

        assert!(recipe_matches_query(&recipe, "release"));
        assert!(recipe_matches_query(&recipe, "readiness"));
        assert!(recipe_matches_query(&recipe, "target_branch"));
        assert!(!recipe_matches_query(&recipe, "scheduler"));
    }

    #[test]
    fn source_labels_include_recipe_origin() {
        let label = source_label(&manifest().source);

        assert_eq!(label.as_ref(), "Local: /recipes/release.yaml");
    }
}
