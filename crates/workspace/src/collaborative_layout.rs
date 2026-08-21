use gpui::{
    AnyView, Bounds, Context, DragMoveEvent, FocusHandle, Hsla, MouseButton, Point, Render, Role,
    canvas, px,
};
use ui::{Window, prelude::*};

use crate::{
    collaborative_accessibility::{REVIEW_LABEL, TIMELINE_LABEL},
    collaborative_layout_persistence::{CollaborativeLayoutState, MIN_REVIEW_WIDTH},
};

const MIN_TIMELINE_WIDTH: Pixels = px(480.);
const RESIZE_HANDLE_WIDTH: Pixels = px(6.);

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CollaborativeLayoutGeometry {
    pub(crate) timeline_width: Pixels,
    pub(crate) review_width: Pixels,
    pub(crate) review_visible: bool,
}

#[derive(Clone)]
struct DraggedCollaborativeReview;

impl Render for DraggedCollaborativeReview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        gpui::Empty
    }
}

pub(crate) struct CollaborativeLayout {
    bounds: Bounds<Pixels>,
    state: CollaborativeLayoutState,
    review_view: Option<AnyView>,
    timeline_focus_handle: FocusHandle,
    review_focus_handle: FocusHandle,
}

impl CollaborativeLayout {
    pub(crate) fn new(state: CollaborativeLayoutState, cx: &mut Context<Self>) -> Self {
        Self {
            bounds: Bounds::default(),
            state,
            review_view: None,
            timeline_focus_handle: cx.focus_handle(),
            review_focus_handle: cx.focus_handle(),
        }
    }

    pub(crate) fn set_review_view(&mut self, review_view: Option<AnyView>, cx: &mut Context<Self>) {
        let changed = self.review_view.as_ref().map(AnyView::entity_id)
            != review_view.as_ref().map(AnyView::entity_id);
        if changed {
            self.review_view = review_view;
            cx.notify();
        }
    }

    pub(crate) fn review_requested(&self) -> bool {
        self.state.review_requested()
    }

    pub(crate) fn timeline_focus_handle(&self) -> FocusHandle {
        self.timeline_focus_handle.clone()
    }

    pub(crate) fn review_focus_handle(&self) -> Option<FocusHandle> {
        let geometry = Self::geometry_for(
            self.bounds.size.width,
            self.state.review_requested(),
            px(self.state.review_width()),
        );
        geometry
            .review_visible
            .then(|| self.review_focus_handle.clone())
    }

    pub(crate) fn state(&self) -> CollaborativeLayoutState {
        self.state
    }

    pub(crate) fn rail_width(&self) -> Pixels {
        px(self.state.rail_width())
    }

    pub(crate) fn set_rail_width(&mut self, width: Pixels, cx: &mut Context<Self>) {
        let state = self.state.with_rail_width(f32::from(width));
        if state != self.state {
            self.state = state;
            cx.notify();
        }
    }

    pub(crate) fn reset_rail_width(&mut self, cx: &mut Context<Self>) {
        let state = self.state.reset_rail_width();
        if state != self.state {
            self.state = state;
            cx.notify();
        }
    }

    pub(crate) fn toggle_review(&mut self, cx: &mut Context<Self>) {
        self.state = self
            .state
            .with_review_requested(!self.state.review_requested());
        cx.notify();
    }

    pub(crate) fn geometry_for(
        available_width: Pixels,
        review_requested: bool,
        requested_review_width: Pixels,
    ) -> CollaborativeLayoutGeometry {
        let available_width = available_width.max(px(0.));
        let minimum_review_width = px(MIN_REVIEW_WIDTH);
        let minimum_expanded_width =
            MIN_TIMELINE_WIDTH + RESIZE_HANDLE_WIDTH + minimum_review_width;
        if !review_requested || available_width < minimum_expanded_width {
            return CollaborativeLayoutGeometry {
                timeline_width: available_width,
                review_width: px(0.),
                review_visible: false,
            };
        }

        let maximum_review_width = available_width - MIN_TIMELINE_WIDTH - RESIZE_HANDLE_WIDTH;
        let review_width = requested_review_width.clamp(minimum_review_width, maximum_review_width);
        CollaborativeLayoutGeometry {
            timeline_width: available_width - RESIZE_HANDLE_WIDTH - review_width,
            review_width,
            review_visible: true,
        }
    }

    fn available_width(&self, window: &Window) -> Pixels {
        if self.bounds.size.width > px(0.) {
            self.bounds.size.width
        } else {
            window.viewport_size().width
        }
    }

