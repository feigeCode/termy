use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::Path,
    sync::{LazyLock, Mutex},
};

use fs4::FileExt;
use termy_config_core::{Rgb8, canonical_color_key, parse_theme_id};

use super::ConfigIoError;
use super::io::{ensure_config_file, notify_config_changed, write_atomic};

static CONFIG_UPDATE_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

fn update_config_contents<R>(
    updater: impl FnOnce(&str) -> Result<(String, R), String>,
) -> Result<R, String> {
    let _process_guard = CONFIG_UPDATE_LOCK
        .lock()
        .map_err(|_| "Failed to acquire config update lock".to_string())?;
    let config_path = ensure_config_file().map_err(|error| error.to_string())?;
    let lock_path = config_path.with_extension("lock");
    let lock_path_display = lock_path.display().to_string();
    let process_lock_file = fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&lock_path)
        .map_err(|source| {
            format!(
                "Failed to open config lock file '{}': {}",
                lock_path_display, source
            )
        })?;
    process_lock_file.lock_exclusive().map_err(|source| {
        format!(
            "Failed to lock config lock file '{}': {}",
            lock_path_display, source
        )
    })?;

    let mut config_lock_file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&config_path)
        .map_err(|source| ConfigIoError::ReadConfig {
            path: config_path.clone(),
            source,
        })
        .map_err(|error| error.to_string())?;
    config_lock_file.lock_exclusive().map_err(|source| {
        format!(
            "Failed to lock config file '{}': {}",
            config_path.display(),
            source
        )
    })?;

    let mut existing = String::new();
    config_lock_file
        .read_to_string(&mut existing)
        .map_err(|source| ConfigIoError::ReadConfig {
            path: config_path.clone(),
            source,
        })
        .map_err(|error| error.to_string())?;
    config_lock_file.unlock().map_err(|source| {
        format!(
            "Failed to unlock config file '{}': {}",
            config_path.display(),
            source
        )
    })?;
    drop(config_lock_file);

    let (updated, result) = updater(&existing)?;
    write_atomic(&config_path, &updated).map_err(|error| error.to_string())?;
    notify_config_changed();
    process_lock_file.unlock().map_err(|source| {
        format!(
            "Failed to unlock config lock file '{}': {}",
            lock_path_display, source
        )
    })?;
    Ok(result)
}

fn upsert_root_assignment(contents: &str, key: &str, value: &str) -> String {
    let mut new_config = String::new();
    let mut replaced = false;
    let mut inserted_before_first_section = false;
    let mut in_root_section = true;

    for line in contents.lines() {
        let trimmed = line.trim();
        let is_section_header = trimmed.starts_with('[') && trimmed.ends_with(']');

        if is_section_header {
            if !replaced && !inserted_before_first_section {
                new_config.push_str(&format!("{} = {}\n", key, value));
                inserted_before_first_section = true;
                replaced = true;
            }
            in_root_section = false;
            new_config.push_str(line);
            new_config.push('\n');
            continue;
        }

        if in_root_section {
            let mut parts = trimmed.splitn(2, '=');
            let line_key = parts.next().unwrap_or("").trim();
            if line_key.eq_ignore_ascii_case(key) {
                if !replaced {
                    new_config.push_str(&format!("{} = {}\n", key, value));
                    replaced = true;
                }
                continue;
            }
        }

        new_config.push_str(line);
        new_config.push('\n');
    }

    if !replaced && !inserted_before_first_section {
        if !new_config.is_empty() && !new_config.ends_with('\n') {
            new_config.push('\n');
        }
        new_config.push_str(&format!("{} = {}\n", key, value));
    }

    new_config
}

fn append_section(out: &mut String, section_name: &str, section_lines: &[String]) {
    out.push_str(&format!("[{}]\n", section_name));
    for section_line in section_lines {
        out.push_str(section_line);
        out.push('\n');
    }
}

fn replace_or_insert_section(
    contents: &str,
    section_name: &str,
    section_lines: &[String],
) -> String {
    let mut new_config = String::new();
    let mut in_target_section = false;
    let mut target_section_inserted = false;
    let target_header = format!("[{}]", section_name);

    for line in contents.lines() {
        let trimmed = line.trim();
        let is_section_header = trimmed.starts_with('[') && trimmed.ends_with(']');
        if is_section_header {
            in_target_section = false;
            if trimmed.eq_ignore_ascii_case(&target_header) {
                if !target_section_inserted {
                    append_section(&mut new_config, section_name, section_lines);
                    target_section_inserted = true;
                }
                in_target_section = true;
                continue;
            }
        }

        if in_target_section {
            continue;
        }

        new_config.push_str(line);
        new_config.push('\n');
    }

    if !target_section_inserted {
        if !new_config.is_empty() {
            new_config.push('\n');
        }
        append_section(&mut new_config, section_name, section_lines);
    }

    new_config
}

