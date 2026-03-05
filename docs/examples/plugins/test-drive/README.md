# Test Drive Plugin

Use this plugin to verify the Plugins settings tab.

## Install

1. Open `Settings -> Plugins`
2. Click `Install From Folder`
3. Choose this `test-drive` directory
4. Click `Start`

## What to expect

- a success toast when the plugin connects
- recent log lines in the plugin card
- a command palette entry named `Test Drive Plugin: Test Drive Ping`
- invoking that command emits another toast and log line
- `Stop` cleanly terminates the plugin
- `Remove` deletes the installed copy from the plugin directory
