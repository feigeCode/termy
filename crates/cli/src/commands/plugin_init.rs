use std::{
    fs,
    path::{Path, PathBuf},
};

use termy_plugin_core::PLUGIN_MANIFEST_FILE_NAME;
use termy_plugin_host::default_plugins_dir;

const DEFAULT_PLUGIN_ID: &str = "example.hello";
const DEFAULT_PLUGIN_NAME: &str = "Hello Plugin";
const DEFAULT_PLUGIN_VERSION: &str = "0.1.0";
const DEFAULT_COMMAND_SUFFIX: &str = ".run";

pub fn run() {
    let Some(root_dir) = default_plugins_dir() else {
        eprintln!(
            "Plugin directory is unavailable because the Termy config path could not be resolved."
        );
        std::process::exit(1);
    };

    match create_plugin_scaffold(&root_dir) {
        Ok(scaffold) => {
            println!("Created plugin scaffold:");
            println!("  id: {}", scaffold.plugin_id);
            println!("  path: {}", scaffold.plugin_dir.display());
            println!("  manifest: {}", scaffold.manifest_path.display());
            println!("  entrypoint: {}", scaffold.entrypoint_path.display());
            println!();
            println!("Inspect it with:");
            println!("  cargo run -p termy_cli -- -list-plugins");
        }
        Err(error) => {
            eprintln!("Failed to create plugin scaffold: {error}");
            std::process::exit(1);
        }
    }
}

#[derive(Debug)]
struct PluginScaffold {
    plugin_id: String,
    plugin_dir: PathBuf,
    manifest_path: PathBuf,
    entrypoint_path: PathBuf,
}

fn create_plugin_scaffold(root_dir: &Path) -> std::io::Result<PluginScaffold> {
    fs::create_dir_all(root_dir)?;

    let plugin_id = next_plugin_id(root_dir);
    let plugin_dir = root_dir.join(&plugin_id);
    fs::create_dir(&plugin_dir)?;

    let entrypoint_name = entrypoint_name();
    let manifest_path = plugin_dir.join(PLUGIN_MANIFEST_FILE_NAME);
    let entrypoint_path = plugin_dir.join(entrypoint_name);
    let command_id = format!("{plugin_id}{DEFAULT_COMMAND_SUFFIX}");
    let plugin_name = plugin_name_for(&plugin_id);

    fs::write(
        &manifest_path,
        manifest_template(&plugin_id, &plugin_name, entrypoint_name, &command_id),
    )?;
    fs::write(
        &entrypoint_path,
        entrypoint_template(&plugin_id, &plugin_name, &command_id),
    )?;
    set_executable_if_supported(&entrypoint_path)?;

    Ok(PluginScaffold {
        plugin_id,
        plugin_dir,
        manifest_path,
        entrypoint_path,
    })
}

fn next_plugin_id(root_dir: &Path) -> String {
    if !root_dir.join(DEFAULT_PLUGIN_ID).exists() {
        return DEFAULT_PLUGIN_ID.to_string();
    }

    for suffix in 2.. {
        let candidate = format!("{DEFAULT_PLUGIN_ID}-{suffix}");
        if !root_dir.join(&candidate).exists() {
            return candidate;
        }
    }

    unreachable!("integer range for plugin suffix is exhausted");
}

fn plugin_name_for(plugin_id: &str) -> String {
    if plugin_id == DEFAULT_PLUGIN_ID {
        DEFAULT_PLUGIN_NAME.to_string()
    } else {
        let suffix = plugin_id
            .trim_start_matches(DEFAULT_PLUGIN_ID)
            .trim_start_matches('-');
        format!("{DEFAULT_PLUGIN_NAME} {suffix}")
    }
}

fn manifest_template(
    plugin_id: &str,
    plugin_name: &str,
    entrypoint_name: &str,
    command_id: &str,
) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "id": "{plugin_id}",
  "name": "{plugin_name}",
  "version": "{DEFAULT_PLUGIN_VERSION}",
  "description": "Starter plugin scaffold created by termy_cli",
  "runtime": "executable",
  "entrypoint": "./{entrypoint_name}",
  "autostart": false,
  "permissions": ["notifications", "ui_panels", "host_events"],
  "subscribes": {{
    "events": ["app_started", "theme_changed", "active_tab_changed"]
  }},
  "contributes": {{
    "commands": [
      {{
        "id": "{command_id}",
        "title": "Run {plugin_name}",
        "description": "Starter command created by termy_cli"
      }}
    ]
  }}
}}
"#
    )
}

#[cfg(target_os = "windows")]
fn entrypoint_name() -> &'static str {
    "plugin.cmd"
}

#[cfg(not(target_os = "windows"))]
fn entrypoint_name() -> &'static str {
    "plugin.sh"
}

