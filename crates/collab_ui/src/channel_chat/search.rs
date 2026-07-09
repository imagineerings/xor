use super::*;
use chrono::{NaiveDate, Utc};

#[derive(Default)]
pub(super) struct SearchState {
    pub(super) active: bool,
    pub(super) query: String,
    pub(super) clean_query: String,
    pub(super) filters: SearchFilters,
    pub(super) results: Vec<proto::SearchResult>,
    pub(super) done: bool,
    pub(super) loading: bool,
    pub(super) loading_more: bool,
    pub(super) error: Option<SharedString>,
    pub(super) selected_result_index: Option<usize>,
    pub(super) request_serial: u64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct SearchFilters {
    pub(super) channel_name: Option<String>,
    pub(super) username: Option<String>,
    pub(super) after_date: Option<SearchDateFilter>,
    pub(super) before_date: Option<SearchDateFilter>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SearchDateFilter {
    pub(super) text: String,
    pub(super) timestamp: u64,
}

#[derive(Clone, Copy)]
enum SearchFilterKind {
    Channel,
    User,
    After,
    Before,
}

impl SearchFilters {
    fn parse(query: &str) -> (Self, String, Option<SharedString>) {
        let mut filters = SearchFilters::default();
        let mut terms = Vec::new();
        let mut error = None;

        for token in tokenize_search_query(query) {
            if token.quoted {
                terms.push(token.text);
                continue;
            }

            if let Some(value) = token
                .text
                .strip_prefix("in:")
                .filter(|value| !value.is_empty())
            {
                filters.channel_name = Some(value.to_string());
            } else if let Some(value) = token
                .text
                .strip_prefix("from:")
                .filter(|value| !value.is_empty())
            {
                filters.username = Some(value.to_string());
            } else if let Some(value) = token
                .text
                .strip_prefix("after:")
                .filter(|value| !value.is_empty())
            {
                match parse_search_date(value, false) {
                    Some(timestamp) => {
                        filters.after_date = Some(SearchDateFilter {
                            text: value.to_string(),
                            timestamp,
                        });
                    }
                    None => error = Some(format!("Invalid after date: {value}").into()),
                }
            } else if let Some(value) = token
                .text
                .strip_prefix("before:")
                .filter(|value| !value.is_empty())
            {
                match parse_search_date(value, true) {
                    Some(timestamp) => {
                        filters.before_date = Some(SearchDateFilter {
                            text: value.to_string(),
                            timestamp,
                        });
                    }
                    None => error = Some(format!("Invalid before date: {value}").into()),
                }
            } else {
                terms.push(token.text);
            }
        }

        (filters, terms.join(" "), error)
    }

    fn request_after(&self) -> Option<u64> {
        self.after_date.as_ref().map(|date| date.timestamp)
    }

    fn request_before(&self) -> Option<u64> {
        self.before_date.as_ref().map(|date| date.timestamp)
    }
}

struct SearchToken {
    text: String,
    quoted: bool,
}

fn tokenize_search_query(query: &str) -> Vec<SearchToken> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut token_was_quoted = false;

    for character in query.chars() {
        match character {
            '"' => {
                if quoted {
                    tokens.push(SearchToken {
                        text: current.trim().to_string(),
                        quoted: true,
                    });
                    current.clear();
                    quoted = false;
                    token_was_quoted = false;
                } else {
                    if !current.trim().is_empty() {
                        tokens.push(SearchToken {
                            text: current.trim().to_string(),
                            quoted: token_was_quoted,
                        });
                    }
                    current.clear();
                    quoted = true;
                    token_was_quoted = true;
                }
            }
            character if character.is_whitespace() && !quoted => {
                if !current.trim().is_empty() {
                    tokens.push(SearchToken {
                        text: current.trim().to_string(),
                        quoted: token_was_quoted,
                    });
                }
                current.clear();
                token_was_quoted = false;
            }
            _ => current.push(character),
        }
    }

    if !current.trim().is_empty() {
        tokens.push(SearchToken {
            text: current.trim().to_string(),
            quoted: token_was_quoted || quoted,
        });
    }

    tokens
}

fn parse_search_date(value: &str, end_of_day: bool) -> Option<u64> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()?;
    let naive = if end_of_day {
        date.and_hms_milli_opt(23, 59, 59, 999)?
    } else {
        date.and_hms_milli_opt(0, 0, 0, 0)?
    };
    Utc.from_utc_datetime(&naive)
        .timestamp_millis()
        .try_into()
        .ok()
}

