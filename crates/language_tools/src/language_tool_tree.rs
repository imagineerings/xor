use gpui::{
    AnyElement, App, FocusHandle, IntoElement, MouseButton, Pixels, Point, Role, ScrollStrategy,
    Task, UniformListScrollHandle, Window, actions, div, px, uniform_list,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use ui::{Disclosure, Icon, IconName, IconSize, prelude::*};

actions!(
    language_tool_tree,
    [
        SelectPrevious,
        SelectNext,
        SelectFirst,
        SelectLast,
        SelectParent,
        SelectFirstChild,
        ToggleExpanded,
        ActivateSelected,
        ExpandAll,
        CollapseAll,
        Refresh,
    ]
);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct LanguageToolNodeId(pub String);

#[derive(Clone, Debug)]
pub struct LanguageToolNode {
    pub id: LanguageToolNodeId,
    pub label: String,
    pub secondary_label: Option<String>,
    pub icon: Option<IconName>,
    pub accessibility_label: String,
    pub children: Vec<LanguageToolNode>,
    pub enabled: bool,
    pub activation_label: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct LanguageToolSnapshot {
    pub roots: Vec<LanguageToolNode>,
    pub status: LanguageToolProviderStatus,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum LanguageToolProviderStatus {
    #[default]
    Current,
    Loading,
    Empty(String),
    Partial(String),
    Stale(String),
    Restricted(String),
    Unsupported(String),
    Mismatch(String),
    Error(String),
    Disconnected(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguageToolTreeStatus {
    Dormant,
    Loading,
    Current,
    Refreshing,
    Empty(String),
    Partial(String),
    Stale(String),
    Restricted(String),
    Unsupported(String),
    Mismatch(String),
    Error(String),
    StaleError(String),
    Disconnected(String),
    DisconnectedStale(String),
}

#[derive(Clone, Debug)]
pub struct VisibleLanguageToolRow {
    pub id: LanguageToolNodeId,
    pub parent_id: Option<LanguageToolNodeId>,
    pub level: usize,
    pub is_branch: bool,
    pub is_expanded: bool,
    pub node: LanguageToolNode,
}

pub fn language_tool_tree(
    rows: Vec<VisibleLanguageToolRow>,
    selected: Option<LanguageToolNodeId>,
    status_message: Option<String>,
    scroll_handle: UniformListScrollHandle,
    on_click: impl Fn(LanguageToolNodeId, usize, &mut Window, &mut App) + 'static,
    on_toggle: impl Fn(LanguageToolNodeId, &mut Window, &mut App) + 'static,
    on_context_menu: impl Fn(LanguageToolNodeId, Point<Pixels>, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let rows = Arc::new(rows);
    let on_click = Arc::new(on_click);
    let on_toggle = Arc::new(on_toggle);
    let on_context_menu = Arc::new(on_context_menu);

    v_flex()
        .flex_1()
        .when_some(status_message, |tree, message| {
            tree.child(div().px_2().py_1().child(message))
        })
        .child(
            div()
                .id("language-tool-tree")
                .role(Role::Tree)
                .flex_1()
                .child(
                    uniform_list(
                        "language-tool-tree-rows",
                        rows.len(),
                        move |range, _, cx| {
                            range
                                .filter_map(|index| rows.get(index))
                                .map(|row| {
                                    let click_id = row.id.clone();
                                    let toggle_id = row.id.clone();
                                    let context_id = row.id.clone();
                                    let on_click = on_click.clone();
                                    let on_toggle = on_toggle.clone();
                                    let on_context_menu = on_context_menu.clone();
                                    let is_selected = selected.as_ref() == Some(&row.id);
                                    div()
                                        .id(row.id.0.clone())
                                        .role(Role::TreeItem)
                                        .aria_label(row.node.accessibility_label.clone())
                                        .aria_level(row.level)
                                        .aria_selected(is_selected)
                                        .when(row.is_branch, |element| {
                                            element.aria_expanded(row.is_expanded)
                                        })
                                        .pl(px((row.level.saturating_sub(1) * 16) as f32))
                                        .h_7()
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .cursor_pointer()
                                        .when(is_selected, |element| {
                                            element.bg(cx.theme().colors().element_selected)
                                        })
                                        .child(if row.is_branch {
                                            Disclosure::new(
                                                format!("language-tool-disclosure:{}", row.id.0),
                                                row.is_expanded,
                                            )
                                            .on_click(move |_, window, cx| {
                                                cx.stop_propagation();
                                                on_toggle(toggle_id.clone(), window, cx);
                                            })
                                            .into_any_element()
                                        } else {
                                            div().w_4().into_any_element()
                                        })
                                        .when_some(row.node.icon, |element, icon| {
                                            element.child(Icon::new(icon).size(IconSize::Small))
                                        })
                                        .child(row.node.label.clone())
                                        .when_some(
                                            row.node.secondary_label.clone(),
                                            |element, secondary| {
                                                element.child(
                                                    div()
                                                        .text_color(cx.theme().colors().text_muted)
                                                        .child(secondary),
                                                )
                                            },
                                        )
                                        .on_click(move |event, window, cx| {
                                            on_click(
                                                click_id.clone(),
                                                event.click_count(),
                                                window,
                                                cx,
                                            );
                                        })
                                        .on_mouse_down(
                                            MouseButton::Right,
                                            move |event, window, cx| {
                                                cx.stop_propagation();
                                                on_context_menu(
                                                    context_id.clone(),
                                                    event.position,
                                                    window,
                                                    cx,
                                                );
                                            },
                                        )
                                        .into_any_element()
                                })
                                .collect::<Vec<AnyElement>>()
                        },
                    )
                    .size_full()
                    .track_scroll(&scroll_handle),
                ),
        )
}

pub fn status_message(status: &LanguageToolTreeStatus) -> Option<String> {
    match status {
        LanguageToolTreeStatus::Dormant => {
            Some("Open this language tool panel to load its project model.".to_string())
        }
        LanguageToolTreeStatus::Loading => Some("Loading project model…".to_string()),
        LanguageToolTreeStatus::Refreshing => Some("Refreshing project model…".to_string()),
        LanguageToolTreeStatus::Empty(message)
        | LanguageToolTreeStatus::Partial(message)
        | LanguageToolTreeStatus::Stale(message)
        | LanguageToolTreeStatus::Restricted(message)
        | LanguageToolTreeStatus::Unsupported(message)
        | LanguageToolTreeStatus::Mismatch(message)
        | LanguageToolTreeStatus::Error(message)
        | LanguageToolTreeStatus::StaleError(message)
        | LanguageToolTreeStatus::Disconnected(message)
        | LanguageToolTreeStatus::DisconnectedStale(message) => Some(message.clone()),
        LanguageToolTreeStatus::Current => None,
    }
}

pub struct LanguageToolTreeHost {
    focus_handle: Option<FocusHandle>,
    scroll_handle: UniformListScrollHandle,
    refresh_task: Task<()>,
    debounce_task: Task<()>,
    snapshot: LanguageToolSnapshot,
    visible_rows: Vec<VisibleLanguageToolRow>,
    parent_by_id: HashMap<LanguageToolNodeId, LanguageToolNodeId>,
    expanded: HashSet<LanguageToolNodeId>,
    selected: Option<LanguageToolNodeId>,
    status: LanguageToolTreeStatus,
    generation: u64,
    dirty: bool,
}

impl Default for LanguageToolTreeHost {
    fn default() -> Self {
        Self {
            focus_handle: None,
            scroll_handle: UniformListScrollHandle::new(),
            refresh_task: Task::ready(()),
            debounce_task: Task::ready(()),
            snapshot: LanguageToolSnapshot::default(),
            visible_rows: Vec::new(),
            parent_by_id: HashMap::new(),
            expanded: HashSet::new(),
            selected: None,
            status: LanguageToolTreeStatus::Dormant,
            generation: 0,
            dirty: false,
        }
    }
}

impl LanguageToolTreeHost {
    pub fn with_focus_handle(focus_handle: FocusHandle) -> Self {
        Self {
            focus_handle: Some(focus_handle),
            ..Self::default()
        }
    }

    pub fn focus_handle(&self) -> Option<&FocusHandle> {
        self.focus_handle.as_ref()
    }

    pub fn scroll_handle(&self) -> &UniformListScrollHandle {
        &self.scroll_handle
    }

    pub fn replace_refresh_task(&mut self, task: Task<()>) {
        self.refresh_task = task;
    }

    pub fn replace_debounce_task(&mut self, task: Task<()>) {
        self.debounce_task = task;
    }

    pub fn cancel_debounce(&mut self) {
        self.debounce_task = Task::ready(());
    }

    pub fn reveal_selection(&self, strategy: ScrollStrategy) {
        if let Some(index) = self.selected_index() {
            self.scroll_handle.scroll_to_item(index, strategy);
        }
    }

    pub fn status(&self) -> &LanguageToolTreeStatus {
        &self.status
    }

    pub fn visible_rows(&self) -> &[VisibleLanguageToolRow] {
        &self.visible_rows
    }

    pub fn visible_rows_in_range(
        &self,
        range: std::ops::Range<usize>,
    ) -> &[VisibleLanguageToolRow] {
        let start = range.start.min(self.visible_rows.len());
        let end = range.end.min(self.visible_rows.len()).max(start);
        &self.visible_rows[start..end]
    }

    pub fn selected(&self) -> Option<&LanguageToolNodeId> {
        self.selected.as_ref()
    }

    pub fn expanded(&self) -> &HashSet<LanguageToolNodeId> {
        &self.expanded
    }

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn start_refresh(&mut self) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.status = if self.snapshot.roots.is_empty() {
            LanguageToolTreeStatus::Loading
        } else {
            LanguageToolTreeStatus::Refreshing
        };
        self.generation
    }

    pub fn apply_refresh(
        &mut self,
        generation: u64,
        result: anyhow::Result<LanguageToolSnapshot>,
    ) -> bool {
        if generation != self.generation {
            return false;
        }
        match result {
            Ok(snapshot) => self.replace_snapshot(snapshot),
            Err(error) if self.snapshot.roots.is_empty() => {
                self.status = LanguageToolTreeStatus::Error(error.to_string())
            }
            Err(error) => self.status = LanguageToolTreeStatus::StaleError(error.to_string()),
        }
        true
    }

    pub fn apply_provider_error(
        &mut self,
        generation: u64,
        status: LanguageToolProviderStatus,
    ) -> bool {
        if generation != self.generation {
            return false;
        }
        self.status = match status {
            LanguageToolProviderStatus::Loading => LanguageToolTreeStatus::Loading,
            LanguageToolProviderStatus::Disconnected(message)
                if !self.snapshot.roots.is_empty() =>
            {
                LanguageToolTreeStatus::DisconnectedStale(message)
            }
            LanguageToolProviderStatus::Disconnected(message) => {
                LanguageToolTreeStatus::Disconnected(message)
            }
            LanguageToolProviderStatus::Unsupported(message) => {
                LanguageToolTreeStatus::Unsupported(message)
            }
            LanguageToolProviderStatus::Restricted(message) => {
                LanguageToolTreeStatus::Restricted(message)
            }
            LanguageToolProviderStatus::Mismatch(message) => {
                LanguageToolTreeStatus::Mismatch(message)
            }
            LanguageToolProviderStatus::Empty(message) => LanguageToolTreeStatus::Empty(message),
            LanguageToolProviderStatus::Partial(message) => {
                LanguageToolTreeStatus::Partial(message)
            }
            LanguageToolProviderStatus::Stale(message) => LanguageToolTreeStatus::Stale(message),
            LanguageToolProviderStatus::Error(message) if !self.snapshot.roots.is_empty() => {
                LanguageToolTreeStatus::StaleError(message)
            }
            LanguageToolProviderStatus::Error(message) => LanguageToolTreeStatus::Error(message),
            LanguageToolProviderStatus::Current => LanguageToolTreeStatus::Current,
        };
        true
    }

    pub fn replace_snapshot(&mut self, snapshot: LanguageToolSnapshot) {
        let previous_rows = self
            .visible_rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<Vec<_>>();
        let previous_selected = self.selected.clone();
        let previous_parent_by_id = self.parent_by_id.clone();
        self.snapshot = snapshot;
        let all_ids = collect_ids(&self.snapshot.roots);
        self.expanded.retain(|id| all_ids.contains(id));
        self.rebuild_visible_rows();
        self.selected = reconcile_selection(
            previous_selected,
            &previous_rows,
            &previous_parent_by_id,
            &self.visible_rows,
        );
        self.status = match &self.snapshot.status {
            LanguageToolProviderStatus::Loading => LanguageToolTreeStatus::Loading,
            LanguageToolProviderStatus::Current => {
                if self.snapshot.roots.is_empty() {
                    LanguageToolTreeStatus::Empty("No items".to_string())
                } else {
                    LanguageToolTreeStatus::Current
                }
            }
            LanguageToolProviderStatus::Partial(message) => {
                LanguageToolTreeStatus::Partial(message.clone())
            }
            LanguageToolProviderStatus::Stale(message) => {
                LanguageToolTreeStatus::Stale(message.clone())
            }
            LanguageToolProviderStatus::Empty(message) => {
                LanguageToolTreeStatus::Empty(message.clone())
            }
            LanguageToolProviderStatus::Disconnected(message) => {
                if self.snapshot.roots.is_empty() {
                    LanguageToolTreeStatus::Disconnected(message.clone())
                } else {
                    LanguageToolTreeStatus::DisconnectedStale(message.clone())
                }
            }
            LanguageToolProviderStatus::Restricted(message) => {
                LanguageToolTreeStatus::Restricted(message.clone())
            }
            LanguageToolProviderStatus::Unsupported(message) => {
                LanguageToolTreeStatus::Unsupported(message.clone())
            }
            LanguageToolProviderStatus::Mismatch(message) => {
                LanguageToolTreeStatus::Mismatch(message.clone())
            }
            LanguageToolProviderStatus::Error(message) => {
                if self.snapshot.roots.is_empty() {
                    LanguageToolTreeStatus::Error(message.clone())
                } else {
                    LanguageToolTreeStatus::StaleError(message.clone())
                }
            }
        };
    }

    pub fn select(&mut self, id: LanguageToolNodeId) {
        if self.visible_rows.iter().any(|row| row.id == id) {
            self.selected = Some(id);
        }
    }

    pub fn select_previous(&mut self) {
        self.select_offset(-1);
    }

    pub fn select_next(&mut self) {
        self.select_offset(1);
    }

    pub fn select_first(&mut self) {
        self.selected = self.visible_rows.first().map(|row| row.id.clone());
    }

    pub fn select_last(&mut self) {
        self.selected = self.visible_rows.last().map(|row| row.id.clone());
    }

    pub fn select_parent(&mut self) {
        let Some(selected) = self.selected.as_ref() else {
            return;
        };
        if let Some(parent) = self.parent_by_id.get(selected) {
            self.selected = Some(parent.clone());
        }
    }

    pub fn select_first_child(&mut self) {
        let Some(selected_index) = self.selected_index() else {
            return;
        };
        let level = self.visible_rows[selected_index].level;
        if let Some(next) = self.visible_rows.get(selected_index + 1)
            && next.level == level + 1
        {
            self.selected = Some(next.id.clone());
        }
    }

    pub fn toggle_selected(&mut self) {
        let Some(selected) = self.selected.clone() else {
            return;
        };
        self.toggle(&selected);
    }

    pub fn toggle(&mut self, id: &LanguageToolNodeId) {
        if !self
            .visible_rows
            .iter()
            .any(|row| row.id == *id && row.is_branch)
        {
            return;
        }
        if !self.expanded.remove(id) {
            self.expanded.insert(id.clone());
        }
        self.rebuild_visible_rows();
    }

    pub fn can_expand_all(&self) -> bool {
        branch_ids(&self.snapshot.roots)
            .iter()
            .any(|id| !self.expanded.contains(id))
    }

    pub fn can_collapse_all(&self) -> bool {
        !self.expanded.is_empty()
    }

    pub fn can_refresh(&self) -> bool {
        !matches!(
            self.status,
            LanguageToolTreeStatus::Loading
                | LanguageToolTreeStatus::Refreshing
                | LanguageToolTreeStatus::Unsupported(_)
                | LanguageToolTreeStatus::Mismatch(_)
                | LanguageToolTreeStatus::Disconnected(_)
                | LanguageToolTreeStatus::DisconnectedStale(_)
        )
    }

    pub fn can_activate(&self) -> bool {
        !matches!(
            self.status,
            LanguageToolTreeStatus::Unsupported(_)
                | LanguageToolTreeStatus::Mismatch(_)
                | LanguageToolTreeStatus::Disconnected(_)
                | LanguageToolTreeStatus::DisconnectedStale(_)
        )
    }

    pub fn expand_all(&mut self) {
        self.expanded = branch_ids(&self.snapshot.roots);
        self.rebuild_visible_rows();
    }

    pub fn collapse_all(&mut self) {
        self.expanded.clear();
        self.rebuild_visible_rows();
    }

    fn select_offset(&mut self, offset: isize) {
        if self.visible_rows.is_empty() {
            self.selected = None;
            return;
        }
        let index = self.selected_index().unwrap_or(0);
        let next = index
            .saturating_add_signed(offset)
            .min(self.visible_rows.len().saturating_sub(1));
        self.selected = Some(self.visible_rows[next].id.clone());
    }

    pub fn selected_index(&self) -> Option<usize> {
        let selected = self.selected.as_ref()?;
        self.visible_rows.iter().position(|row| &row.id == selected)
    }

    fn rebuild_visible_rows(&mut self) {
        self.visible_rows.clear();
        self.parent_by_id.clear();
        flatten_nodes(
            &self.snapshot.roots,
            None,
            1,
            &self.expanded,
            &mut self.parent_by_id,
            &mut self.visible_rows,
        );
    }
}

fn flatten_nodes(
    nodes: &[LanguageToolNode],
    parent_id: Option<&LanguageToolNodeId>,
    level: usize,
    expanded: &HashSet<LanguageToolNodeId>,
    parent_by_id: &mut HashMap<LanguageToolNodeId, LanguageToolNodeId>,
    rows: &mut Vec<VisibleLanguageToolRow>,
) {
    for node in nodes {
        if let Some(parent_id) = parent_id {
            parent_by_id.insert(node.id.clone(), parent_id.clone());
        }
        let is_branch = !node.children.is_empty();
        let is_expanded = is_branch && expanded.contains(&node.id);
        rows.push(VisibleLanguageToolRow {
            id: node.id.clone(),
            parent_id: parent_id.cloned(),
            level,
            is_branch,
            is_expanded,
            node: node.clone(),
        });
        if is_expanded {
            flatten_nodes(
                &node.children,
                Some(&node.id),
                level + 1,
                expanded,
                parent_by_id,
                rows,
            );
        }
    }
}

fn collect_ids(nodes: &[LanguageToolNode]) -> HashSet<LanguageToolNodeId> {
    let mut ids = HashSet::new();
    for node in nodes {
        ids.insert(node.id.clone());
        ids.extend(collect_ids(&node.children));
    }
    ids
}

fn branch_ids(nodes: &[LanguageToolNode]) -> HashSet<LanguageToolNodeId> {
    let mut ids = HashSet::new();
    for node in nodes {
        if !node.children.is_empty() {
            ids.insert(node.id.clone());
            ids.extend(branch_ids(&node.children));
        }
    }
    ids
}

fn reconcile_selection(
    selected: Option<LanguageToolNodeId>,
    previous_rows: &[LanguageToolNodeId],
    previous_parent_by_id: &HashMap<LanguageToolNodeId, LanguageToolNodeId>,
    rows: &[VisibleLanguageToolRow],
) -> Option<LanguageToolNodeId> {
    let available: HashSet<&LanguageToolNodeId> = rows.iter().map(|row| &row.id).collect();
    let mut candidate = selected?;
    if available.contains(&candidate) {
        return Some(candidate);
    }
    while let Some(parent) = previous_parent_by_id.get(&candidate) {
        if available.contains(parent) {
            return Some(parent.clone());
        }
        candidate = parent.clone();
    }
    let previous_index = previous_rows
        .iter()
        .position(|id| id == &candidate)
        .unwrap_or(0);
    rows.get(previous_index.min(rows.len().saturating_sub(1)))
        .map(|row| row.id.clone())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use gpui::{Context, Render, TestAppContext, VisualTestContext, Window, px, size};

    use super::*;

    fn node(id: &str, children: Vec<LanguageToolNode>) -> LanguageToolNode {
        LanguageToolNode {
            id: LanguageToolNodeId(id.to_string()),
            label: id.to_string(),
            secondary_label: None,
            icon: None,
            accessibility_label: id.to_string(),
            children,
            enabled: true,
            activation_label: Some("Open".to_string()),
        }
    }

    struct RenderedTree {
        rows: Vec<VisibleLanguageToolRow>,
        scroll_handle: UniformListScrollHandle,
    }

    impl Render for RenderedTree {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            v_flex().size_full().child(language_tool_tree(
                self.rows.clone(),
                None,
                None,
                self.scroll_handle.clone(),
                |_, _, _, _| {},
                |_, _, _| {},
                |_, _, _, _| {},
            ))
        }
    }

    #[gpui::test]
    fn populated_tree_list_and_rows_have_visible_bounds(cx: &mut TestAppContext) {
        cx.update(|cx| {
            settings::init(cx);
            theme_settings::init(theme::LoadThemes::JustBase, cx);
        });

        let mut host = LanguageToolTreeHost::default();
        host.replace_snapshot(LanguageToolSnapshot {
            roots: vec![node("visible-root", Vec::new())],
            status: LanguageToolProviderStatus::Current,
        });
        let rows = host.visible_rows().to_vec();
        let scroll_handle = host.scroll_handle().clone();
        let rendered_scroll_handle = scroll_handle.clone();
        let window = cx.open_window(size(px(320.), px(240.)), |_, _| RenderedTree {
            rows,
            scroll_handle,
        });
        let cx = VisualTestContext::from_window(*window, cx);
        cx.run_until_parked();

        let scroll_state = rendered_scroll_handle.0.borrow();
        let list_bounds = scroll_state.base_handle.bounds();
        assert!(list_bounds.size.height > px(0.));
        let item_size = scroll_state
            .last_item_size
            .expect("the projected root row should be measured");
        assert!(item_size.item.height > px(0.));
    }

    #[test]
    fn arbitrary_tree_expansion_and_selection_are_stable() {
        let snapshot = LanguageToolSnapshot {
            roots: vec![node(
                "root",
                vec![node("child", vec![node("leaf", vec![])])],
            )],
            status: LanguageToolProviderStatus::Current,
        };
        let mut host = LanguageToolTreeHost::default();
        host.replace_snapshot(snapshot.clone());
        host.select_first();
        host.expand_all();
        host.select(LanguageToolNodeId("leaf".to_string()));
        host.replace_snapshot(snapshot);
        assert_eq!(host.visible_rows().len(), 3);
        assert_eq!(
            host.selected(),
            Some(&LanguageToolNodeId("leaf".to_string()))
        );
    }

    #[test]
    fn stale_generations_do_not_replace_current_snapshot() {
        let mut host = LanguageToolTreeHost::default();
        let old_generation = host.start_refresh();
        let current_generation = host.start_refresh();
        assert!(!host.apply_refresh(
            old_generation,
            Ok(LanguageToolSnapshot {
                roots: vec![node("old", vec![])],
                status: LanguageToolProviderStatus::Current,
            })
        ));
        assert!(host.apply_refresh(
            current_generation,
            Ok(LanguageToolSnapshot {
                roots: vec![node("current", vec![])],
                status: LanguageToolProviderStatus::Current,
            })
        ));
        assert_eq!(
            host.visible_rows()[0].id,
            LanguageToolNodeId("current".to_string())
        );
    }

    #[test]
    fn rust_workspace_large_model_flattens_visible_ranges_deterministically() {
        let roots = (0..10_000)
            .map(|index| node(&format!("node-{index:05}"), vec![]))
            .collect();
        let mut host = LanguageToolTreeHost::default();
        host.replace_snapshot(LanguageToolSnapshot {
            roots,
            status: LanguageToolProviderStatus::Current,
        });
        assert_eq!(host.visible_rows().len(), 10_000);
        assert_eq!(host.visible_rows()[9_999].node.label, "node-09999");
        assert_eq!(host.visible_rows_in_range(400..425).len(), 25);
    }

    #[test]
    fn keyboard_traversal_and_selection_fallback_cover_arbitrary_depth() {
        let mut host = LanguageToolTreeHost::default();
        host.replace_snapshot(LanguageToolSnapshot {
            roots: vec![node(
                "root",
                vec![node("child", vec![node("leaf", vec![])])],
            )],
            status: LanguageToolProviderStatus::Current,
        });
        host.select_first();
        host.toggle_selected();
        host.select_first_child();
        host.toggle_selected();
        host.select_last();
        assert_eq!(host.selected().map(|id| id.0.as_str()), Some("leaf"));
        host.replace_snapshot(LanguageToolSnapshot {
            roots: vec![node("root", vec![node("child", vec![])])],
            status: LanguageToolProviderStatus::Partial("partial".to_string()),
        });
        assert_eq!(host.selected().map(|id| id.0.as_str()), Some("child"));
        assert_eq!(
            host.status(),
            &LanguageToolTreeStatus::Partial("partial".to_string())
        );
        host.select_parent();
        assert_eq!(host.selected().map(|id| id.0.as_str()), Some("root"));
    }

    #[test]
    fn action_availability_tracks_tree_and_connection_state() {
        let mut host = LanguageToolTreeHost::default();
        assert!(host.can_refresh());
        assert!(!host.can_expand_all());
        assert!(!host.can_collapse_all());

        host.replace_snapshot(LanguageToolSnapshot {
            roots: vec![node("root", vec![node("child", vec![])])],
            status: LanguageToolProviderStatus::Current,
        });
        assert!(host.can_expand_all());
        host.toggle(&LanguageToolNodeId("root".to_string()));
        assert!(!host.can_expand_all());
        assert!(host.can_collapse_all());

        let generation = host.start_refresh();
        assert!(!host.can_refresh());
        assert!(host.apply_provider_error(
            generation,
            LanguageToolProviderStatus::Disconnected("offline".to_string()),
        ));
        assert!(!host.can_refresh());
        assert!(!host.can_activate());
        assert_eq!(status_message(host.status()).as_deref(), Some("offline"));
    }

    #[test]
    fn dirty_and_stale_error_states_preserve_current_rows() {
        let mut host = LanguageToolTreeHost::default();
        host.replace_snapshot(LanguageToolSnapshot {
            roots: vec![node("current", vec![])],
            status: LanguageToolProviderStatus::Current,
        });
        host.mark_dirty();
        assert!(host.take_dirty());
        assert!(!host.take_dirty());
        let generation = host.start_refresh();
        assert!(host.apply_refresh(generation, Err(anyhow::anyhow!("offline"))));
        assert_eq!(host.visible_rows().len(), 1);
        assert_eq!(
            host.status(),
            &LanguageToolTreeStatus::StaleError("offline".to_string())
        );
    }

    #[gpui::test]
    async fn manual_refresh_supersedes_a_timed_generation(cx: &mut TestAppContext) {
        let mut host = LanguageToolTreeHost::default();
        let debounced_generation = host.start_refresh();
        cx.background_executor.timer(Duration::from_millis(1)).await;
        let manual_generation = host.start_refresh();
        assert!(!host.apply_refresh(
            debounced_generation,
            Ok(LanguageToolSnapshot {
                roots: vec![node("obsolete", vec![])],
                status: LanguageToolProviderStatus::Current,
            }),
        ));
        assert!(host.apply_refresh(
            manual_generation,
            Ok(LanguageToolSnapshot {
                roots: vec![node("current", vec![])],
                status: LanguageToolProviderStatus::Current,
            }),
        ));
    }
}
