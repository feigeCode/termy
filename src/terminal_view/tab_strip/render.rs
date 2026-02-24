use super::super::tab_chrome;
use super::super::tabs::{TabDropMarkerSide, TabStripOverflowState};
use super::super::*;
use super::layout::TabStripGeometry;
use gpui::{Hsla, TextRun};

#[derive(Clone, Copy)]
struct TabStripPalette {
    tab_stroke_color: gpui::Rgba,
    inactive_tab_bg: gpui::Rgba,
    active_tab_bg: gpui::Rgba,
    hovered_tab_bg: gpui::Rgba,
    active_tab_text: gpui::Rgba,
    inactive_tab_text: gpui::Rgba,
    close_button_hover_bg: gpui::Rgba,
    close_button_hover_text: gpui::Rgba,
    tab_drop_marker_color: gpui::Rgba,
    tabbar_new_tab_bg: gpui::Rgba,
    tabbar_new_tab_hover_bg: gpui::Rgba,
    tabbar_new_tab_border: gpui::Rgba,
    tabbar_new_tab_hover_border: gpui::Rgba,
    tabbar_new_tab_text: gpui::Rgba,
    tabbar_new_tab_hover_text: gpui::Rgba,
    tab_overflow_fade_soft: gpui::Rgba,
    tab_overflow_fade_mid: gpui::Rgba,
    tab_overflow_fade_strong: gpui::Rgba,
}

struct TabStripRenderState {
    geometry: TabStripGeometry,
    content_width: f32,
    overflow_state: TabStripOverflowState,
    chrome_layout: tab_chrome::TabChromeLayout,
}

struct TabItemRenderInput {
    index: usize,
    tab_width: f32,
    tab_strokes: tab_chrome::TabStrokeRects,
    label: String,
    is_active: bool,
    is_hovered: bool,
    is_renaming: bool,
    show_tab_close: bool,
    close_slot_width: f32,
    drop_marker_side: Option<TabDropMarkerSide>,
}

