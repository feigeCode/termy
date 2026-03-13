# Full Example Plugin

This is a standalone, installable Termy plugin example.

It demonstrates:

- command contributions
- host event subscriptions
- settings panel updates
- interactive panel actions
- toast notifications

## Build locally

```bash
cargo build --manifest-path examples/plugin-full/Cargo.toml --release
```

## Test locally

```bash
cargo test --manifest-path examples/plugin-full/Cargo.toml
```

## Install manually

Create a plugin directory and copy the manifest plus built binary:

```bash
mkdir -p ~/.config/termy/plugins/example.full
cp examples/plugin-full/termy-plugin.json ~/.config/termy/plugins/example.full/
cp examples/plugin-full/target/release/plugin-full ~/.config/termy/plugins/example.full/
```

Then open `Settings -> Plugins` in Termy and start `Full Example Plugin`.

## CI artifact

GitHub Actions packages this example as a tarball containing:

- `termy-plugin.json`
- `README.md`
- the built `plugin-full` executable
