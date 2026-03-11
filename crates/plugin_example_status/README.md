# Status Example Plugin

This crate is a reference Termy plugin written in Rust.

It demonstrates:

- command handling
- host event subscriptions
- toast notifications
- settings panel updates
- panel action buttons wired to contributed commands

Build it with:

```bash
cargo build -p plugin_example_status
```

To install it locally, copy `crates/plugin_example_status/termy-plugin.json` and the built binary into a plugin folder under `~/.config/termy/plugins/example.status/`.