pub fn import_colors_from_json(json_path: &Path) -> Result<String, String> {
    let contents =
        fs::read_to_string(json_path).map_err(|e| format!("Failed to read file: {}", e))?;

    let json: serde_json::Value =
        serde_json::from_str(&contents).map_err(|e| format!("Invalid JSON: {}", e))?;

    let colors = json
        .as_object()
        .ok_or_else(|| "JSON must be an object".to_string())?;

    let mut color_lines = Vec::new();
    let mut color_line_indices: HashMap<&'static str, usize> = HashMap::new();

    for (key, value) in colors {
        if key.starts_with('$') {
            continue;
        }

        let Some(config_key) = canonical_color_key(key) else {
            continue;
        };

        let hex = value
            .as_str()
            .ok_or_else(|| format!("Color '{}' must be a hex string", key))?;

        if Rgb8::from_hex(hex).is_none() {
            return Err(format!("Invalid hex color for '{}': {}", key, hex));
        }

        let is_canonical_key = key.eq_ignore_ascii_case(config_key);
        if let Some(existing_index) = color_line_indices.get(config_key).copied() {
            if is_canonical_key {
                color_lines[existing_index] = format!("{} = {}", config_key, hex);
            }
            continue;
        }

        color_line_indices.insert(config_key, color_lines.len());
        color_lines.push(format!("{} = {}", config_key, hex));
    }

    if color_lines.is_empty() {
        return Err("No valid colors found in JSON".to_string());
    }

    let color_count = color_lines.len();
    update_config_contents(|existing| {
        Ok((
            replace_or_insert_section(existing, "colors", &color_lines),
            (),
        ))
    })?;
    Ok(format!("Imported {} colors", color_count))
}

pub fn set_theme_in_config(theme_id: &str) -> Result<String, String> {
    let theme = parse_theme_id(theme_id).ok_or_else(|| "Invalid theme id".to_string())?;
    update_config_contents(|existing| {
        Ok((
            upsert_root_assignment(existing, "theme", &theme),
            format!("Theme set to {}", theme),
        ))
    })
}

