use crate::config;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

const DEFAULT_THEME_STORE_API_URL: &str = "https://api.termy.run";
const DEFAULT_THEME_DEEPLINK_API_URL: &str = "https://termy.run/theme-api";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ThemeStoreTheme {
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) description: String,
    pub(crate) latest_version: Option<String>,
    pub(crate) file_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InstalledTheme {
    pub(crate) slug: String,
    pub(crate) version: String,
    pub(crate) message: String,
}

pub(crate) fn theme_store_api_base_url() -> String {
    std::env::var("THEME_STORE_API_URL").unwrap_or_else(|_| DEFAULT_THEME_STORE_API_URL.into())
}

pub(crate) fn fetch_theme_store_themes_blocking(
    api_base: &str,
) -> Result<Vec<ThemeStoreTheme>, String> {
    let base = api_base.trim_end_matches('/');
    let url = format!("{base}/themes");
    let response = ureq::get(&url)
        .set("Accept", "application/json")
        .call()
        .map_err(|error| format!("Failed to fetch store themes: {error}"))?;

    let payload: serde_json::Value = response
        .into_json()
        .map_err(|error| format!("Invalid theme store response: {error}"))?;

    let themes = payload
        .as_array()
        .ok_or_else(|| "Theme store response must be a JSON array".to_string())?;

    let mut parsed = Vec::with_capacity(themes.len());
    for theme in themes {
        if let Some(parsed_theme) = parse_theme_value(theme) {
            parsed.push(parsed_theme);
        }
    }

    parsed.sort_unstable_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
    });
    Ok(parsed)
}

pub(crate) fn fetch_theme_for_deeplink_blocking(slug: &str) -> Result<ThemeStoreTheme, String> {
    let slug = normalize_slug(slug)?;
    let base = std::env::var("THEME_STORE_DEEPLINK_API_URL")
        .unwrap_or_else(|_| DEFAULT_THEME_DEEPLINK_API_URL.into());
    let url = format!("{}/themes/{}", base.trim_end_matches('/'), slug);
    let response = ureq::get(&url)
        .set("Accept", "application/json")
        .call()
        .map_err(|error| format!("Failed to fetch theme '{slug}': {error}"))?;

    let payload: serde_json::Value = response
        .into_json()
        .map_err(|error| format!("Invalid theme response for '{slug}': {error}"))?;

    parse_theme_value(&payload)
        .ok_or_else(|| format!("Theme response for '{slug}' is missing required fields"))
}

pub(crate) fn load_installed_theme_versions() -> HashMap<String, String> {
    let Some(path) = installed_theme_state_path() else {
        return HashMap::new();
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return HashMap::new();
    };

    if let Ok(parsed_map) = serde_json::from_str::<HashMap<String, String>>(&contents) {
        return parsed_map
            .into_iter()
            .map(|(slug, version)| (slug.trim().to_ascii_lowercase(), version.trim().to_string()))
            .filter(|(slug, _)| !slug.is_empty())
            .collect();
    }

    if let Ok(parsed_list) = serde_json::from_str::<Vec<String>>(&contents) {
        return parsed_list
            .into_iter()
            .map(|slug| (slug.trim().to_ascii_lowercase(), String::new()))
            .filter(|(slug, _)| !slug.is_empty())
            .collect();
    }

    HashMap::new()
}

pub(crate) fn persist_installed_theme_versions(
    versions: &HashMap<String, String>,
) -> Result<(), String> {
    let Some(path) = installed_theme_state_path() else {
        return Err("Config path unavailable".to_string());
    };
    let Some(parent) = path.parent() else {
        return Err("Invalid installed-theme metadata path".to_string());
    };
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create metadata directory: {error}"))?;

    let mut sorted_entries: Vec<(String, String)> = versions
        .iter()
        .map(|(slug, version)| (slug.clone(), version.clone()))
        .collect();
    sorted_entries.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    let normalized: HashMap<String, String> = sorted_entries.into_iter().collect();
    let contents = serde_json::to_string_pretty(&normalized)
        .map_err(|error| format!("Failed to serialize installed themes: {error}"))?;
    std::fs::write(&path, contents)
        .map_err(|error| format!("Failed to write installed themes metadata: {error}"))?;
    Ok(())
}

pub(crate) fn install_theme_from_store_blocking(
    theme: ThemeStoreTheme,
) -> Result<InstalledTheme, String> {
    let file_url = theme
        .file_url
        .clone()
        .ok_or_else(|| format!("Theme '{}' has no downloadable file URL", theme.slug))?;

    let response = ureq::get(&file_url)
        .set("Accept", "application/json")
        .call()
        .map_err(|error| format!("Failed to download theme '{}': {error}", theme.slug))?;
    let contents = response
        .into_string()
        .map_err(|error| format!("Failed to read theme '{}': {error}", theme.slug))?;

    let mut file =
        tempfile::NamedTempFile::new().map_err(|error| format!("Temp file error: {error}"))?;
    file.write_all(contents.as_bytes())
        .map_err(|error| format!("Failed to write temp theme file: {error}"))?;

    config::import_colors_from_json(file.path())
        .map_err(|error| format!("Failed to install theme '{}': {error}", theme.name))?;

    let mut installed_versions = HashMap::new();
    let normalized_slug = theme.slug.trim().to_ascii_lowercase();
    let installed_version = theme.latest_version.clone().unwrap_or_default();
    installed_versions.insert(normalized_slug.clone(), installed_version.clone());
    persist_installed_theme_versions(&installed_versions)?;

    Ok(InstalledTheme {
        slug: normalized_slug,
        version: installed_version,
        message: format!("Installed theme '{}'", theme.name),
    })
}

fn installed_theme_state_path() -> Option<PathBuf> {
    let config_path = config::ensure_config_file().ok()?;
    let parent = config_path.parent()?;
    Some(parent.join("theme_store_installed.json"))
}

fn normalize_slug(slug: &str) -> Result<String, String> {
    let slug = slug.trim().to_ascii_lowercase();
    if slug.is_empty() {
        return Err("Theme install deeplink is missing a slug".to_string());
    }
    if !slug.chars().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
    }) {
        return Err(format!("Invalid theme slug '{slug}'"));
    }
    Ok(slug)
}

fn parse_theme_value(theme: &serde_json::Value) -> Option<ThemeStoreTheme> {
    let object = theme.as_object()?;
    let name = object
        .get("name")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    let slug = object
        .get("slug")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())?;

    let description = object
        .get("description")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    let latest_version = object
        .get("latestVersion")
        .and_then(|value| value.as_str())
        .map(ToString::to_string);
    let file_url = object
        .get("fileUrl")
        .and_then(|value| value.as_str())
        .map(ToString::to_string);

    Some(ThemeStoreTheme {
        name: name.to_string(),
        slug: slug.to_string(),
        description,
        latest_version,
        file_url,
    })
}