impl ChannelChat {
    pub(super) fn toggle_search(
        &mut self,
        _: &ToggleSearch,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.search_state.active = !self.search_state.active;
        if self.search_state.active {
            window.focus(&self.search_editor.focus_handle(cx), cx);
            self.schedule_search(cx);
        }
        cx.notify();
    }

    pub(super) fn close_search(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.search_state.active {
            return false;
        }

        self.search_state.active = false;
        self.search_state.error = None;
        self.search_state.selected_result_index = None;
        self.search_state.loading = false;
        self.search_state.loading_more = false;
        self.pending_search.take();
        cx.notify();
        true
    }

    pub(super) fn schedule_search(&mut self, cx: &mut Context<Self>) {
        let query = self.search_editor.read(cx).text(cx);
        let (filters, clean_query, parse_error) = SearchFilters::parse(&query);
        self.search_state.query = query;
        self.search_state.filters = filters;
        self.search_state.clean_query = clean_query.trim().to_string();
        self.search_state.request_serial = self.search_state.request_serial.saturating_add(1);
        let request_serial = self.search_state.request_serial;

        if let Some(error) = parse_error {
            self.search_state.results.clear();
            self.search_state.done = true;
            self.search_state.loading = false;
            self.search_state.loading_more = false;
            self.search_state.error = Some(error);
            self.search_state.selected_result_index = None;
            cx.notify();
            return;
        }

        if self.search_state.query.trim().is_empty() {
            self.search_state.results.clear();
            self.search_state.done = true;
            self.search_state.loading = false;
            self.search_state.loading_more = false;
            self.search_state.error = None;
            self.search_state.selected_result_index = None;
            cx.notify();
            return;
        }

        if self.search_state.clean_query.chars().count() < 2 {
            self.search_state.results.clear();
            self.search_state.done = true;
            self.search_state.loading = false;
            self.search_state.loading_more = false;
            self.search_state.error = Some("Query must be at least 2 characters".into());
            self.search_state.selected_result_index = None;
            cx.notify();
            return;
        }

        self.search_state.results.clear();
        self.search_state.done = false;
        self.search_state.loading = true;
        self.search_state.loading_more = false;
        self.search_state.error = None;
        self.search_state.selected_result_index = None;
        cx.notify();

        let client = self.client.clone();
        let request = self.search_request(None);
        self.pending_search = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SEARCH_DEBOUNCE).await;
            let result = client.search_channel_messages(request).await;
            this.update(cx, |this, cx| {
                if this.search_state.request_serial != request_serial {
                    return;
                }
                this.pending_search.take();
                this.search_state.loading = false;
                match result {
                    Ok(response) => {
                        this.search_state.results = response.results;
                        this.search_state.done = response.done;
                        this.search_state.selected_result_index =
                            (!this.search_state.results.is_empty()).then_some(0);
                        this.search_state.error = None;
                    }
                    Err(error) => {
                        this.search_state.results.clear();
                        this.search_state.done = true;
                        this.search_state.selected_result_index = None;
                        this.search_state.error =
                            Some(format!("Failed to load search results: {error}").into());
                    }
                }
                cx.notify();
            })
            .log_err();
        }));
    }

    fn search_request(&self, before_message_id: Option<u64>) -> SearchChannelMessages {
        SearchChannelMessages {
            channel_id: None,
            query: self.search_state.clean_query.clone(),
            before_message_id,
            limit: SEARCH_PAGE_SIZE,
            filter_channel: self.search_state.filters.channel_name.clone(),
            filter_user: self.search_state.filters.username.clone(),
            filter_after: self.search_state.filters.request_after(),
            filter_before: self.search_state.filters.request_before(),
        }
    }

    pub(super) fn load_more_search_results(&mut self, cx: &mut Context<Self>) {
        if self.search_state.done || self.search_state.loading || self.search_state.loading_more {
            return;
        }
        let Some(before_message_id) = self
            .search_state
            .results
            .last()
            .and_then(|result| result.message.as_ref())
            .map(|message| message.id)
        else {
            return;
        };

        self.search_state.loading_more = true;
        self.search_state.error = None;
        self.search_state.request_serial = self.search_state.request_serial.saturating_add(1);
        let request_serial = self.search_state.request_serial;
        let request = self.search_request(Some(before_message_id));
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let result = client.search_channel_messages(request).await;
            this.update(cx, |this, cx| {
                if this.search_state.request_serial != request_serial {
                    return;
                }
                this.search_state.loading_more = false;
                match result {
                    Ok(response) => {
                        let had_selection = this.search_state.selected_result_index.is_some();
                        this.search_state.results.extend(response.results);
                        this.search_state.done = response.done;
                        if !had_selection && !this.search_state.results.is_empty() {
                            this.search_state.selected_result_index = Some(0);
                        }
                        this.search_state.error = None;
                    }
                    Err(error) => {
                        this.search_state.error =
                            Some(format!("Failed to load search results: {error}").into());
                    }
                }
                cx.notify();
            })
            .log_err();
        })
        .detach();
    }

    pub(super) fn select_next_search_result(
        &mut self,
        _: &SelectNext,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.search_state.active || self.search_state.results.is_empty() {
            return;
        }

        let last_index = self.search_state.results.len() - 1;
        self.search_state.selected_result_index = Some(
            self.search_state
                .selected_result_index
                .map(|index| index.saturating_add(1).min(last_index))
                .unwrap_or(0),
        );
        cx.notify();
    }

    pub(super) fn select_previous_search_result(
        &mut self,
        _: &SelectPrevious,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.search_state.active || self.search_state.results.is_empty() {
            return;
        }

        self.search_state.selected_result_index = Some(
            self.search_state
                .selected_result_index
                .map(|index| index.saturating_sub(1))
                .unwrap_or(0),
        );
        cx.notify();
    }

    pub(super) fn open_selected_search_result(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.search_state.active {
            return false;
        }

        let Some(result) = self
            .search_state
            .selected_result_index
            .and_then(|index| self.search_state.results.get(index))
            .cloned()
        else {
            return false;
        };
        self.open_search_result(result, window, cx);
        true
    }

    fn open_search_result(
        &mut self,
        result: proto::SearchResult,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(message_id) = result.message.as_ref().map(|message| message.id) else {
            return;
        };
        let channel_id = ChannelId(result.channel_id);
        self.close_search(cx);

        if channel_id == self.channel_id {
            self.highlight_search_message(message_id, cx);
            return;
        }

        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        let open_channel = ChannelChat::open(channel_id, workspace, window, cx);
        cx.spawn_in(window, async move |_, cx| {
            let chat = open_channel.await?;
            chat.update(cx, |chat, cx| {
                chat.highlight_search_message(message_id, cx);
            });
            anyhow::Ok(())
        })
        .detach_and_log_err(cx);
    }

    fn highlight_search_message(&mut self, message_id: u64, cx: &mut Context<Self>) {
        self.highlighted_search_message_id = Some(message_id);
        cx.notify();

        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(2)).await;
            this.update(cx, |this, cx| {
                if this.highlighted_search_message_id == Some(message_id) {
                    this.highlighted_search_message_id = None;
                    cx.notify();
                }
            })
            .log_err();
        })
        .detach();
    }

    fn remove_search_filter(
        &mut self,
        filter: SearchFilterKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut filters = self.search_state.filters.clone();
        match filter {
            SearchFilterKind::Channel => filters.channel_name = None,
            SearchFilterKind::User => filters.username = None,
            SearchFilterKind::After => filters.after_date = None,
            SearchFilterKind::Before => filters.before_date = None,
        }

        let mut query_parts = Vec::new();
        if !self.search_state.clean_query.is_empty() {
            query_parts.push(self.search_state.clean_query.clone());
        }
        if let Some(channel_name) = filters.channel_name.as_ref() {
            query_parts.push(format!("in:{channel_name}"));
        }
        if let Some(username) = filters.username.as_ref() {
            query_parts.push(format!("from:{username}"));
        }
        if let Some(after_date) = filters.after_date.as_ref() {
            query_parts.push(format!("after:{}", after_date.text));
        }
        if let Some(before_date) = filters.before_date.as_ref() {
            query_parts.push(format!("before:{}", before_date.text));
        }

        self.search_editor.update(cx, |editor, cx| {
            editor.set_text(query_parts.join(" "), window, cx)
        });
        self.schedule_search(cx);
    }

    pub(super) fn render_search_header(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let channel_name = self
            .channel(cx)
            .map(|channel| channel.name.clone())
            .unwrap_or_else(|| "Channel".into());

        v_flex()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .child(
                h_flex()
                    .h(px(40.))
                    .px_3()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::Hash).size(IconSize::Small))
                            .child(Label::new(channel_name).weight(gpui::FontWeight::MEDIUM)),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                IconButton::new("open-scheduled-messages", IconName::Clock)
                                    .icon_size(IconSize::Small)
                                    .icon_color(if self.scheduled_messages_panel.is_some() {
                                        Color::Accent
                                    } else {
                                        Color::Muted
                                    })
                                    .on_click(cx.listener(Self::open_scheduled_messages_panel))
                                    .tooltip(Tooltip::text("Scheduled messages")),
                            )
                            .child(
                                IconButton::new(
                                    "toggle-channel-message-search",
                                    IconName::ListFilter,
                                )
                                .icon_size(IconSize::Small)
                                .icon_color(if self.search_state.active {
                                    Color::Accent
                                } else {
                                    Color::Muted
                                })
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.toggle_search(&ToggleSearch, window, cx);
                                }))
                                .tooltip(Tooltip::text("Search messages")),
                            ),
                    ),
            )
            .when(self.search_state.active, |this| {
                this.child(self.render_search_input(window, cx))
            })
            .into_any_element()
    }

    fn render_search_input(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        v_flex()
            .key_context("ChannelMessageSearch")
            .gap_2()
            .px_3()
            .pb_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(IconName::ListFilter).size(IconSize::Small))
                    .child(div().flex_1().child(self.search_editor.clone()))
                    .when(self.search_state.loading, |this| {
                        this.child(Icon::new(IconName::LoadCircle).size(IconSize::Small))
                    })
                    .child(
                        IconButton::new("close-channel-message-search", IconName::Close)
                            .icon_size(IconSize::Small)
                            .icon_color(Color::Muted)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.close_search(cx);
                            }))
                            .tooltip(Tooltip::text("Close search")),
                    ),
            )
            .when(self.has_search_filters(), |this| {
                this.child(self.render_search_filter_chips(window, cx))
            })
            .when_some(self.search_state.error.clone(), |this, error| {
                this.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Label::new(error)
                                .size(LabelSize::XSmall)
                                .color(Color::Error),
                        )
                        .when(self.search_state.clean_query.chars().count() >= 2, |this| {
                            this.child(
                                Button::new("retry-channel-message-search", "Retry")
                                    .label_size(LabelSize::XSmall)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.schedule_search(cx);
                                    })),
                            )
                        }),
                )
            })
            .into_any_element()
    }

    fn has_search_filters(&self) -> bool {
        self.search_state.filters.channel_name.is_some()
            || self.search_state.filters.username.is_some()
            || self.search_state.filters.after_date.is_some()
            || self.search_state.filters.before_date.is_some()
    }

    fn render_search_filter_chips(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let mut chips = Vec::new();
        if let Some(channel_name) = self.search_state.filters.channel_name.clone() {
            chips.push(self.render_search_filter_chip(
                format!("in: {channel_name}"),
                SearchFilterKind::Channel,
                cx,
            ));
        }
        if let Some(username) = self.search_state.filters.username.clone() {
            chips.push(self.render_search_filter_chip(
                format!("from: {username}"),
                SearchFilterKind::User,
                cx,
            ));
        }
        if let Some(after_date) = self.search_state.filters.after_date.clone() {
            chips.push(self.render_search_filter_chip(
                format!("after: {}", after_date.text),
                SearchFilterKind::After,
                cx,
            ));
        }
        if let Some(before_date) = self.search_state.filters.before_date.clone() {
            chips.push(self.render_search_filter_chip(
                format!("before: {}", before_date.text),
                SearchFilterKind::Before,
                cx,
            ));
        }

        h_flex()
            .gap_1()
            .flex_wrap()
            .children(chips)
            .into_any_element()
    }

    fn render_search_filter_chip(
        &self,
        label: String,
        filter: SearchFilterKind,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let filter_id: u32 = match filter {
            SearchFilterKind::Channel => 0,
            SearchFilterKind::User => 1,
            SearchFilterKind::After => 2,
            SearchFilterKind::Before => 3,
        };
        h_flex()
            .id(("channel-search-filter-chip", filter_id))
            .gap_1()
            .items_center()
            .px_2()
            .py_1()
            .rounded_sm()
            .bg(cx.theme().colors().element_background)
            .border_1()
            .border_color(cx.theme().colors().border_variant)
            .child(Label::new(label).size(LabelSize::XSmall))
            .child(
                IconButton::new(("remove-channel-search-filter", filter_id), IconName::Close)
                    .icon_size(IconSize::XSmall)
                    .icon_color(Color::Muted)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.remove_search_filter(filter, window, cx);
                    })),
            )
            .into_any_element()
    }

    pub(super) fn render_search_results_panel(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        v_flex()
            .id("channel-message-search-results")
            .max_h(px(320.))
            .overflow_y_scroll()
            .border_b_1()
            .border_color(cx.theme().colors().border)
            .bg(cx.theme().colors().editor_background)
            .when(
                !self.search_state.loading
                    && self.search_state.done
                    && self.search_state.results.is_empty()
                    && self.search_state.error.is_none()
                    && !self.search_state.clean_query.is_empty(),
                |this| {
                    this.child(
                        v_flex()
                            .gap_1()
                            .p_3()
                            .child(Label::new("No results found").size(LabelSize::Small))
                            .child(
                                Label::new("Try fewer filters or a different phrase.")
                                    .size(LabelSize::XSmall)
                                    .color(Color::Muted),
                            ),
                    )
                },
            )
            .children(
                self.search_state
                    .results
                    .clone()
                    .into_iter()
                    .enumerate()
                    .map(|(index, result)| self.render_search_result(index, result, window, cx)),
            )
            .when(
                !self.search_state.done && !self.search_state.results.is_empty(),
                |this| {
                    this.child(
                        h_flex().p_2().justify_center().child(
                            Button::new(
                                "load-more-channel-message-search-results",
                                if self.search_state.loading_more {
                                    "Loading..."
                                } else {
                                    "Load more"
                                },
                            )
                            .label_size(LabelSize::Small)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.load_more_search_results(cx);
                            })),
                        ),
                    )
                },
            )
            .into_any_element()
    }

    fn render_search_result(
        &mut self,
        index: usize,
        result: proto::SearchResult,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let Some(message) = result.message.as_ref() else {
            return div().into_any_element();
        };
        let selected = self.search_state.selected_result_index == Some(index);
        let header = format!(
            "#{} · @{} · {}",
            result.channel_name,
            result.sender_name,
            format_timestamp(message.timestamp)
        );

        v_flex()
            .id(("channel-message-search-result", message.id))
            .gap_1()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().colors().border_variant)
            .when(selected, |this| {
                this.bg(cx.theme().colors().element_selected)
                    .border_color(cx.theme().colors().text_accent)
            })
            .hover(|style| style.bg(cx.theme().colors().element_hover))
            .on_click(cx.listener({
                let result = result.clone();
                move |this, _, window, cx| {
                    this.open_search_result(result.clone(), window, cx);
                }
            }))
            .child(
                Label::new(header)
                    .size(LabelSize::XSmall)
                    .color(Color::Muted),
            )
            .child(self.render_search_result_body(&message.body, cx))
            .into_any_element()
    }

    fn render_search_result_body(&self, body: &str, cx: &mut Context<Self>) -> gpui::AnyElement {
        let ranges = search_match_ranges(body, &self.search_state.clean_query);
        if ranges.is_empty() {
            return Label::new(body.to_string())
                .size(LabelSize::Small)
                .into_any_element();
        }

        let mut cursor = 0;
        let mut chunks = Vec::new();
        for (start, end) in ranges {
            if start > cursor
                && let Some(text) = body.get(cursor..start)
            {
                chunks.push(
                    Label::new(text.to_string())
                        .size(LabelSize::Small)
                        .into_any_element(),
                );
            }
            if let Some(text) = body.get(start..end) {
                chunks.push(
                    div()
                        .rounded_sm()
                        .bg(cx.theme().colors().element_selected)
                        .child(Label::new(text.to_string()).size(LabelSize::Small))
                        .into_any_element(),
                );
            }
            cursor = end;
        }
        if let Some(text) = body.get(cursor..) {
            chunks.push(
                Label::new(text.to_string())
                    .size(LabelSize::Small)
                    .into_any_element(),
            );
        }

        h_flex().flex_wrap().children(chunks).into_any_element()
    }
}

