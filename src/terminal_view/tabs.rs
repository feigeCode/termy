use super::*;
use super::tab_strip::state::TabDragState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TabDropMarkerSide {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct TabStripOverflowState {
    pub(super) left: bool,
    pub(super) right: bool,
}

impl TerminalView {
    fn clear_tab_hover_fields(
        hovered_tab: &mut Option<usize>,
        hovered_tab_close: &mut Option<usize>,
    ) -> bool {
        let hovered_tab_changed = hovered_tab.take().is_some();
        let hovered_close_changed = hovered_tab_close.take().is_some();
        hovered_tab_changed || hovered_close_changed
    }

    fn reset_tab_rename_fields(
        renaming_tab: &mut Option<usize>,
        rename_input: &mut InlineInputState,
        inline_input_selecting: &mut bool,
    ) -> bool {
        let was_renaming = renaming_tab.take().is_some();
        let had_text = !rename_input.text().is_empty();
        let was_selecting = *inline_input_selecting;
        rename_input.clear();
        *inline_input_selecting = false;
        was_renaming || had_text || was_selecting
    }

    fn allocate_tab_id(&mut self) -> TabId {
        let id = self.next_tab_id;
        self.next_tab_id = self.next_tab_id.checked_add(1).expect("tab id overflow");
        id
    }

    fn tab_index_for_id_in_order(
        tab_ids: impl IntoIterator<Item = TabId>,
        tab_id: TabId,
    ) -> Option<usize> {
        tab_ids
            .into_iter()
            .enumerate()
            .find_map(|(index, candidate)| (candidate == tab_id).then_some(index))
    }

    pub(super) fn index_for_tab_id(&self, tab_id: TabId) -> Option<usize> {
        Self::tab_index_for_id_in_order(self.tabs.iter().map(|tab| tab.id), tab_id)
    }

    pub(super) fn clear_tab_title_width_cache(&mut self) {
        self.tab_strip.title_width_cache.clear();
    }

    pub(super) fn invalidate_tab_title_width_cache_for_title(&mut self, title: &str) {
        self.tab_strip.title_width_cache.invalidate_title(title);
    }

    pub(super) fn clear_tab_hover_state(&mut self) -> bool {
        Self::clear_tab_hover_fields(
            &mut self.tab_strip.hovered_tab,
            &mut self.tab_strip.hovered_tab_close,
        )
    }

    pub(super) fn reset_tab_rename_state(&mut self) -> bool {
        Self::reset_tab_rename_fields(
            &mut self.renaming_tab,
            &mut self.rename_input,
            &mut self.inline_input_selecting,
        )
    }

    pub(super) fn reset_tab_drag_state(&mut self) -> bool {
        self.finish_tab_drag()
    }

    pub(super) fn reset_tab_interaction_state(&mut self) -> bool {
        let rename_changed = self.reset_tab_rename_state();
        let hover_changed = self.clear_tab_hover_state();
        let drag_changed = self.reset_tab_drag_state();
        self.clear_selection();
        rename_changed || hover_changed || drag_changed
    }

    pub(super) fn tab_strip_fixed_content_width(&self) -> f32 {
        let tabs_width: f32 = self.tabs.iter().map(|tab| tab.display_width).sum();
        let gaps = TAB_ITEM_GAP * self.tabs.len().saturating_sub(1) as f32;
        TAB_HORIZONTAL_PADDING + tabs_width + gaps + TAB_HORIZONTAL_PADDING
    }

    pub(super) fn tab_strip_expected_max_scroll_for_viewport(&self, viewport_width: f32) -> f32 {
        (self.tab_strip_fixed_content_width() - viewport_width.max(0.0)).max(0.0)
    }

    pub(super) fn tab_strip_scroll_max_x(&self) -> f32 {
        self.tab_strip_expected_max_scroll_for_viewport(self.tab_strip.layout_last_synced_viewport_width)
    }

    pub(super) fn scroll_active_tab_into_view(&self) {
        if self.active_tab >= self.tabs.len() {
            return;
        }

        let viewport_width = self.tab_strip.layout_last_synced_viewport_width.max(0.0);
        if viewport_width <= f32::EPSILON {
            return;
        }

        let max_scroll = self.tab_strip_scroll_max_x();
        let mut tab_left = TAB_HORIZONTAL_PADDING;
        for (index, tab) in self.tabs.iter().enumerate() {
            let tab_right = tab_left + tab.display_width;
            if index == self.active_tab {
                let offset = self.tab_strip.scroll_handle.offset();
                let current_scroll = -Into::<f32>::into(offset.x);
                let mut target_scroll = current_scroll;
                if tab_left < current_scroll {
                    target_scroll = tab_left;
                } else if tab_right > current_scroll + viewport_width {
                    target_scroll = tab_right - viewport_width;
                }

                let clamped_scroll = target_scroll.clamp(0.0, max_scroll);
                let next_offset_x = -clamped_scroll;
                let current_offset_x: f32 = offset.x.into();
                if (next_offset_x - current_offset_x).abs() > f32::EPSILON {
                    self.tab_strip
                        .scroll_handle
                        .set_offset(point(px(next_offset_x), offset.y));
                }
                return;
            }
            tab_left = tab_right + TAB_ITEM_GAP;
        }
    }

    fn clear_tab_drag_preview_state(&mut self) {
        self.tab_strip.drag_pointer_x = None;
        self.tab_strip.drag_viewport_width = 0.0;
        self.tab_strip.drag_autoscroll_animating = false;
    }

    fn ensure_tab_drag_autoscroll_animation(&mut self, cx: &mut Context<Self>) {
        if self.tab_strip.drag_autoscroll_animating {
            return;
        }
        self.tab_strip.drag_autoscroll_animating = true;

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                smol::Timer::after(Duration::from_millis(16)).await;
                let keep_animating = match cx.update(|cx| {
                    this.update(cx, |view, cx| {
                        if !view.tab_strip.drag_autoscroll_animating || view.tab_strip.drag.is_none() {
                            view.tab_strip.drag_autoscroll_animating = false;
                            return false;
                        }

                        let Some(pointer_x) = view.tab_strip.drag_pointer_x else {
                            view.tab_strip.drag_autoscroll_animating = false;
                            return false;
                        };
                        let viewport_width = view.tab_strip.drag_viewport_width;
                        let scrolled =
                            view.auto_scroll_tab_strip_during_drag(pointer_x, viewport_width);
                        let marker_changed = view.update_tab_drag_marker(pointer_x, cx);
                        if scrolled && !marker_changed {
                            cx.notify();
                        }
                        if !scrolled {
                            view.tab_strip.drag_autoscroll_animating = false;
                            return false;
                        }
                        true
                    })
                }) {
                    Ok(keep_animating) => keep_animating,
                    _ => break,
                };

                if !keep_animating {
                    break;
                }
            }
        })
        .detach();
    }

    fn tab_strip_overflow_state_for_scroll(
        scroll_x: f32,
        max_scroll_x: f32,
    ) -> TabStripOverflowState {
        let max_scroll = max_scroll_x.max(0.0);
        if max_scroll <= TAB_STRIP_SCROLL_EPSILON {
            return TabStripOverflowState::default();
        }

        let clamped_scroll = scroll_x.clamp(0.0, max_scroll);
        TabStripOverflowState {
            left: clamped_scroll > TAB_STRIP_SCROLL_EPSILON,
            right: (max_scroll - clamped_scroll) > TAB_STRIP_SCROLL_EPSILON,
        }
    }

    pub(super) fn tab_strip_overflow_state(&self) -> TabStripOverflowState {
        let offset = self.tab_strip.scroll_handle.offset();
        let scroll_x = -Into::<f32>::into(offset.x);
        let max_scroll = self.tab_strip_scroll_max_x();
        Self::tab_strip_overflow_state_for_scroll(scroll_x, max_scroll)
    }

    fn tab_strip_offset_x_for_delta(
        current_offset_x: f32,
        delta_x: f32,
        max_scroll: f32,
    ) -> Option<f32> {
        if delta_x.abs() <= f32::EPSILON {
            return None;
        }

        let bounded_max = max_scroll.max(0.0);
        if bounded_max <= TAB_STRIP_SCROLL_EPSILON {
            return None;
        }

        let clamped_current = current_offset_x.clamp(-bounded_max, 0.0);
        let next_offset = (clamped_current + delta_x).clamp(-bounded_max, 0.0);
        ((next_offset - clamped_current).abs() > f32::EPSILON).then_some(next_offset)
    }

    pub(super) fn scroll_tab_strip_by(&mut self, delta_x: f32) -> bool {
        let max_scroll = self.tab_strip_scroll_max_x();
        let offset = self.tab_strip.scroll_handle.offset();
        let current_offset_x: f32 = offset.x.into();
        let Some(next_offset_x) =
            Self::tab_strip_offset_x_for_delta(current_offset_x, delta_x, max_scroll)
        else {
            return false;
        };

        self.tab_strip.scroll_handle
            .set_offset(point(px(next_offset_x), offset.y));
        true
    }

    pub(super) fn effective_tab_max_width_for_viewport(
        viewport_width: f32,
        tab_count: usize,
    ) -> f32 {
        let content_width = (viewport_width - (TAB_HORIZONTAL_PADDING * 2.0)).max(TAB_MAX_WIDTH);
        let share = content_width / tab_count.max(1) as f32;
        let elastic_growth = (share - TAB_MAX_WIDTH).max(0.0) * TAB_ADAPTIVE_GROWTH_FACTOR;
        let elastic = TAB_MAX_WIDTH + elastic_growth;
        let hard_cap = (content_width * TAB_ADAPTIVE_HARD_CAP_RATIO).max(TAB_MAX_WIDTH);

        elastic.min(hard_cap)
    }

    fn tab_display_width_for_text_px_with_close_policy(
        text_width_px: f32,
        max_width: f32,
        reserve_close_slot: bool,
    ) -> f32 {
        let text_width = if text_width_px.is_finite() {
            text_width_px.max(0.0)
        } else {
            0.0
        };
        let close_slot_width = if reserve_close_slot {
            TAB_CLOSE_SLOT_WIDTH
        } else {
            0.0
        };
        let base_width = (TAB_TEXT_PADDING_X * 2.0) + text_width + close_slot_width;
        let slack_start = TAB_MIN_WIDTH - TAB_TITLE_LAYOUT_SLACK_PX;
        let slack_end = TAB_MIN_WIDTH + TAB_TITLE_LAYOUT_SLACK_PX;
        let slack_span = (slack_end - slack_start).max(f32::EPSILON);
        let slack_factor = ((slack_end - base_width) / slack_span).clamp(0.0, 1.0);
        let effective_slack = TAB_TITLE_LAYOUT_SLACK_PX * slack_factor;
        let width = base_width + effective_slack;
        width.clamp(TAB_MIN_WIDTH, max_width.max(TAB_MIN_WIDTH))
    }

    pub(super) fn tab_display_width_for_text_px_with_max(text_width_px: f32, max_width: f32) -> f32 {
        Self::tab_display_width_for_text_px_with_close_policy(text_width_px, max_width, true)
    }

    pub(super) fn tab_display_width_for_text_px_without_close_with_max(
        text_width_px: f32,
        max_width: f32,
    ) -> f32 {
        Self::tab_display_width_for_text_px_with_close_policy(text_width_px, max_width, false)
    }

    pub(super) fn tab_title_text_area_width(tab_width: f32, close_slot_width: f32) -> f32 {
        (tab_width - (TAB_TEXT_PADDING_X * 2.0) - close_slot_width).max(0.0)
    }

    pub(super) fn sync_tab_title_text_widths(&mut self, measured_text_widths: &[f32]) -> bool {
        debug_assert_eq!(measured_text_widths.len(), self.tabs.len());
        let mut changed = false;

        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let Some(width) = measured_text_widths.get(index).copied() else {
                continue;
            };
            let width = if width.is_finite() { width.max(0.0) } else { 0.0 };
            if (tab.title_text_width - width).abs() <= 0.01 {
                continue;
            }
            tab.title_text_width = width;
            changed = true;
        }

        if changed {
            self.mark_tab_strip_layout_dirty();
        }

        changed
    }

    fn tab_reserves_close_slot_for_layout(
        tab_width_mode: TabWidthMode,
        tab_close_visibility: TabCloseVisibility,
        is_active: bool,
    ) -> bool {
        match tab_width_mode {
            TabWidthMode::Stable => true,
            TabWidthMode::ActiveGrow | TabWidthMode::ActiveGrowSticky => {
                matches!(tab_close_visibility, TabCloseVisibility::Always)
                    || (matches!(tab_close_visibility, TabCloseVisibility::ActiveHover) && is_active)
            }
        }
    }

    fn resolve_tab_width_for_mode(
        tab_width_mode: TabWidthMode,
        text_width_px: f32,
        effective_max: f32,
        reserve_close_slot: bool,
        sticky_title_width: f32,
    ) -> (f32, f32) {
        let capped_max = effective_max.max(TAB_MIN_WIDTH);
        let title_only_width =
            Self::tab_display_width_for_text_px_without_close_with_max(text_width_px, effective_max);
        let close_policy_width = Self::tab_display_width_for_text_px_with_close_policy(
            text_width_px,
            effective_max,
            reserve_close_slot,
        );

        match tab_width_mode {
            TabWidthMode::Stable | TabWidthMode::ActiveGrow => (close_policy_width, title_only_width),
            TabWidthMode::ActiveGrowSticky => {
                let next_sticky_width = sticky_title_width.max(title_only_width).min(capped_max);
                let next_width = if reserve_close_slot {
                    close_policy_width.max(next_sticky_width).min(capped_max)
                } else {
                    next_sticky_width
                };
                (next_width, next_sticky_width)
            }
        }
    }

    pub(super) fn sync_tab_display_widths_for_viewport(&mut self, viewport_width: f32) -> bool {
        let viewport_width = if viewport_width.is_finite() {
            viewport_width.max(0.0)
        } else {
            0.0
        };
        let effective_max =
            Self::effective_tab_max_width_for_viewport(viewport_width, self.tabs.len());
        let mut changed = false;
        let tab_width_mode = self.tab_width_mode;
        let tab_close_visibility = self.tab_close_visibility;

        for (index, tab) in self.tabs.iter_mut().enumerate() {
            let is_active = index == self.active_tab;
            let reserve_close_slot = Self::tab_reserves_close_slot_for_layout(
                tab_width_mode,
                tab_close_visibility,
                is_active,
            );
            let (next_width, next_sticky_width) = Self::resolve_tab_width_for_mode(
                tab_width_mode,
                tab.title_text_width,
                effective_max,
                reserve_close_slot,
                tab.sticky_title_width,
            );
            tab.sticky_title_width = next_sticky_width;
            if (tab.display_width - next_width).abs() <= f32::EPSILON {
                continue;
            }

            tab.display_width = next_width;
            changed = true;
        }

        changed
    }

    pub(super) fn mark_tab_strip_layout_dirty(&mut self) {
        self.tab_strip.layout_revision = self.tab_strip.layout_revision.wrapping_add(1);
    }

    pub(super) fn sync_tab_display_widths_for_viewport_if_needed(
        &mut self,
        viewport_width: f32,
    ) -> bool {
        let clamped_viewport = if viewport_width.is_finite() {
            viewport_width.max(0.0)
        } else {
            0.0
        };
        let viewport_unchanged =
            (self.tab_strip.layout_last_synced_viewport_width - clamped_viewport).abs() <= f32::EPSILON;
        let revision_unchanged = self.tab_strip.layout_last_synced_revision == self.tab_strip.layout_revision;
        if viewport_unchanged && revision_unchanged {
            return false;
        }

        let changed = self.sync_tab_display_widths_for_viewport(clamped_viewport);
        self.tab_strip.layout_last_synced_viewport_width = clamped_viewport;
        self.tab_strip.layout_last_synced_revision = self.tab_strip.layout_revision;
        changed
    }

    pub(super) fn tab_shows_close(
        close_visibility: TabCloseVisibility,
        is_active: bool,
        hovered_tab: Option<usize>,
        hovered_tab_close: Option<usize>,
        index: usize,
    ) -> bool {
        match close_visibility {
            TabCloseVisibility::ActiveHover => {
                is_active || hovered_tab == Some(index) || hovered_tab_close == Some(index)
            }
            TabCloseVisibility::Hover => {
                hovered_tab == Some(index) || hovered_tab_close == Some(index)
            }
            TabCloseVisibility::Always => true,
        }
    }

    fn remap_index_after_move(index: usize, from: usize, to: usize) -> usize {
        if index == from {
            return to;
        }

        if from < to {
            if (from + 1..=to).contains(&index) {
                return index - 1;
            }
            index
        } else if (to..from).contains(&index) {
            index + 1
        } else {
            index
        }
    }

    pub(super) fn begin_tab_drag(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.clear_tab_drag_preview_state();
            self.tab_strip.drag = Some(TabDragState {
                source_index: index,
                drop_slot: None,
            });
        }
    }

    pub(super) fn finish_tab_drag(&mut self) -> bool {
        let marker_was_visible = self
            .tab_strip
            .drag
            .as_ref()
            .and_then(|drag| drag.drop_slot)
            .is_some();
        self.tab_strip.drag = None;
        self.clear_tab_drag_preview_state();
        marker_was_visible
    }

    fn tab_drop_slot_from_pointer_x_for_widths(
        tab_widths: impl IntoIterator<Item = f32>,
        pointer_x: f32,
        scroll_offset_x: f32,
    ) -> usize {
        let mut left = TAB_HORIZONTAL_PADDING + scroll_offset_x;
        let mut slot = 0;

        for width in tab_widths {
            let midpoint_x = left + (width * 0.5);
            if pointer_x < midpoint_x {
                return slot;
            }

            left += width + TAB_ITEM_GAP;
            slot += 1;
        }

        slot
    }

    fn tab_drop_slot_from_pointer_x(&self, pointer_x: f32) -> usize {
        let scroll_offset_x: f32 = self.tab_strip.scroll_handle.offset().x.into();
        Self::tab_drop_slot_from_pointer_x_for_widths(
            self.tabs.iter().map(|tab| tab.display_width),
            pointer_x,
            scroll_offset_x,
        )
    }

    fn normalized_drop_slot(source_index: usize, raw_slot: usize) -> Option<usize> {
        if raw_slot == source_index || raw_slot == source_index.saturating_add(1) {
            return None;
        }
        Some(raw_slot)
    }

    fn reorder_target_index_for_drop_slot(source_index: usize, drop_slot: usize) -> usize {
        if drop_slot > source_index {
            drop_slot - 1
        } else {
            drop_slot
        }
    }

    fn tab_drop_marker_side_for_slot(index: usize, drop_slot: usize) -> Option<TabDropMarkerSide> {
        if drop_slot == index {
            Some(TabDropMarkerSide::Left)
        } else if drop_slot == index.saturating_add(1) {
            Some(TabDropMarkerSide::Right)
        } else {
            None
        }
    }

    pub(super) fn tab_drop_marker_side(&self, index: usize) -> Option<TabDropMarkerSide> {
        if index >= self.tabs.len() {
            return None;
        }

        let drop_slot = self.tab_strip.drag.and_then(|drag| drag.drop_slot)?;
        Self::tab_drop_marker_side_for_slot(index, drop_slot)
    }

    fn update_tab_drag_marker(&mut self, pointer_x: f32, cx: &mut Context<Self>) -> bool {
        let Some(source_index) = self.tab_strip.drag.map(|drag| drag.source_index) else {
            return false;
        };

        let raw_drop_slot = self.tab_drop_slot_from_pointer_x(pointer_x);
        let next_drop_slot = Self::normalized_drop_slot(source_index, raw_drop_slot);

        let Some(drag) = self.tab_strip.drag.as_mut() else {
            return false;
        };
        if drag.drop_slot == next_drop_slot {
            return false;
        }

        drag.drop_slot = next_drop_slot;
        cx.notify();
        true
    }

    fn auto_scroll_tab_strip_during_drag(&mut self, pointer_x: f32, viewport_width: f32) -> bool {
        if self.tab_strip.drag.is_none() || viewport_width <= f32::EPSILON {
            return false;
        }

        let edge = TAB_DRAG_AUTOSCROLL_EDGE_WIDTH
            .min(viewport_width * 0.5)
            .max(f32::EPSILON);
        let left_strength = ((edge - pointer_x) / edge).clamp(0.0, 1.0);
        let right_start = (viewport_width - edge).max(0.0);
        let right_strength = ((pointer_x - right_start) / edge).clamp(0.0, 1.0);
        let delta = (right_strength - left_strength) * TAB_DRAG_AUTOSCROLL_MAX_STEP;
        self.scroll_tab_strip_by(-delta)
    }

    pub(super) fn update_tab_drag_preview(
        &mut self,
        pointer_x: f32,
        viewport_width: f32,
        cx: &mut Context<Self>,
    ) -> bool {
        if self.tab_strip.drag.is_none() {
            return false;
        }
        self.tab_strip.drag_pointer_x = Some(pointer_x);
        self.tab_strip.drag_viewport_width = viewport_width.max(0.0);
        let widths_changed =
            self.sync_tab_display_widths_for_viewport_if_needed(self.tab_strip.drag_viewport_width);

        let scrolled = self.auto_scroll_tab_strip_during_drag(pointer_x, viewport_width);
        let marker_changed = self.update_tab_drag_marker(pointer_x, cx);
        if scrolled && !marker_changed {
            cx.notify();
        }
        if widths_changed && !scrolled && !marker_changed {
            cx.notify();
        }
        if scrolled {
            self.ensure_tab_drag_autoscroll_animation(cx);
        } else {
            self.tab_strip.drag_autoscroll_animating = false;
        }
        scrolled || marker_changed || widths_changed
    }

    pub(super) fn commit_tab_drag(&mut self, cx: &mut Context<Self>) {
        let drag = self.tab_strip.drag.take();
        self.clear_tab_drag_preview_state();
        let Some(TabDragState {
            source_index,
            drop_slot,
        }) = drag
        else {
            return;
        };

        let Some(drop_slot) = drop_slot else {
            return;
        };

        let target_index = Self::reorder_target_index_for_drop_slot(source_index, drop_slot);
        if source_index == target_index {
            cx.notify();
            return;
        }

        if !self.reorder_tab(source_index, target_index, cx) {
            cx.notify();
        }
    }

    pub(super) fn reorder_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) -> bool {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return false;
        }

        let moved_tab = self.tabs.remove(from);
        self.tabs.insert(to, moved_tab);

        self.active_tab = Self::remap_index_after_move(self.active_tab, from, to);
        self.renaming_tab = self
            .renaming_tab
            .map(|index| Self::remap_index_after_move(index, from, to));
        self.tab_strip.hovered_tab = self
            .tab_strip
            .hovered_tab
            .map(|index| Self::remap_index_after_move(index, from, to));
        self.tab_strip.hovered_tab_close = self
            .tab_strip
            .hovered_tab_close
            .map(|index| Self::remap_index_after_move(index, from, to));

        self.scroll_active_tab_into_view();
        cx.notify();
        true
    }

    pub(super) fn add_tab(&mut self, cx: &mut Context<Self>) {
        let terminal = Terminal::new(
            TerminalSize::default(),
            self.configured_working_dir.as_deref(),
            Some(self.event_wakeup_tx.clone()),
            Some(&self.tab_shell_integration),
            Some(&self.terminal_runtime),
        )
        .expect("Failed to create terminal tab");

        let predicted_prompt_cwd = Self::predicted_prompt_cwd(
            self.configured_working_dir.as_deref(),
            self.terminal_runtime.working_dir_fallback,
        );
        let predicted_title =
            Self::predicted_prompt_seed_title(&self.tab_title, predicted_prompt_cwd.as_deref());

        let tab_id = self.allocate_tab_id();
        self.tabs.push(TerminalTab::new(tab_id, terminal, predicted_title));
        self.active_tab = self.tabs.len() - 1;
        self.refresh_tab_title(self.active_tab);
        self.mark_tab_strip_layout_dirty();
        self.reset_tab_interaction_state();
        self.scroll_active_tab_into_view();
        cx.notify();
    }

    pub(super) fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return;
        }

        self.tabs.remove(index);
        self.mark_tab_strip_layout_dirty();

        if self.active_tab > index {
            self.active_tab -= 1;
        } else if self.active_tab >= self.tabs.len() {
            self.active_tab = self.tabs.len() - 1;
        }

        match self.renaming_tab {
            Some(editing) if editing == index => {
                self.reset_tab_rename_state();
            }
            Some(editing) if editing > index => {
                self.renaming_tab = Some(editing - 1);
            }
            _ => {}
        }

        self.tab_strip.hovered_tab = match self.tab_strip.hovered_tab {
            Some(hovered) if hovered == index => None,
            Some(hovered) if hovered > index => Some(hovered - 1),
            value => value,
        };
        self.tab_strip.hovered_tab_close = match self.tab_strip.hovered_tab_close {
            Some(hovered) if hovered == index => None,
            Some(hovered) if hovered > index => Some(hovered - 1),
            value => value,
        };
        self.reset_tab_drag_state();

        self.clear_selection();
        self.scroll_active_tab_into_view();
        cx.notify();
    }

    pub(super) fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        self.close_tab(self.active_tab, cx);
    }

    pub(super) fn begin_rename_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }

        if self.command_palette_open {
            self.close_command_palette(cx);
        }
        if self.search_open {
            self.close_search(cx);
        }

        if self.active_tab != index {
            self.switch_tab(index, cx);
        }

        self.reset_tab_drag_state();
        self.renaming_tab = Some(index);
        self.rename_input.set_text(self.tabs[index].title.clone());
        self.reset_cursor_blink_phase();
        self.inline_input_selecting = false;
        cx.notify();
    }

    pub(super) fn switch_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() || index == self.active_tab {
            return;
        }

        let old_active = self.active_tab;
        self.active_tab = index;
        if self.tab_width_mode != TabWidthMode::Stable {
            self.mark_tab_strip_layout_dirty();
            self.sync_tab_display_widths_for_viewport_if_needed(
                self.tab_strip.layout_last_synced_viewport_width,
            );
        }

        // Apply inactive_tab_scrollback optimization if configured
        if let Some(inactive_scrollback) = self.inactive_tab_scrollback {
            // Shrink the previously active tab's scrollback to save memory
            self.tabs[old_active]
                .terminal
                .set_scrollback_history(inactive_scrollback);

            // Restore full scrollback for the newly active tab
            self.tabs[index]
                .terminal
                .set_scrollback_history(self.terminal_runtime.scrollback_history);
        }

        self.reset_tab_rename_state();
        self.reset_tab_drag_state();
        self.clear_selection();
        self.scroll_active_tab_into_view();
        cx.notify();
    }

    pub(super) fn commit_rename_tab(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.renaming_tab else {
            return;
        };

        let trimmed = self.rename_input.text().trim();
        self.tabs[index].manual_title = (!trimmed.is_empty())
            .then(|| Self::truncate_tab_title(trimmed))
            .filter(|title| !title.is_empty());
        self.refresh_tab_title(index);

        self.reset_tab_rename_state();
        self.reset_tab_drag_state();
        cx.notify();
    }

    pub(super) fn cancel_rename_tab(&mut self, cx: &mut Context<Self>) {
        if self.renaming_tab.is_none() {
            return;
        }

        self.reset_tab_rename_state();
        self.reset_tab_drag_state();
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_title_width_px(title: &str) -> f32 {
        title.chars().count() as f32 * 7.0
    }

    fn assert_float_eq(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < 0.0001,
            "expected {expected}, got {actual}"
        );
    }

    #[test]
    fn clear_tab_hover_fields_clears_both_slots_and_reports_change() {
        let mut hovered_tab = Some(2);
        let mut hovered_tab_close = Some(4);
        assert!(TerminalView::clear_tab_hover_fields(
            &mut hovered_tab,
            &mut hovered_tab_close
        ));
        assert_eq!(hovered_tab, None);
        assert_eq!(hovered_tab_close, None);
        assert!(!TerminalView::clear_tab_hover_fields(
            &mut hovered_tab,
            &mut hovered_tab_close
        ));
    }

    #[test]
    fn reset_tab_rename_fields_clears_text_focus_and_selection() {
        let mut renaming_tab = Some(1);
        let mut rename_input = InlineInputState::new("rename me".to_string());
        let mut inline_input_selecting = true;
        assert!(TerminalView::reset_tab_rename_fields(
            &mut renaming_tab,
            &mut rename_input,
            &mut inline_input_selecting
        ));
        assert_eq!(renaming_tab, None);
        assert!(rename_input.text().is_empty());
        assert!(!inline_input_selecting);
    }

    #[test]
    fn reset_tab_rename_fields_is_noop_when_state_is_already_clear() {
        let mut renaming_tab = None;
        let mut rename_input = InlineInputState::new(String::new());
        let mut inline_input_selecting = false;
        assert!(!TerminalView::reset_tab_rename_fields(
            &mut renaming_tab,
            &mut rename_input,
            &mut inline_input_selecting
        ));
    }

    #[test]
    fn tab_display_width_for_text_px_clamps_to_min() {
        let width = TerminalView::tab_display_width_for_text_px_with_max(1.0, TAB_MAX_WIDTH);
        assert_eq!(width, TAB_MIN_WIDTH);
    }

    #[test]
    fn tab_display_width_for_text_px_clamps_to_max() {
        let width = TerminalView::tab_display_width_for_text_px_with_max(4000.0, TAB_MAX_WIDTH);
        assert_eq!(width, TAB_MAX_WIDTH);
    }

    #[test]
    fn tab_display_width_for_text_px_tapers_slack_for_short_titles() {
        let long_text_width = 90.0;
        let long_width =
            TerminalView::tab_display_width_for_text_px_with_max(long_text_width, TAB_MAX_WIDTH * 2.0);
        let expected_long = (TAB_TEXT_PADDING_X * 2.0) + long_text_width + TAB_CLOSE_SLOT_WIDTH;
        assert_eq!(long_width, expected_long);

        let short_text_width = 49.0;
        let short_width = TerminalView::tab_display_width_for_text_px_with_max(
            short_text_width,
            TAB_MAX_WIDTH * 2.0,
        );
        let short_base = (TAB_TEXT_PADDING_X * 2.0) + short_text_width + TAB_CLOSE_SLOT_WIDTH;
        assert!(short_width > short_base);
        assert!(short_width < short_base + TAB_TITLE_LAYOUT_SLACK_PX);
    }

    #[test]
    fn tab_display_width_for_text_px_is_monotonic_near_slack_transition() {
        let width_7 = TerminalView::tab_display_width_for_text_px_with_max(49.0, 512.0);
        let width_8 = TerminalView::tab_display_width_for_text_px_with_max(56.0, 512.0);
        let width_9 = TerminalView::tab_display_width_for_text_px_with_max(63.0, 512.0);

        assert!(width_7 < width_8);
        assert!(width_8 < width_9);
    }

    #[test]
    fn tab_display_width_for_text_px_with_max_uses_provided_cap() {
        let width = TerminalView::tab_display_width_for_text_px_with_max(4000.0, 512.0);
        assert_eq!(width, 512.0);
    }

    #[test]
    fn effective_tab_max_width_grows_for_sparse_tabs() {
        let effective = TerminalView::effective_tab_max_width_for_viewport(1600.0, 1);
        assert!(effective > TAB_MAX_WIDTH);
    }

    #[test]
    fn effective_tab_max_width_stays_baseline_for_crowded_tabs() {
        let effective = TerminalView::effective_tab_max_width_for_viewport(1600.0, 8);
        assert_float_eq(effective, TAB_MAX_WIDTH);
    }

    #[test]
    fn effective_tab_max_width_respects_hard_cap_ratio() {
        let viewport_width = 4000.0;
        let content_width = (viewport_width - (TAB_HORIZONTAL_PADDING * 2.0)).max(TAB_MAX_WIDTH);
        let expected_hard_cap = (content_width * TAB_ADAPTIVE_HARD_CAP_RATIO).max(TAB_MAX_WIDTH);
        let effective = TerminalView::effective_tab_max_width_for_viewport(viewport_width, 1);
        assert_float_eq(effective, expected_hard_cap);
    }

    #[test]
    fn remap_index_after_move_handles_move_to_right() {
        assert_eq!(TerminalView::remap_index_after_move(1, 1, 3), 3);
        assert_eq!(TerminalView::remap_index_after_move(2, 1, 3), 1);
        assert_eq!(TerminalView::remap_index_after_move(3, 1, 3), 2);
        assert_eq!(TerminalView::remap_index_after_move(0, 1, 3), 0);
    }

    #[test]
    fn remap_index_after_move_handles_move_to_left() {
        assert_eq!(TerminalView::remap_index_after_move(3, 3, 1), 1);
        assert_eq!(TerminalView::remap_index_after_move(1, 3, 1), 2);
        assert_eq!(TerminalView::remap_index_after_move(2, 3, 1), 3);
        assert_eq!(TerminalView::remap_index_after_move(4, 3, 1), 4);
    }

    #[test]
    fn tab_index_for_id_follows_reordered_tab_sequence() {
        let tab_ids: [TabId; 4] = [11, 13, 17, 19];
        assert_eq!(TerminalView::tab_index_for_id_in_order(tab_ids, 17), Some(2));

        // Simulate drag reorder 17 -> slot 1
        let reordered: [TabId; 4] = [11, 17, 13, 19];
        assert_eq!(TerminalView::tab_index_for_id_in_order(reordered, 17), Some(1));
    }

    #[test]
    fn tab_index_for_id_returns_none_after_tab_close() {
        let tab_ids: [TabId; 4] = [11, 13, 17, 19];
        let after_close: [TabId; 3] = [11, 13, 19];
        assert_eq!(TerminalView::tab_index_for_id_in_order(tab_ids, 17), Some(2));
        assert_eq!(TerminalView::tab_index_for_id_in_order(after_close, 17), None);
    }

    #[test]
    fn tab_shows_close_for_active_or_hovered() {
        assert!(TerminalView::tab_shows_close(
            TabCloseVisibility::ActiveHover,
            true,
            None,
            None,
            1,
        ));
        assert!(TerminalView::tab_shows_close(
            TabCloseVisibility::ActiveHover,
            false,
            Some(1),
            None,
            1,
        ));
        assert!(TerminalView::tab_shows_close(
            TabCloseVisibility::ActiveHover,
            false,
            None,
            Some(1),
            1,
        ));
        assert!(!TerminalView::tab_shows_close(
            TabCloseVisibility::ActiveHover,
            false,
            Some(2),
            None,
            1,
        ));
        assert!(!TerminalView::tab_shows_close(
            TabCloseVisibility::ActiveHover,
            false,
            None,
            Some(2),
            1,
        ));
    }

    #[test]
    fn tab_shows_close_hover_mode_ignores_active_state() {
        assert!(!TerminalView::tab_shows_close(
            TabCloseVisibility::Hover,
            true,
            None,
            None,
            0,
        ));
        assert!(TerminalView::tab_shows_close(
            TabCloseVisibility::Hover,
            false,
            Some(0),
            None,
            0,
        ));
    }

    #[test]
    fn tab_shows_close_always_mode_always_true() {
        assert!(TerminalView::tab_shows_close(
            TabCloseVisibility::Always,
            false,
            None,
            None,
            2,
        ));
    }

    #[test]
    fn active_grow_mode_reserves_close_slot_for_active_only() {
        let text_width = synthetic_title_width_px("~/Desktop");
        let max_width = 512.0;
        let expected_active = TerminalView::tab_display_width_for_text_px_with_close_policy(
            text_width, max_width, true,
        );
        let expected_inactive = TerminalView::tab_display_width_for_text_px_with_close_policy(
            text_width, max_width, false,
        );

        let (active_width, active_sticky) = TerminalView::resolve_tab_width_for_mode(
            TabWidthMode::ActiveGrow,
            text_width,
            max_width,
            true,
            TAB_MIN_WIDTH,
        );
        let (inactive_width, inactive_sticky) = TerminalView::resolve_tab_width_for_mode(
            TabWidthMode::ActiveGrow,
            text_width,
            max_width,
            false,
            TAB_MIN_WIDTH,
        );

        assert_eq!(active_width, expected_active);
        assert_eq!(inactive_width, expected_inactive);
        assert_eq!(active_sticky, expected_inactive);
        assert_eq!(inactive_sticky, expected_inactive);
        assert!(active_width > inactive_width);
    }

    #[test]
    fn active_grow_sticky_drops_close_only_extra_when_inactive() {
        let text_width = synthetic_title_width_px("~/Desktop");
        let max_width = 512.0;
        let title_only = TerminalView::tab_display_width_for_text_px_without_close_with_max(
            text_width, max_width,
        );
        let with_close = TerminalView::tab_display_width_for_text_px_with_close_policy(
            text_width, max_width, true,
        );

        let (active_width, sticky_after_active) = TerminalView::resolve_tab_width_for_mode(
            TabWidthMode::ActiveGrowSticky,
            text_width,
            max_width,
            true,
            title_only,
        );
        let (inactive_width, sticky_after_inactive) = TerminalView::resolve_tab_width_for_mode(
            TabWidthMode::ActiveGrowSticky,
            text_width,
            max_width,
            false,
            sticky_after_active,
        );

        assert_eq!(active_width, with_close);
        assert_eq!(inactive_width, title_only);
        assert_eq!(sticky_after_inactive, title_only);
    }

    #[test]
    fn active_grow_sticky_respects_manual_sticky_reset_to_current_title() {
        let long_text_width = synthetic_title_width_px(
            "~/Desktop/claudeCode/claude-code-provider-proxy/docs/test2/test4/test4",
        );
        let short_text_width = synthetic_title_width_px("~/Desktop");
        let max_width = 512.0;
        let short_title_only =
            TerminalView::tab_display_width_for_text_px_without_close_with_max(short_text_width, max_width);

        let (_, sticky_after_long) = TerminalView::resolve_tab_width_for_mode(
            TabWidthMode::ActiveGrowSticky,
            long_text_width,
            max_width,
            false,
            0.0,
        );
        let (width_without_reset, _) = TerminalView::resolve_tab_width_for_mode(
            TabWidthMode::ActiveGrowSticky,
            short_text_width,
            max_width,
            false,
            sticky_after_long,
        );
        let (width_with_reset, sticky_with_reset) = TerminalView::resolve_tab_width_for_mode(
            TabWidthMode::ActiveGrowSticky,
            short_text_width,
            max_width,
            false,
            0.0,
        );

        assert!(width_without_reset > width_with_reset);
        assert_eq!(width_with_reset, short_title_only);
        assert_eq!(sticky_with_reset, short_title_only);
    }

    #[test]
    fn active_grow_sticky_caps_sticky_width_under_pressure() {
        let text_width = synthetic_title_width_px("tab");
        let effective_max = 118.0;
        let prior_sticky = 260.0;

        let (next_width, next_sticky) = TerminalView::resolve_tab_width_for_mode(
            TabWidthMode::ActiveGrowSticky,
            text_width,
            effective_max,
            false,
            prior_sticky,
        );

        assert_eq!(next_sticky, effective_max);
        assert_eq!(next_width, effective_max);
    }

    #[test]
    fn normalized_drop_slot_filters_noop_boundaries() {
        assert_eq!(TerminalView::normalized_drop_slot(2, 2), None);
        assert_eq!(TerminalView::normalized_drop_slot(2, 3), None);
        assert_eq!(TerminalView::normalized_drop_slot(2, 1), Some(1));
        assert_eq!(TerminalView::normalized_drop_slot(2, 4), Some(4));
    }

    #[test]
    fn reorder_target_index_for_drop_slot_moves_right_correctly() {
        assert_eq!(TerminalView::reorder_target_index_for_drop_slot(1, 3), 2);
        assert_eq!(TerminalView::reorder_target_index_for_drop_slot(0, 3), 2);
    }

    #[test]
    fn reorder_target_index_for_drop_slot_moves_left_correctly() {
        assert_eq!(TerminalView::reorder_target_index_for_drop_slot(3, 1), 1);
        assert_eq!(TerminalView::reorder_target_index_for_drop_slot(2, 0), 0);
    }

    #[test]
    fn tab_drop_slot_from_pointer_x_respects_midpoints() {
        let widths = [100.0, 100.0, 100.0];
        assert_eq!(
            TerminalView::tab_drop_slot_from_pointer_x_for_widths(widths, 40.0, 0.0),
            0
        );
        assert_eq!(
            TerminalView::tab_drop_slot_from_pointer_x_for_widths(widths, 70.0, 0.0),
            1
        );
        assert_eq!(
            TerminalView::tab_drop_slot_from_pointer_x_for_widths(widths, 170.0, 0.0),
            2
        );
        assert_eq!(
            TerminalView::tab_drop_slot_from_pointer_x_for_widths(widths, 270.0, 0.0),
            3
        );
    }

    #[test]
    fn tab_drop_slot_from_pointer_x_respects_scroll_offset() {
        let widths = [100.0, 100.0];
        assert_eq!(
            TerminalView::tab_drop_slot_from_pointer_x_for_widths(widths, 40.0, 0.0),
            0
        );
        assert_eq!(
            TerminalView::tab_drop_slot_from_pointer_x_for_widths(widths, 40.0, -30.0),
            1
        );
    }

    #[test]
    fn tab_drop_marker_side_maps_slot_to_left_and_right_edges() {
        assert_eq!(
            TerminalView::tab_drop_marker_side_for_slot(2, 2),
            Some(TabDropMarkerSide::Left)
        );
        assert_eq!(
            TerminalView::tab_drop_marker_side_for_slot(2, 3),
            Some(TabDropMarkerSide::Right)
        );
        assert_eq!(TerminalView::tab_drop_marker_side_for_slot(2, 1), None);
    }

    #[test]
    fn tab_drop_slot_mapping_is_stable_with_adaptive_widths() {
        let effective_max = TerminalView::effective_tab_max_width_for_viewport(1500.0, 3);
        let widths = [
            TerminalView::tab_display_width_for_text_px_with_max(
                synthetic_title_width_px("~/Desktop/claudeCode/claude-code-provider-proxy/docs"),
                effective_max,
            ),
            TerminalView::tab_display_width_for_text_px_with_max(
                synthetic_title_width_px("~"),
                effective_max,
            ),
            TerminalView::tab_display_width_for_text_px_with_max(
                synthetic_title_width_px("~/projects/termy"),
                effective_max,
            ),
        ];

        let first_midpoint = TAB_HORIZONTAL_PADDING + (widths[0] * 0.5);
        assert_eq!(
            TerminalView::tab_drop_slot_from_pointer_x_for_widths(
                widths,
                first_midpoint - 1.0,
                0.0,
            ),
            0
        );
        assert_eq!(
            TerminalView::tab_drop_slot_from_pointer_x_for_widths(
                widths,
                first_midpoint + 1.0,
                0.0,
            ),
            1
        );
    }

    #[test]
    fn tab_strip_overflow_state_reports_none_without_scroll_range() {
        assert_eq!(
            TerminalView::tab_strip_overflow_state_for_scroll(0.0, 0.0),
            TabStripOverflowState::default()
        );
    }

    #[test]
    fn tab_strip_overflow_state_reports_right_overflow_at_start() {
        assert_eq!(
            TerminalView::tab_strip_overflow_state_for_scroll(0.0, 120.0),
            TabStripOverflowState {
                left: false,
                right: true,
            }
        );
    }

    #[test]
    fn tab_strip_overflow_state_reports_left_overflow_at_end() {
        assert_eq!(
            TerminalView::tab_strip_overflow_state_for_scroll(120.0, 120.0),
            TabStripOverflowState {
                left: true,
                right: false,
            }
        );
    }

    #[test]
    fn tab_strip_overflow_state_reports_both_when_scrolled_in_middle() {
        assert_eq!(
            TerminalView::tab_strip_overflow_state_for_scroll(42.0, 120.0),
            TabStripOverflowState {
                left: true,
                right: true,
            }
        );
    }

    #[test]
    fn tab_strip_offset_x_for_delta_clamps_to_left_limit() {
        assert_eq!(
            TerminalView::tab_strip_offset_x_for_delta(-24.0, 96.0, 120.0),
            Some(0.0)
        );
    }

    #[test]
    fn tab_strip_offset_x_for_delta_clamps_to_right_limit() {
        assert_eq!(
            TerminalView::tab_strip_offset_x_for_delta(-96.0, -64.0, 120.0),
            Some(-120.0)
        );
    }

    #[test]
    fn tab_strip_offset_x_for_delta_is_noop_without_scroll_range() {
        assert_eq!(
            TerminalView::tab_strip_offset_x_for_delta(0.0, 24.0, 0.0),
            None
        );
    }

    #[test]
    fn tab_strip_geometry_positions_action_rail_and_button_bounds() {
        let geometry = TerminalView::tab_strip_geometry_for_viewport_width(1280.0);
        assert!(geometry.row_start_x > 0.0);
        assert!(geometry.tabs_viewport_width > 0.0);
        assert_float_eq(geometry.tabs_viewport_end_x(), geometry.gutter_start_x);
        assert_float_eq(geometry.gutter_end_x(), geometry.action_rail_start_x);
        assert_float_eq(
            geometry.action_rail_end_x(),
            geometry.action_rail_start_x + geometry.action_rail_width,
        );
        assert!(geometry.button_start_x >= geometry.action_rail_start_x);
        assert!(geometry.button_end_x <= geometry.action_rail_end_x());
        assert!(geometry.button_start_y >= TOP_STRIP_CONTENT_OFFSET_Y);
    }

    #[test]
    fn tab_strip_geometry_clamps_action_rail_for_narrow_viewport() {
        let viewport_width =
            TerminalView::titlebar_left_padding_for_platform() + TOP_STRIP_SIDE_PADDING + 24.0;
        let geometry = TerminalView::tab_strip_geometry_for_viewport_width(viewport_width);

        assert_float_eq(geometry.row_width, 24.0);
        assert_float_eq(geometry.action_rail_width, 24.0);
        assert_float_eq(geometry.tabs_viewport_width, 0.0);
        assert!(geometry.button_start_x >= geometry.action_rail_start_x);
        assert!(geometry.button_end_x <= geometry.action_rail_end_x());
    }

    #[test]
    fn tab_strip_geometry_uses_half_open_bounds_between_regions() {
        let geometry = TerminalView::tab_strip_geometry_for_viewport_width(1280.0);
        let boundary_x = geometry.tabs_viewport_end_x();
        assert!(!geometry.contains_tabs_viewport_x(boundary_x));
        assert!(geometry.contains_gutter_x(boundary_x));
        assert!(geometry.contains_tabs_viewport_x(boundary_x - 1.0));
        assert!(geometry.contains_action_rail_x(geometry.gutter_end_x()));
    }

    #[test]
    fn tab_strip_pointer_transform_accounts_for_non_zero_origin() {
        let geometry = TerminalView::tab_strip_geometry_for_viewport_width(1280.0);
        assert_float_eq(
            TerminalView::tab_strip_pointer_x_from_window_x_for_geometry(
                geometry.row_start_x + 24.0,
                geometry,
            ),
            24.0,
        );
        assert_float_eq(
            TerminalView::tab_strip_pointer_x_from_window_x_for_geometry(0.0, geometry),
            0.0,
        );
    }

    #[test]
    fn tab_strip_geometry_detects_new_tab_button_hit_bounds() {
        let geometry = TerminalView::tab_strip_geometry_for_viewport_width(960.0);
        let button_center_x = (geometry.button_start_x + geometry.button_end_x) * 0.5;
        let button_center_y = (geometry.button_start_y + geometry.button_end_y) * 0.5;
        assert!(geometry.contains_action_rail_x(button_center_x));
        assert!(geometry.new_tab_button_contains(button_center_x, button_center_y));
        assert!(!geometry.new_tab_button_contains(geometry.button_start_x - 1.0, button_center_y));
    }
}