pub fn set_config_value(key: &str, value: &str) -> Result<(), String> {
    update_config_contents(|existing| Ok((upsert_root_assignment(existing, key, value), ())))
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use std::sync::{LazyLock, Mutex};

    use super::{import_colors_from_json, replace_or_insert_section, upsert_root_assignment};

    static ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    struct XdgConfigHomeGuard {
        previous_xdg: Option<OsString>,
    }

    impl XdgConfigHomeGuard {
        fn set(xdg_home: &Path) -> Self {
            let previous_xdg = std::env::var_os("XDG_CONFIG_HOME");
            unsafe { std::env::set_var("XDG_CONFIG_HOME", xdg_home) };
            Self { previous_xdg }
        }
    }

    impl Drop for XdgConfigHomeGuard {
        fn drop(&mut self) {
            if let Some(previous) = self.previous_xdg.take() {
                unsafe { std::env::set_var("XDG_CONFIG_HOME", previous) };
            } else {
                unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
            }
        }
    }

    fn with_temp_xdg_config_home(test: impl FnOnce(&Path)) {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let xdg_home = temp_dir.path().join("xdg");
        std::fs::create_dir_all(&xdg_home).expect("create xdg home");

        let _restore_guard = XdgConfigHomeGuard::set(&xdg_home);
        test(temp_dir.path());
    }

    #[test]
    fn with_temp_xdg_config_home_restores_environment_after_panic() {
        let before = std::env::var_os("XDG_CONFIG_HOME");
        let result = std::panic::catch_unwind(|| {
            with_temp_xdg_config_home(|_| panic!("intentional panic"));
        });
        assert!(result.is_err());
        assert_eq!(std::env::var_os("XDG_CONFIG_HOME"), before);
    }

    #[test]
    fn upsert_root_assignment_replaces_existing_root_value() {
        let input = "theme = termy\nfont_size = 14\n";
        let output = upsert_root_assignment(input, "theme", "nord");
        assert_eq!(output, "theme = nord\nfont_size = 14\n");
    }

    #[test]
    fn upsert_root_assignment_inserts_before_first_section_when_missing() {
        let input = "font_size = 14\n\n[colors]\nforeground = #ffffff\n";
        let output = upsert_root_assignment(input, "theme", "tokyo-night");
        assert_eq!(
            output,
            "font_size = 14\n\ntheme = tokyo-night\n[colors]\nforeground = #ffffff\n"
        );
    }

    #[test]
    fn upsert_root_assignment_handles_missing_trailing_newline() {
        let input = "font_size = 14";
        let output = upsert_root_assignment(input, "theme", "nord");
        assert_eq!(output, "font_size = 14\ntheme = nord\n");
    }

    #[test]
    fn upsert_root_assignment_preserves_leading_comments() {
        let input = "# default config\n# keep this\n\n[colors]\nforeground = #ffffff\n";
        let output = upsert_root_assignment(input, "theme", "tokyo-night");
        assert_eq!(
            output,
            "# default config\n# keep this\n\ntheme = tokyo-night\n[colors]\nforeground = #ffffff\n"
        );
    }

    #[test]
    fn upsert_root_assignment_collapses_duplicate_root_keys() {
        let input = "font_size = 12\nfont_size = 13\n[colors]\nforeground = #ffffff\n";
        let output = upsert_root_assignment(input, "font_size", "14");
        assert_eq!(output, "font_size = 14\n[colors]\nforeground = #ffffff\n");
    }

    #[test]
    fn replace_or_insert_section_replaces_existing_section_body() {
        let input = "theme = termy\n[colors]\nforeground = #ffffff\nbackground = #000000\n";
        let output = replace_or_insert_section(
            input,
            "colors",
            &[
                "foreground = #111111".to_string(),
                "cursor = #222222".to_string(),
            ],
        );

        assert_eq!(
            output,
            "theme = termy\n[colors]\nforeground = #111111\ncursor = #222222\n"
        );
    }

    #[test]
    fn replace_or_insert_section_matches_header_case_insensitively() {
        let input = "theme = termy\n[Colors]\nforeground = #ffffff\n";
        let output =
            replace_or_insert_section(input, "colors", &["foreground = #111111".to_string()]);
        assert_eq!(output, "theme = termy\n[colors]\nforeground = #111111\n");
    }

    #[test]
    fn replace_or_insert_section_collapses_duplicate_sections() {
        let input = "theme = termy\n[colors]\nforeground = #ffffff\n[other]\nvalue = 1\n[colors]\nbackground = #000000\n";
        let output = replace_or_insert_section(
            input,
            "colors",
            &[
                "foreground = #111111".to_string(),
                "background = #222222".to_string(),
            ],
        );
        assert_eq!(
            output,
            "theme = termy\n[colors]\nforeground = #111111\nbackground = #222222\n[other]\nvalue = 1\n"
        );
    }

    #[test]
    fn replace_or_insert_section_appends_missing_section() {
        let input = "theme = termy\nfont_size = 14\n";
        let output =
            replace_or_insert_section(input, "colors", &["foreground = #111111".to_string()]);

        assert_eq!(
            output,
            "theme = termy\nfont_size = 14\n\n[colors]\nforeground = #111111\n"
        );
    }

    #[test]
    fn import_colors_ignores_unknown_metadata_keys() {
        with_temp_xdg_config_home(|temp_path| {
            let json_path = temp_path.join("colors.json");
            std::fs::write(
                &json_path,
                "{\n  \"$schema\": \"https://example.com/theme.schema.json\",\n  \"metadata\": {\"name\": \"demo\"},\n  \"foreground\": \"#112233\"\n}\n",
            )
            .expect("write colors json");

            let result = import_colors_from_json(&json_path).expect("import colors");
            assert_eq!(result, "Imported 1 colors");

            let config_path = termy_config_core::config_path().expect("config path");
            let contents = std::fs::read_to_string(config_path).expect("read config");
            assert!(contents.contains("[colors]\nforeground = #112233\n"));
        });
    }

    #[test]
    fn import_colors_alias_collision_prefers_canonical_key_value() {
        with_temp_xdg_config_home(|temp_path| {
            let json_path = temp_path.join("colors.json");
            std::fs::write(
                &json_path,
                "{\n  \"fg\": \"#111111\",\n  \"foreground\": \"#222222\"\n}\n",
            )
            .expect("write colors json");

            let result = import_colors_from_json(&json_path).expect("import colors");
            assert_eq!(result, "Imported 1 colors");

            let config_path = termy_config_core::config_path().expect("config path");
            let contents = std::fs::read_to_string(config_path).expect("read config");
            let foreground_lines: Vec<&str> = contents
                .lines()
                .filter(|line| line.trim_start().starts_with("foreground ="))
                .collect();
            assert_eq!(foreground_lines, vec!["foreground = #222222"]);
        });
    }
}