fn search_match_ranges(body: &str, query: &str) -> Vec<(usize, usize)> {
    let lower_body = body.to_lowercase();
    let mut ranges = Vec::new();

    for term in query.split_whitespace().map(str::to_lowercase) {
        if term.len() < 2 {
            continue;
        }
        for (start, _) in lower_body.match_indices(&term) {
            ranges.push((start, start + term.len()));
        }
    }

    ranges.sort_unstable();
    ranges.dedup();
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_search_filters() {
        let (filters, query, error) =
            SearchFilters::parse("deploy in:general from:alice after:2026-01-02 before:2026-01-03");

        assert_eq!(query, "deploy");
        assert!(error.is_none());
        assert_eq!(filters.channel_name.as_deref(), Some("general"));
        assert_eq!(filters.username.as_deref(), Some("alice"));
        assert_eq!(
            filters.after_date.as_ref().map(|date| date.text.as_str()),
            Some("2026-01-02")
        );
        assert_eq!(
            filters.before_date.as_ref().map(|date| date.text.as_str()),
            Some("2026-01-03")
        );
    }

    #[test]
    fn quoted_filter_syntax_stays_literal() {
        let (filters, query, error) = SearchFilters::parse("\"in:general from:alice\" deploy");

        assert_eq!(query, "in:general from:alice deploy");
        assert!(error.is_none());
        assert_eq!(filters, SearchFilters::default());
    }

    #[test]
    fn invalid_date_returns_error() {
        let (_, _, error) = SearchFilters::parse("deploy after:not-a-date");

        assert!(error.is_some());
    }

    #[test]
    fn finds_body_match_ranges() {
        let ranges = search_match_ranges("Deploy alpha then deploy beta", "deploy beta");

        assert_eq!(ranges, vec![(0, 6), (18, 24), (25, 29)]);
    }
}
