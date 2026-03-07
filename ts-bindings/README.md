# Termy TypeScript bindings

TypeScript SDK for Termy plugins that speak the existing newline-delimited JSON protocol over stdio.

## What this package provides

- Protocol constants and manifest/message types aligned with `termy_plugin_core`
- `PluginSession` that mirrors the Rust SDK handshake and message loop
- Bun-friendly stdio support for plugins compiled to a single executable

## Build the bindings package

```bash
bun install
bun run build
```

## Run tests

```bash
bun test
bun run type-check
```

## Minimal plugin example

Remember to install 

```bash
bun add @termy-oss/ts-bindings
```

See [`examples/hello-plugin/plugin.ts`](/Users/lassevestergaard/Documents/dev/termy/ts-bindings/examples/hello-plugin/plugin.ts) and [`examples/hello-plugin/termy-plugin.json`](/Users/lassevestergaard/Documents/dev/termy/ts-bindings/examples/hello-plugin/termy-plugin.json).

The example follows the same flow as the Rust SDK:

1. Read the host hello from stdin
2. Reply with plugin hello
3. React to `ping`, `invoke_command`, and `shutdown`

## Compile a Termy plugin to a single Bun executable

```bash
bun build --compile ./examples/hello-plugin/plugin.ts --outfile ./dist/hello-plugin
```

Then point your plugin manifest at the compiled binary:

```json
{
  "schema_version": 1,
  "id": "example.hello",
  "name": "Hello Plugin",
  "version": "0.1.0",
  "description": "Minimal TypeScript Termy plugin compiled with Bun",
  "runtime": "executable",
  "entrypoint": "./dist/hello-plugin",
  "autostart": true,
  "permissions": ["notifications"],
  "contributes": {
    "commands": [
      {
        "id": "example.hello.run",
        "title": "Run Hello",
        "description": "Show a sample toast from the TypeScript example plugin"
      }
    ]
  }
}
```

No Termy host change is required for Bun plugins in this model. `termy_plugin_host` already launches plugins as executables and communicates over stdio.
