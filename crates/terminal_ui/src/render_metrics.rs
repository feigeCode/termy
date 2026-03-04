#[cfg(any(debug_assertions, test))]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(any(debug_assertions, test))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerminalUiRenderMetricsSnapshot {
    pub grid_paint_count: u64,
    pub shape_line_calls: u64,
}

#[cfg(any(debug_assertions, test))]
impl TerminalUiRenderMetricsSnapshot {
    pub fn saturating_sub(self, previous: Self) -> Self {
        Self {
            grid_paint_count: self.grid_paint_count.saturating_sub(previous.grid_paint_count),
            shape_line_calls: self.shape_line_calls.saturating_sub(previous.shape_line_calls),
        }
    }
}

// Keep render metrics active in tests, including `cargo test --release`.
#[cfg(any(debug_assertions, test))]
static GRID_PAINT_COUNT: AtomicU64 = AtomicU64::new(0);
#[cfg(any(debug_assertions, test))]
static SHAPE_LINE_CALLS: AtomicU64 = AtomicU64::new(0);

#[cfg(any(debug_assertions, test))]
fn increment_counter(counter: &AtomicU64) {
    let _ = counter.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
        Some(current.saturating_add(1))
    });
}

#[cfg(any(debug_assertions, test))]
pub(crate) fn increment_grid_paint_count() {
    increment_counter(&GRID_PAINT_COUNT);
}

#[cfg(not(any(debug_assertions, test)))]
pub(crate) fn increment_grid_paint_count() {}

#[cfg(any(debug_assertions, test))]
pub(crate) fn increment_shape_line_calls() {
    increment_counter(&SHAPE_LINE_CALLS);
}

#[cfg(not(any(debug_assertions, test)))]
pub(crate) fn increment_shape_line_calls() {}

#[cfg(any(debug_assertions, test))]
pub fn terminal_ui_render_metrics_snapshot() -> TerminalUiRenderMetricsSnapshot {
    TerminalUiRenderMetricsSnapshot {
        grid_paint_count: GRID_PAINT_COUNT.load(Ordering::Relaxed),
        shape_line_calls: SHAPE_LINE_CALLS.load(Ordering::Relaxed),
    }
}

#[cfg(any(debug_assertions, test))]
pub fn terminal_ui_render_metrics_reset() {
    GRID_PAINT_COUNT.store(0, Ordering::Relaxed);
    SHAPE_LINE_CALLS.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_is_zero_after_reset() {
        terminal_ui_render_metrics_reset();
        assert_eq!(terminal_ui_render_metrics_snapshot(), TerminalUiRenderMetricsSnapshot::default());
    }

    #[test]
    fn increment_grid_paint_updates_snapshot() {
        terminal_ui_render_metrics_reset();
        increment_grid_paint_count();
        let snapshot = terminal_ui_render_metrics_snapshot();
        assert_eq!(snapshot.grid_paint_count, 1);
        assert_eq!(snapshot.shape_line_calls, 0);
    }

    #[test]
    fn increment_shape_line_updates_snapshot() {
        terminal_ui_render_metrics_reset();
        increment_shape_line_calls();
        increment_shape_line_calls();
        let snapshot = terminal_ui_render_metrics_snapshot();
        assert_eq!(snapshot.grid_paint_count, 0);
        assert_eq!(snapshot.shape_line_calls, 2);
    }

    #[test]
    fn reset_clears_counters_after_increments() {
        terminal_ui_render_metrics_reset();
        increment_grid_paint_count();
        increment_shape_line_calls();
        terminal_ui_render_metrics_reset();
        assert_eq!(terminal_ui_render_metrics_snapshot(), TerminalUiRenderMetricsSnapshot::default());
    }
}