    fn resize_review(
        &mut self,
        bounds: Bounds<Pixels>,
        pointer: Point<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let geometry = Self::geometry_for(
            bounds.size.width,
            self.state.review_requested(),
            px(self.state.review_width()),
        );
        if !geometry.review_visible {
            return;
        }

        let maximum_review_width = bounds.size.width - MIN_TIMELINE_WIDTH - RESIZE_HANDLE_WIDTH;
        let review_width =
            (bounds.right() - pointer.x).clamp(px(MIN_REVIEW_WIDTH), maximum_review_width);
        let state = self.state.with_review_width(f32::from(review_width));
        if state != self.state {
            self.state = state;
            cx.notify();
        }
    }

    fn render_timeline(
        geometry: CollaborativeLayoutGeometry,
        background: Hsla,
        focus_handle: &FocusHandle,
    ) -> impl IntoElement {
        v_flex()
            .id("collaborative-timeline-region")
            .debug_selector(|| "COLLABORATIVE-TIMELINE-REGION".to_owned())
            .h_full()
            .w(geometry.timeline_width)
            .flex_none()
            .track_focus(focus_handle)
            .tab_index(0)
            .role(Role::Document)
            .aria_label(TIMELINE_LABEL)
            .overflow_hidden()
            .bg(background)
            .child(
                v_flex().size_full().items_center().justify_center().child(
                    Label::new("Timeline")
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
            )
    }

    fn render_review(
        geometry: CollaborativeLayoutGeometry,
        background: Hsla,
        review_view: Option<AnyView>,
        focus_handle: &FocusHandle,
    ) -> impl IntoElement {
        let review_available = review_view.is_some();
        v_flex()
            .id("collaborative-review-region")
            .debug_selector(|| "COLLABORATIVE-REVIEW-REGION".to_owned())
            .h_full()
            .w(geometry.review_width)
            .min_w(px(MIN_REVIEW_WIDTH))
            .flex_none()
            .track_focus(focus_handle)
            .tab_index(0)
            .role(Role::Complementary)
            .aria_label(REVIEW_LABEL)
            .overflow_hidden()
            .bg(background)
            .when_some(review_view, |this, review_view| {
                this.child(
                    div()
                        .id("collaborative-review-content")
                        .debug_selector(|| "COLLABORATIVE-REVIEW-CONTENT".to_owned())
                        .size_full()
                        .overflow_hidden()
                        .child(review_view),
                )
            })
            .when(!review_available, |this| {
                this.child(
                    v_flex().size_full().items_center().justify_center().child(
                        Label::new("Review Changes")
                            .size(LabelSize::Small)
                            .color(Color::Muted),
                    ),
                )
            })
    }

    fn render_resize_handle(border: Hsla) -> impl IntoElement {
        div()
            .id("collaborative-review-resize-handle")
            .debug_selector(|| "COLLABORATIVE-REVIEW-RESIZE-HANDLE".to_owned())
            .h_full()
            .w(RESIZE_HANDLE_WIDTH)
            .flex_none()
            .cursor_col_resize()
            .border_l_1()
            .border_color(border)
            .on_drag(DraggedCollaborativeReview, |dragged, _, _, cx| {
                cx.stop_propagation();
                cx.new(|_| dragged.clone())
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
    }

    #[cfg(test)]
    pub(crate) fn test_review_width(&self) -> Pixels {
        px(self.state.review_width())
    }
}

impl Render for CollaborativeLayout {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let geometry = Self::geometry_for(
            self.available_width(window),
            self.state.review_requested(),
            px(self.state.review_width()),
        );
        let colors = cx.theme().colors();
        let timeline_background = colors.background;
        let review_background = colors.panel_background;
        let border = colors.border;
        let layout = cx.entity();
        let review_view = self.review_view.clone();

        h_flex()
            .id("collaborative-layout")
            .debug_selector(|| "COLLABORATIVE-LAYOUT".to_owned())
            .relative()
            .size_full()
            .overflow_hidden()
            .on_drag_move(cx.listener(
                |this, event: &DragMoveEvent<DraggedCollaborativeReview>, _, cx| {
                    this.resize_review(event.bounds, event.event.position, cx);
                },
            ))
            .child(
                canvas(
                    move |bounds, _, cx| {
                        layout.update(cx, |layout, cx| {
                            if layout.bounds != bounds {
                                layout.bounds = bounds;
                                cx.notify();
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .child(Self::render_timeline(
                geometry,
                timeline_background,
                &self.timeline_focus_handle,
            ))
            .when(geometry.review_visible, |this| {
                this.child(Self::render_resize_handle(border))
                    .child(Self::render_review(
                        geometry,
                        review_background,
                        review_view,
                        &self.review_focus_handle,
                    ))
            })
    }
}