#[cfg(target_os = "windows")]
fn entrypoint_template(plugin_id: &str, plugin_name: &str, command_id: &str) -> String {
    format!(
        r#"@echo off
setlocal enabledelayedexpansion

for /f "usebackq delims=" %%L in (`more`) do (
  set "line=%%L"
  echo(!line! | findstr /c:"""type"":""hello""" >nul && (
    echo {{"type":"hello","payload":{{"protocol_version":1,"plugin_id":"{plugin_id}","name":"{plugin_name}","version":"{DEFAULT_PLUGIN_VERSION}","capabilities":["command_provider","event_subscriber","ui_panel"]}}}}
    echo {{"type":"panel","payload":{{"title":"{plugin_name}","body":"Ready. Invoke {command_id} or change the active tab/theme."}}}}
  )
  echo(!line! | findstr /c:"""type"":""ping""" >nul && (
    echo {{"type":"pong"}}
  )
  echo(!line! | findstr /c:"""type"":""invoke_command""" >nul && echo(!line! | findstr /c:"""command_id"":""{command_id}""" >nul && (
    echo {{"type":"log","payload":{{"level":"info","message":"{command_id} invoked"}}}}
    echo {{"type":"toast","payload":{{"level":"success","message":"Hello from {plugin_id}","duration_ms":2000}}}}
    echo {{"type":"panel","payload":{{"title":"{plugin_name}","body":"Command {command_id} just ran successfully."}}}}
  )
  echo(!line! | findstr /c:"""type"":""event""" >nul && echo(!line! | findstr /c:"""event"":""app_started""" >nul && (
    echo {{"type":"log","payload":{{"level":"info","message":"observed app_started"}}}}
    echo {{"type":"panel","payload":{{"title":"{plugin_name}","body":"Observed host startup."}}}}
  )
  echo(!line! | findstr /c:"""type"":""event""" >nul && echo(!line! | findstr /c:"""event"":""theme_changed""" >nul && (
    echo {{"type":"log","payload":{{"level":"info","message":"observed theme_changed"}}}}
    echo {{"type":"panel","payload":{{"title":"{plugin_name}","body":"Observed a theme change event."}}}}
  )
  echo(!line! | findstr /c:"""type"":""event""" >nul && echo(!line! | findstr /c:"""event"":""active_tab_changed""" >nul && (
    echo {{"type":"log","payload":{{"level":"info","message":"observed active_tab_changed"}}}}
    echo {{"type":"panel","payload":{{"title":"{plugin_name}","body":"Observed an active tab change event."}}}}
  )
  echo(!line! | findstr /c:"""type"":""shutdown""" >nul && exit /b 0
)
"#
    )
}

#[cfg(not(target_os = "windows"))]
fn entrypoint_template(plugin_id: &str, plugin_name: &str, command_id: &str) -> String {
    format!(
        r#"#!/bin/sh
set -eu

while IFS= read -r line; do
  case "$line" in
    *'"type":"hello"'*)
      printf '%s\n' '{{"type":"hello","payload":{{"protocol_version":1,"plugin_id":"{plugin_id}","name":"{plugin_name}","version":"{DEFAULT_PLUGIN_VERSION}","capabilities":["command_provider","event_subscriber","ui_panel"]}}}}'
      printf '%s\n' '{{"type":"panel","payload":{{"title":"{plugin_name}","body":"Ready. Invoke {command_id} or change the active tab/theme."}}}}'
      ;;
    *'"type":"ping"'*)
      printf '%s\n' '{{"type":"pong"}}'
      ;;
    *'"type":"invoke_command"'*)
      case "$line" in
        *'"command_id":"{command_id}"'*)
          printf '%s\n' '{{"type":"log","payload":{{"level":"info","message":"{command_id} invoked"}}}}'
          printf '%s\n' '{{"type":"toast","payload":{{"level":"success","message":"Hello from {plugin_id}","duration_ms":2000}}}}'
          printf '%s\n' '{{"type":"panel","payload":{{"title":"{plugin_name}","body":"Command {command_id} just ran successfully."}}}}'
          ;;
      esac
      ;;
    *'"type":"event"'*'"event":"app_started"'*)
      printf '%s\n' '{{"type":"log","payload":{{"level":"info","message":"observed app_started"}}}}'
      printf '%s\n' '{{"type":"panel","payload":{{"title":"{plugin_name}","body":"Observed host startup."}}}}'
      ;;
    *'"type":"event"'*'"event":"theme_changed"'*)
      printf '%s\n' '{{"type":"log","payload":{{"level":"info","message":"observed theme_changed"}}}}'
      printf '%s\n' '{{"type":"panel","payload":{{"title":"{plugin_name}","body":"Observed a theme change event."}}}}'
      ;;
    *'"type":"event"'*'"event":"active_tab_changed"'*)
      printf '%s\n' '{{"type":"log","payload":{{"level":"info","message":"observed active_tab_changed"}}}}'
      printf '%s\n' '{{"type":"panel","payload":{{"title":"{plugin_name}","body":"Observed an active tab change event."}}}}'
      ;;
    *'"type":"shutdown"'*)
      exit 0
      ;;
  esac
done
"#
    )
}

#[cfg(unix)]
fn set_executable_if_supported(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path)?.permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
}

#[cfg(not(unix))]
fn set_executable_if_supported(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::create_plugin_scaffold;
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn temp_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("termy-cli-plugin-init-{name}-{unique}"))
    }

    #[test]
    fn creates_manifest_and_entrypoint() {
        let root = temp_root("creates-files");
        let scaffold = create_plugin_scaffold(&root).expect("create scaffold");

        let manifest = fs::read_to_string(&scaffold.manifest_path).expect("read manifest");
        let entrypoint = fs::read_to_string(&scaffold.entrypoint_path).expect("read entrypoint");

        assert_eq!(scaffold.plugin_id, "example.hello");
        assert!(manifest.contains(r#""id": "example.hello""#));
        assert!(manifest.contains(r#""entrypoint": "./"#));
        assert!(entrypoint.contains("command_provider"));
        assert!(manifest.contains(r#""ui_panels""#));
        assert!(manifest.contains(r#""host_events""#));
        assert!(manifest.contains(r#""subscribes""#));
        assert!(entrypoint.contains("event_subscriber"));
        assert!(entrypoint.contains("\"type\":\"panel\""));

        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn increments_plugin_id_when_default_exists() {
        let root = temp_root("increments-id");
        fs::create_dir_all(root.join("example.hello")).expect("create existing plugin dir");

        let scaffold = create_plugin_scaffold(&root).expect("create scaffold");

        assert_eq!(scaffold.plugin_id, "example.hello-2");
        assert!(scaffold.plugin_dir.exists());

        fs::remove_dir_all(root).ok();
    }
}