impl TerminalView {
    fn measure_tab_title_width(
        &mut self,
        window: &Window,
        font_family: &SharedString,
        font_family_key: &str,
        title: &str,
    ) -> f32 {
        if title.is_empty() {
            return 0.0;
        }

        let font_size_px = 12.0f32;
        let font_size_bits = font_size_px.to_bits();
        if let Some(width) =
            self.tab_strip
                .title_width_cache
                .get(title, font_family_key, font_size_bits)
        {
            return width;
        }

        let run = TextRun {
            len: title.len(),
            font: Font {
                family: font_family.clone(),
                weight: FontWeight::NORMAL,
                ..Default::default()
            },
            color: Hsla {
                h: 0.0,
                s: 0.0,
                l: 1.0,
                a: 1.0,
            },
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let shaped = window
            .text_system()
            .shape_line(title.to_string().into(), px(font_size_px), &[run], None);
        let width: f32 = shaped.x_for_index(title.len()).into();
        let width = width.max(0.0);
        self.tab_strip
            .title_width_cache
            .insert(title, font_family_key, font_size_bits, width);
        width
    }

    fn measure_tab_title_widths(
        &mut self,
        window: &Window,
        font_family: &SharedString,
        font_family_key: &str,
    ) -> Vec<f32> {
        let mut widths = Vec::with_capacity(self.tabs.len());
        for index in 0..self.tabs.len() {
            let title = self.tabs[index].title.clone();
            widths.push(self.measure_tab_title_width(window, font_family, font_family_key, &title));
        }
        widths
    }

    fn resolve_tab_strip_palette(&self, colors: &TerminalColors, tabbar_bg: gpui::Rgba) -> TabStripPalette {
        let tab_stroke_color = tab_chrome::resolve_tab_stroke_color(
            tabbar_bg,
            colors.foreground,
            TAB_STROKE_FOREGROUND_MIX,
        );
        let mut inactive_tab_bg = colors.foreground;
        inactive_tab_bg.a = self.scaled_chrome_alpha(0.10);
        let mut active_tab_bg = tabbar_bg;
        active_tab_bg.a = 0.0;
        let mut hovered_tab_bg = colors.foreground;
        hovered_tab_bg.a = self.scaled_chrome_alpha(0.13);
        let mut active_tab_text = colors.foreground;
        active_tab_text.a = 0.95;
        let mut inactive_tab_text = colors.foreground;
        inactive_tab_text.a = 0.7;
        let mut close_button_hover_bg = colors.foreground;
        close_button_hover_bg.a = self.scaled_chrome_alpha(0.24);
        let mut close_button_hover_text = colors.foreground;
        close_button_hover_text.a = 0.98;
        let mut tab_drop_marker_color = colors.cursor;
        tab_drop_marker_color.a = self.scaled_chrome_alpha(0.95);
        let mut tabbar_new_tab_bg = colors.foreground;
        tabbar_new_tab_bg.a = self.scaled_chrome_alpha(0.11);
        let mut tabbar_new_tab_hover_bg = colors.foreground;
        tabbar_new_tab_hover_bg.a = self.scaled_chrome_alpha(0.2);
        let mut tabbar_new_tab_border = colors.foreground;
        tabbar_new_tab_border.a = self.scaled_chrome_alpha(0.24);
        let mut tabbar_new_tab_hover_border = colors.cursor;
        tabbar_new_tab_hover_border.a = self.scaled_chrome_alpha(0.76);
        let mut tabbar_new_tab_text = colors.foreground;
        tabbar_new_tab_text.a = 0.9;
        let mut tabbar_new_tab_hover_text = colors.cursor;
        tabbar_new_tab_hover_text.a = 0.98;
        let mut tab_overflow_fade_soft = tabbar_bg;
        tab_overflow_fade_soft.a = self.scaled_chrome_alpha(0.18);
        let mut tab_overflow_fade_mid = tabbar_bg;
        tab_overflow_fade_mid.a = self.scaled_chrome_alpha(0.28);
        let mut tab_overflow_fade_strong = tabbar_bg;
        tab_overflow_fade_strong.a = self.scaled_chrome_alpha(0.38);

        TabStripPalette {
            tab_stroke_color,
            inactive_tab_bg,
            active_tab_bg,
            hovered_tab_bg,
            active_tab_text,
            inactive_tab_text,
            close_button_hover_bg,
            close_button_hover_text,
            tab_drop_marker_color,
            tabbar_new_tab_bg,
            tabbar_new_tab_hover_bg,
            tabbar_new_tab_border,
            tabbar_new_tab_hover_border,
            tabbar_new_tab_text,
            tabbar_new_tab_hover_text,
            tab_overflow_fade_soft,
            tab_overflow_fade_mid,
            tab_overflow_fade_strong,
        }
    }

    fn build_tab_strip_render_state(&mut self, window: &Window) -> TabStripRenderState {
        let layout = self.tab_strip_layout(window);
        self.set_tab_strip_layout_snapshot(layout);

        let geometry = layout.geometry;
        let tab_strip_viewport_width = geometry.tabs_viewport_width;
        self.sync_tab_display_widths_for_viewport_if_needed(tab_strip_viewport_width);
        let content_width = self
            .tab_strip_fixed_content_width()
            .max(tab_strip_viewport_width);
        let overflow_state = self.tab_strip_overflow_state();
        let active_tab_index = (self.active_tab < self.tabs.len()).then_some(self.active_tab);
        let chrome_layout = tab_chrome::compute_tab_chrome_layout(
            self.tabs.iter().map(|tab| tab.display_width),
            tab_chrome::TabChromeInput {
                active_index: active_tab_index,
                tabbar_height: TABBAR_HEIGHT,
                tab_item_height: TAB_ITEM_HEIGHT,
                horizontal_padding: TAB_HORIZONTAL_PADDING,
                tab_item_gap: TAB_ITEM_GAP,
            },
        );
        debug_assert!(chrome_layout.tab_strokes.len() == self.tabs.len());

        TabStripRenderState {
            geometry,
            content_width,
            overflow_state,
            chrome_layout,
        }
    }

    fn render_tab_stroke(stroke: tab_chrome::StrokeRect, color: gpui::Rgba) -> AnyElement {
        div()
            .absolute()
            .left(px(stroke.x))
            .top(px(stroke.y))
            .w(px(stroke.w))
            .h(px(stroke.h))
            .bg(color)
            .into_any_element()
    }

    fn render_inset_lane(
        id: &'static str,
        width: f32,
        tab_baseline_y: f32,
        tab_stroke_color: gpui::Rgba,
    ) -> AnyElement {
        div()
            .id(id)
            .relative()
            .flex_none()
            .w(px(width))
            .h_full()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(tab_baseline_y))
                    .h(px(TAB_STROKE_THICKNESS))
                    .bg(tab_stroke_color),
            )
            .into_any_element()
    }

    fn render_gutter_lane(
        gutter_width: f32,
        tab_baseline_y: f32,
        tab_stroke_color: gpui::Rgba,
    ) -> AnyElement {
        div()
            .id("tabbar-action-gutter")
            .relative()
            .flex_none()
            .w(px(gutter_width))
            .h_full()
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(tab_baseline_y))
                    .h(px(TAB_STROKE_THICKNESS))
                    .bg(tab_stroke_color),
            )
            .child(
                div()
                    .absolute()
                    .left(px(((gutter_width - TAB_STROKE_THICKNESS) * 0.5).max(0.0)))
                    .top(px(TAB_STROKE_THICKNESS))
                    .bottom_0()
                    .w(px(TAB_STROKE_THICKNESS))
                    .bg(tab_stroke_color),
            )
            .into_any_element()
    }

    fn render_baseline_segments(
        layout: &tab_chrome::TabChromeLayout,
        tab_stroke_color: gpui::Rgba,
    ) -> Vec<AnyElement> {
        let mut elements = Vec::with_capacity(layout.baseline_strokes.len() + 1);
        for segment in &layout.baseline_strokes {
            elements.push(Self::render_tab_stroke(*segment, tab_stroke_color));
        }
        elements.push(
            div()
                .id("tabs-baseline-tail-filler")
                .flex_1()
                .min_w(px(0.0))
                .h(px(TABBAR_HEIGHT))
                .relative()
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .right_0()
                        .top(px(layout.baseline_y))
                        .h(px(TAB_STROKE_THICKNESS))
                        .bg(tab_stroke_color),
                )
                .into_any_element(),
        );
        elements
    }

    fn render_tab_item(
        &mut self,
        input: TabItemRenderInput,
        font_family: &SharedString,
        colors: &TerminalColors,
        palette: &TabStripPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let switch_tab_index = input.index;
        let hover_tab_index = input.index;
        let close_tab_index = input.index;

        let rename_text_color = if input.is_active {
            palette.active_tab_text
        } else {
            palette.inactive_tab_text
        };
        let mut rename_selection_color = colors.cursor;
        rename_selection_color.a = if input.is_active { 0.34 } else { 0.24 };

        let tab_bg = if input.is_active {
            palette.active_tab_bg
        } else if input.is_hovered {
            palette.hovered_tab_bg
        } else {
            palette.inactive_tab_bg
        };

        let mut close_text_color = if input.is_active {
            palette.active_tab_text
        } else {
            palette.inactive_tab_text
        };
        if !input.show_tab_close {
            close_text_color.a = 0.0;
        }

        let close_button = div()
            .w(px(input.close_slot_width))
            .h(px(TAB_CLOSE_HITBOX))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(5.0))
            .text_color(close_text_color)
            .text_size(px(12.0))
            .child("×")
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _event: &MouseDownEvent, _window, cx| {
                    let is_active = close_tab_index == this.active_tab;
                    if Self::tab_shows_close(
                        this.tab_close_visibility,
                        is_active,
                        this.tab_strip.hovered_tab,
                        this.tab_strip.hovered_tab_close,
                        close_tab_index,
                    ) {
                        this.close_tab(close_tab_index, cx);
                        cx.stop_propagation();
                    }
                }),
            )
            .on_mouse_move(
                cx.listener(move |this, _event: &MouseMoveEvent, _window, cx| {
                    this.on_tab_close_mouse_move(hover_tab_index, cx);
                    cx.stop_propagation();
                }),
            )
            .hover(move |style| {
                style
                    .bg(palette.close_button_hover_bg)
                    .text_color(palette.close_button_hover_text)
            })
            .cursor_pointer();

        let tab_shell = div()
            .flex_none()
            .relative()
            .bg(tab_bg)
            .w(px(input.tab_width))
            .h(px(TAB_ITEM_HEIGHT))
            .px(px(TAB_TEXT_PADDING_X))
            .flex()
            .items_center()
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
                    this.on_tab_mouse_down(switch_tab_index, event.click_count, cx);
                    cx.stop_propagation();
                }),
            )
            .on_mouse_move(
                cx.listener(move |this, event: &MouseMoveEvent, window, cx| {
                    this.on_tab_mouse_move(hover_tab_index, event, window, cx);
                    cx.stop_propagation();
                }),
            )
            .child(Self::render_tab_stroke(
                input.tab_strokes.top,
                palette.tab_stroke_color,
            ))
            .children(
                input
                    .tab_strokes
                    .left_boundary
                    .map(|stroke| Self::render_tab_stroke(stroke, palette.tab_stroke_color)),
            )
            .children(
                input
                    .tab_strokes
                    .right_boundary
                    .map(|stroke| Self::render_tab_stroke(stroke, palette.tab_stroke_color)),
            );

        let drop_marker = input.drop_marker_side.map(|side| {
            let marker_x = match side {
                TabDropMarkerSide::Left => 0.0,
                TabDropMarkerSide::Right => input.tab_width - TAB_DROP_MARKER_WIDTH,
            }
            .max(0.0);
            let marker_height = (TAB_ITEM_HEIGHT - (TAB_DROP_MARKER_INSET_Y * 2.0)).max(0.0);

            div()
                .absolute()
                .left(px(marker_x))
                .top(px(TAB_DROP_MARKER_INSET_Y))
                .w(px(TAB_DROP_MARKER_WIDTH))
                .h(px(marker_height))
                .bg(palette.tab_drop_marker_color)
        });

        tab_shell
            .child(div().flex_1().min_w(px(0.0)).h_full().relative().child(
                if input.is_renaming {
                    self.render_inline_input_layer(
                        Font {
                            family: font_family.clone(),
                            weight: FontWeight::NORMAL,
                            ..Default::default()
                        },
                        px(12.0),
                        rename_text_color.into(),
                        rename_selection_color.into(),
                        InlineInputAlignment::Left,
                        cx,
                    )
                } else {
                    let title_text = div()
                        .size_full()
                        .flex()
                        .items_center()
                        .overflow_x_hidden()
                        .whitespace_nowrap()
                        .font_family(font_family.clone())
                        .text_color(rename_text_color)
                        .text_size(px(12.0))
                        .text_ellipsis();
                    title_text.child(input.label).into_any_element()
                },
            ))
            .child(close_button)
            .children(drop_marker)
            .into_any_element()
    }

    fn build_tabs_scroll_content(
        &mut self,
        window: &Window,
        state: &TabStripRenderState,
        palette: &TabStripPalette,
        font_family: &SharedString,
        font_family_key: &str,
        colors: &TerminalColors,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut tabs_scroll_content = div()
            .id("tabs-scroll-content")
            .flex_none()
            .w(px(state.content_width))
            .min_w(px(state.content_width))
            .h(px(TABBAR_HEIGHT))
            .flex()
            .relative()
            .items_end()
            .gap(px(TAB_ITEM_GAP))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                this.on_tabs_content_mouse_move(event, window, cx);
            }));

        tabs_scroll_content = tabs_scroll_content.child(
            div()
                .id("tabs-left-padding-spacer")
                .flex_none()
                .w(px(TAB_HORIZONTAL_PADDING))
                .h(px(TABBAR_HEIGHT)),
        );

        for index in 0..self.tabs.len() {
            let (tab_width, tab_title) = {
                let tab = &self.tabs[index];
                (tab.display_width, tab.title.clone())
            };
            let is_active = index == self.active_tab;
            let is_hovered = self.tab_strip.hovered_tab == Some(index);
            let show_tab_close = Self::tab_shows_close(
                self.tab_close_visibility,
                is_active,
                self.tab_strip.hovered_tab,
                self.tab_strip.hovered_tab_close,
                index,
            );
            let is_renaming = self.renaming_tab == Some(index);
            let close_slot_width = if show_tab_close {
                TAB_CLOSE_SLOT_WIDTH
            } else {
                0.0
            };
            let available_text_px = Self::tab_title_text_area_width(tab_width, close_slot_width);
            let label = Self::format_tab_label_for_render_measured(
                &tab_title,
                available_text_px,
                |candidate| {
                    self.measure_tab_title_width(window, font_family, font_family_key, candidate)
                },
            );

            let tab_item = self.render_tab_item(
                TabItemRenderInput {
                    index,
                    tab_width,
                    tab_strokes: state.chrome_layout.tab_strokes[index],
                    label,
                    is_active,
                    is_hovered,
                    is_renaming,
                    show_tab_close,
                    close_slot_width,
                    drop_marker_side: self.tab_drop_marker_side(index),
                },
                font_family,
                colors,
                palette,
                cx,
            );

            tabs_scroll_content = tabs_scroll_content.child(tab_item);
        }

        tabs_scroll_content = tabs_scroll_content.child(
            div()
                .id("tabs-right-padding-spacer")
                .flex_none()
                .w(px(TAB_HORIZONTAL_PADDING))
                .h(px(TABBAR_HEIGHT)),
        );

        for element in Self::render_baseline_segments(&state.chrome_layout, palette.tab_stroke_color) {
            tabs_scroll_content = tabs_scroll_content.child(element);
        }

        tabs_scroll_content.into_any_element()
    }

    fn render_tabbar_new_tab_button(
        &self,
        bg: gpui::Rgba,
        hover_bg: gpui::Rgba,
        border: gpui::Rgba,
        hover_border: gpui::Rgba,
        text: gpui::Rgba,
        hover_text: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .id("tabbar-new-tab")
            .w(px(TABBAR_NEW_TAB_BUTTON_SIZE))
            .h(px(TABBAR_NEW_TAB_BUTTON_SIZE))
            .rounded(px(TABBAR_NEW_TAB_BUTTON_RADIUS))
            .bg(bg)
            .border_1()
            .border_color(border)
            .text_color(text)
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _event: &MouseDownEvent, _window, cx| {
                    this.disarm_titlebar_window_move();
                    this.add_tab(cx);
                    cx.stop_propagation();
                }),
            )
            .hover(move |style| {
                style
                    .bg(hover_bg)
                    .border_color(hover_border)
                    .text_color(hover_text)
            })
            .child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(TABBAR_NEW_TAB_ICON_SIZE))
                    .font_weight(FontWeight::MEDIUM)
                    .mt(px(TABBAR_NEW_TAB_ICON_BASELINE_NUDGE_Y))
                    .child("+"),
            )
            .into_any_element()
    }

    fn render_action_rail(
        &mut self,
        state: &TabStripRenderState,
        palette: &TabStripPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let tabbar_new_tab_left =
            (state.geometry.button_start_x - state.geometry.action_rail_start_x).max(0.0);
        let tabbar_new_tab_top =
            (state.geometry.button_start_y - TOP_STRIP_CONTENT_OFFSET_Y).max(0.0);
        let tab_baseline_y = state.chrome_layout.baseline_y;

        div()
            .id("tabbar-action-rail")
            .relative()
            .flex_none()
            .w(px(state.geometry.action_rail_width))
            .h_full()
            .on_scroll_wheel(cx.listener(Self::handle_tab_strip_action_rail_scroll_wheel))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, window, cx| {
                this.on_action_rail_mouse_move(event, window, cx);
            }))
            .child(
                div()
                    .absolute()
                    .left(px(tabbar_new_tab_left))
                    .top(px(tabbar_new_tab_top))
                    .child(self.render_tabbar_new_tab_button(
                        palette.tabbar_new_tab_bg,
                        palette.tabbar_new_tab_hover_bg,
                        palette.tabbar_new_tab_border,
                        palette.tabbar_new_tab_hover_border,
                        palette.tabbar_new_tab_text,
                        palette.tabbar_new_tab_hover_text,
                        cx,
                    )),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(px(tab_baseline_y))
                    .h(px(TAB_STROKE_THICKNESS))
                    .bg(palette.tab_stroke_color),
            )
            .into_any_element()
    }

    pub(crate) fn render_tab_strip(
        &mut self,
        window: &Window,
        colors: &TerminalColors,
        font_family: &SharedString,
        tabbar_bg: gpui::Rgba,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let font_family_key = font_family.to_string();
        let measured_title_widths =
            self.measure_tab_title_widths(window, font_family, font_family_key.as_str());
        self.sync_tab_title_text_widths(&measured_title_widths);

        let state = self.build_tab_strip_render_state(window);
        let palette = self.resolve_tab_strip_palette(colors, tabbar_bg);
        let tabs_scroll_content = self.build_tabs_scroll_content(
            window,
            &state,
            &palette,
            font_family,
            font_family_key.as_str(),
            colors,
            cx,
        );

        let left_overflow_indicator = state.overflow_state.left.then(|| {
            Self::render_overflow_indicator(
                "tabs-overflow-left",
                true,
                palette.tab_overflow_fade_soft,
                palette.tab_overflow_fade_mid,
                palette.tab_overflow_fade_strong,
            )
        });
        let right_overflow_indicator = state.overflow_state.right.then(|| {
            Self::render_overflow_indicator(
                "tabs-overflow-right",
                false,
                palette.tab_overflow_fade_soft,
                palette.tab_overflow_fade_mid,
                palette.tab_overflow_fade_strong,
            )
        });

        div()
            .w_full()
            .h(px(TABBAR_HEIGHT))
            .flex()
            .children((state.geometry.left_inset_width > 0.0).then(|| {
                Self::render_inset_lane(
                    "tabbar-left-inset",
                    state.geometry.left_inset_width,
                    state.chrome_layout.baseline_y,
                    palette.tab_stroke_color,
                )
            }))
            .child(
                div()
                    .id("tabs-scroll-viewport-lane")
                    .flex_none()
                    .w(px(state.geometry.tabs_viewport_width))
                    .min_w(px(0.0))
                    .h_full()
                    .relative()
                    .child(
                        div()
                            .id("tabs-scroll-viewport")
                            .absolute()
                            .left_0()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .overflow_x_scroll()
                            .track_scroll(&self.tab_strip.scroll_handle)
                            .child(tabs_scroll_content),
                    )
                    .children(left_overflow_indicator)
                    .children(right_overflow_indicator),
            )
            .children((state.geometry.gutter_width > 0.0).then(|| {
                Self::render_gutter_lane(
                    state.geometry.gutter_width,
                    state.chrome_layout.baseline_y,
                    palette.tab_stroke_color,
                )
            }))
            .children((state.geometry.action_rail_width > 0.0).then(|| {
                self.render_action_rail(&state, &palette, cx)
            }))
            .children((state.geometry.right_inset_width > 0.0).then(|| {
                Self::render_inset_lane(
                    "tabbar-right-inset",
                    state.geometry.right_inset_width,
                    state.chrome_layout.baseline_y,
                    palette.tab_stroke_color,
                )
            }))
            .into_any_element()
    }

    fn render_overflow_indicator(
        id: &'static str,
        left_side: bool,
        soft: gpui::Rgba,
        mid: gpui::Rgba,
        strong: gpui::Rgba,
    ) -> AnyElement {
        let lane = if left_side {
            div()
                .id(id)
                .absolute()
                .top_0()
                .left_0()
                .bottom_0()
                .w(px(9.0))
                .child(
                    div()
                        .absolute()
                        .left_0()
                        .top_0()
                        .bottom_0()
                        .w(px(3.0))
                        .bg(strong),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(3.0))
                        .top_0()
                        .bottom_0()
                        .w(px(3.0))
                        .bg(mid),
                )
                .child(
                    div()
                        .absolute()
                        .left(px(6.0))
                        .top_0()
                        .bottom_0()
                        .w(px(3.0))
                        .bg(soft),
                )
        } else {
            div()
                .id(id)
                .absolute()
                .top_0()
                .right_0()
                .bottom_0()
                .w(px(9.0))
                .child(
                    div()
                        .absolute()
                        .right_0()
                        .top_0()
                        .bottom_0()
                        .w(px(3.0))
                        .bg(strong),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(3.0))
                        .top_0()
                        .bottom_0()
                        .w(px(3.0))
                        .bg(mid),
                )
                .child(
                    div()
                        .absolute()
                        .right(px(6.0))
                        .top_0()
                        .bottom_0()
                        .w(px(3.0))
                        .bg(soft),
                )
        };

        lane.into_any_element()
    }
}
