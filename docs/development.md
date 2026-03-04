# Development

## Render metrics (debug-only)

Enable render churn metrics in debug builds:

```sh
RUST_LOG=info TERMY_RENDER_METRICS=1 cargo run -p termy
```

Note:
- Metrics logs are `debug_assertions`-only, so `--release` will not emit `render_metrics` lines.
- Counter meaning:
  - `full`: full per-pane cell cache rebuild decisions
  - `partial`: dirty-span patch decisions
  - `reuse`: no cell cache update decisions
  - `dirty_span`: number of dirty spans consumed during partial updates
  - `patched_cell`: number of cells patched from dirty spans
  - `grid_paint` / `shape_line`: paint + text shaping work done that interval
- Cursor-blink sanity check: `full` should stay near `0`; `reuse` or small `partial` values are expected depending on reported terminal damage.

## Tmux integration tests

Run the local end-to-end tmux split integration harness:

```sh
just test-tmux-integration
```

Requirements:
- tmux `>= 3.3`

Optional:
- Override tmux binary path with `TERMY_TEST_TMUX_BIN=/path/to/tmux`
