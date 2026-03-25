use super::*;
use alacritty_terminal::grid::Dimensions;
use gpui::{ObjectFit, StatefulInteractiveElement, StyledImage, img};
use libsqlite3_sys as sqlite3;
use serde::{Deserialize, Serialize};
use std::ffi::{CStr, CString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const AGENT_WORKSPACE_DB_FILE: &str = "agents.sqlite3";
const LEGACY_AGENT_WORKSPACE_STATE_FILE: &str = "agents.json";
const AGENT_WORKSPACE_SCHEMA_VERSION: u64 = 1;
const LEGACY_AGENT_WORKSPACE_STATE_VERSION: u64 = 1;
const AGENT_WORKSPACE_STATE_ROW_KEY: &str = "state";
const AGENT_SIDEBAR_MIN_WIDTH: f32 = 180.0;
const AGENT_SIDEBAR_MAX_WIDTH: f32 = 500.0;
const AGENT_SIDEBAR_HEADER_HEIGHT: f32 = 30.0;
const AGENT_SIDEBAR_SEARCH_HEIGHT: f32 = 28.0;
const AGENT_SIDEBAR_PROJECT_ROW_HEIGHT: f32 = 24.0;
const AGENT_GIT_PANEL_WIDTH: f32 = 320.0;
const AGENT_STATUS_VISIBLE_LINE_COUNT: i32 = 6;
static NEXT_AGENT_ENTITY_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum AgentSidebarFilter {
    #[default]
    All,
    Live,
    Saved,
    Busy,
    Pinned,
}

impl AgentSidebarFilter {
    const ALL: [Self; 5] = [Self::All, Self::Live, Self::Saved, Self::Busy, Self::Pinned];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Live => "Live",
            Self::Saved => "Saved",
            Self::Busy => "Busy",
            Self::Pinned => "Pinned",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AgentGitPanelState {
    pub(super) open: bool,
    source_path: Option<String>,
    label: Option<String>,
    repo_root: Option<String>,
    branch: Option<String>,
    current_branch: Option<String>,
    ahead: usize,
    behind: usize,
    dirty_count: usize,
    last_commit: Option<String>,
    loading: bool,
    error: Option<String>,
    filter: AgentGitPanelFilter,
    entries: Vec<AgentGitPanelEntry>,
    selected_repo_path: Option<String>,
    preview_loading: bool,
    preview_error: Option<String>,
    preview_diff_lines: Vec<String>,
    preview_history: Vec<AgentGitHistoryEntry>,
    project_history: Vec<AgentGitHistoryEntry>,
    branches: Vec<String>,
    stashes: Vec<AgentGitStashEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AgentGitPanelEntry {
    pub(super) status: String,
    pub(super) path: String,
    repo_path: String,
}

struct AgentGitPanelSnapshot {
    repo_root: String,
    branch: Option<String>,
    current_branch: Option<String>,
    ahead: usize,
    behind: usize,
    dirty_count: usize,
    last_commit: Option<String>,
    entries: Vec<AgentGitPanelEntry>,
    project_history: Vec<AgentGitHistoryEntry>,
    branches: Vec<String>,
    stashes: Vec<AgentGitStashEntry>,
}

struct AgentGitPanelPreviewSnapshot {
    diff_lines: Vec<String>,
    history: Vec<AgentGitHistoryEntry>,
}

impl AgentGitPanelEntry {
    fn from_status_line(status: &str, raw_path: &str) -> Self {
        let repo_path = raw_path
            .split(" -> ")
            .last()
            .unwrap_or(raw_path)
            .to_string();
        Self {
            status: status.to_string(),
            path: raw_path.to_string(),
            repo_path,
        }
    }

    fn is_untracked(&self) -> bool {
        self.status == "??"
    }

    fn is_staged(&self) -> bool {
        self.status
            .chars()
            .next()
            .is_some_and(|ch| ch != ' ' && ch != '?')
    }

    fn is_unstaged(&self) -> bool {
        self.status
            .chars()
            .nth(1)
            .is_some_and(|ch| ch != ' ' && ch != '?')
    }

    fn is_deleted(&self) -> bool {
        self.status.contains('D')
    }

    fn badge_label(&self) -> SharedString {
        if self.is_untracked() {
            return "new".into();
        }
        if self.status.contains('R') {
            return "ren".into();
        }
        if self.status.contains('A') {
            return "add".into();
        }
        if self.status.contains('D') {
            return "del".into();
        }
        if self.status.contains('U') {
            return "conf".into();
        }
        if self.status.contains('M') {
            return "mod".into();
        }
        self.status.trim().to_lowercase().into()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum AgentGitPanelFilter {
    #[default]
    All,
    Staged,
    Unstaged,
    Untracked,
}

impl AgentGitPanelFilter {
    const ALL: [Self; 4] = [Self::All, Self::Staged, Self::Unstaged, Self::Untracked];

    fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Staged => "Staged",
            Self::Unstaged => "Unstaged",
            Self::Untracked => "Untracked",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AgentGitPanelInputMode {
    Commit,
    CreateBranch,
    SaveStash,
}

impl AgentGitPanelInputMode {
    fn title(self) -> &'static str {
        match self {
            Self::Commit => "Commit message",
            Self::CreateBranch => "New branch",
            Self::SaveStash => "Stash message",
        }
    }

    fn placeholder(self) -> &'static str {
        match self {
            Self::Commit => "Write a commit message",
            Self::CreateBranch => "feature/my-branch",
            Self::SaveStash => "WIP stash",
        }
    }

    fn action_label(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::CreateBranch => "create",
            Self::SaveStash => "save",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AgentGitHistoryEntry {
    summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AgentGitStashEntry {
    name: String,
    summary: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AgentThreadRuntimeStatus {
    Busy,
    Ready,
    Saved,
}

impl AgentThreadRuntimeStatus {
    fn label(self) -> &'static str {
        match self {
            Self::Busy => "busy",
            Self::Ready => "ready",
            Self::Saved => "saved",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AgentThreadStatusTone {
    Active,
    Warning,
    Error,
    Muted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AgentThreadStatusPresentation {
    label: String,
    detail: Option<String>,
    tone: AgentThreadStatusTone,
}

struct AgentSidebarTooltip {
    title: &'static str,
    detail: &'static str,
    bg: gpui::Rgba,
    border: gpui::Rgba,
    text: gpui::Rgba,
    muted: gpui::Rgba,
}

impl AgentSidebarTooltip {
    fn new(
        title: &'static str,
        detail: &'static str,
        bg: gpui::Rgba,
        border: gpui::Rgba,
        text: gpui::Rgba,
        muted: gpui::Rgba,
    ) -> Self {
        Self {
            title,
            detail,
            bg,
            border,
            text,
            muted,
        }
    }
}

impl Render for AgentSidebarTooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().pl(px(8.0)).pt(px(10.0)).child(
            div()
                .w(px(196.0))
                .px(px(10.0))
                .py(px(8.0))
                .flex()
                .flex_col()
                .gap(px(4.0))
                .bg(self.bg)
                .border_1()
                .border_color(self.border)
                .child(
                    div()
                        .w_full()
                        .whitespace_normal()
                        .text_size(px(11.0))
                        .text_color(self.text)
                        .child(self.title),
                )
                .child(
                    div()
                        .w_full()
                        .whitespace_normal()
                        .text_size(px(10.0))
                        .text_color(self.muted)
                        .child(self.detail),
                ),
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct PersistedAgentWorkspaceState {
    version: u64,
    sidebar_open: bool,
    active_project_id: Option<String>,
    #[serde(default)]
    collapsed_project_ids: Vec<String>,
    projects: Vec<AgentProject>,
    threads: Vec<AgentThread>,
}

impl Default for PersistedAgentWorkspaceState {
    fn default() -> Self {
        Self {
            version: AGENT_WORKSPACE_SCHEMA_VERSION,
            sidebar_open: false,
            active_project_id: None,
            collapsed_project_ids: Vec::new(),
            projects: Vec::new(),
            threads: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct AgentProject {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) root_path: String,
    #[serde(default)]
    pub(super) pinned: bool,
    pub(super) created_at_ms: u64,
    pub(super) updated_at_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct AgentThread {
    id: String,
    project_id: String,
    agent: command_palette::AiAgentPreset,
    title: String,
    #[serde(default)]
    custom_title: Option<String>,
    #[serde(default)]
    pinned: bool,
    launch_command: String,
    working_dir: String,
    last_seen_title: Option<String>,
    last_seen_command: Option<String>,
    #[serde(default)]
    last_status_label: Option<String>,
    #[serde(default)]
    last_status_detail: Option<String>,
    created_at_ms: u64,
    updated_at_ms: u64,
    #[serde(skip)]
    linked_tab_id: Option<TabId>,
}

fn next_agent_entity_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let counter = NEXT_AGENT_ENTITY_ID.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{millis}-{counter}")
}

fn now_unix_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}

struct AgentWorkspaceDb {
    raw: *mut sqlite3::sqlite3,
}

struct AgentWorkspaceStatement<'db> {
    db: &'db AgentWorkspaceDb,
    raw: *mut sqlite3::sqlite3_stmt,
}

impl AgentWorkspaceDb {
    fn open(path: &Path) -> Result<Self, String> {
        let parent = path
            .parent()
            .ok_or_else(|| format!("Invalid agent workspace path '{}'", path.display()))?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create '{}': {}", parent.display(), error))?;

        let path_string = path.to_string_lossy().into_owned();
        let path_cstr =
            CString::new(path_string.clone()).map_err(|_| "Invalid SQLite path".to_string())?;

        let mut raw = ptr::null_mut();
        let open_status = unsafe {
            sqlite3::sqlite3_open_v2(
                path_cstr.as_ptr(),
                &mut raw,
                sqlite3::SQLITE_OPEN_READWRITE | sqlite3::SQLITE_OPEN_CREATE,
                ptr::null(),
            )
        };
        if open_status != sqlite3::SQLITE_OK {
            let error = sqlite_error_message(raw);
            if !raw.is_null() {
                unsafe {
                    sqlite3::sqlite3_close(raw);
                }
            }
            return Err(format!(
                "Failed to open agent workspace database '{}': {}",
                path.display(),
                error
            ));
        }

        let db = Self { raw };
        db.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS agent_workspace_meta (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );
            ",
        )?;
        Ok(db)
    }

    fn execute_batch(&self, sql: &str) -> Result<(), String> {
        let sql_cstr = CString::new(sql)
            .map_err(|_| "SQLite statement contains an embedded NUL byte".to_string())?;
        let mut error_message = ptr::null_mut();
        let status = unsafe {
            sqlite3::sqlite3_exec(
                self.raw,
                sql_cstr.as_ptr(),
                None,
                ptr::null_mut(),
                &mut error_message,
            )
        };
        if status == sqlite3::SQLITE_OK {
            return Ok(());
        }

        Err(format!(
            "SQLite statement failed: {}",
            sqlite_exec_error_message(error_message)
        ))
    }

    fn prepare(&self, sql: &str) -> Result<AgentWorkspaceStatement<'_>, String> {
        let sql_cstr = CString::new(sql)
            .map_err(|_| "SQLite statement contains an embedded NUL byte".to_string())?;
        let mut statement = ptr::null_mut();
        let status = unsafe {
            sqlite3::sqlite3_prepare_v2(
                self.raw,
                sql_cstr.as_ptr(),
                -1,
                &mut statement,
                ptr::null_mut(),
            )
        };
        if status != sqlite3::SQLITE_OK {
            return Err(format!(
                "Failed to prepare SQLite statement: {}",
                sqlite_error_message(self.raw)
            ));
        }

        Ok(AgentWorkspaceStatement {
            db: self,
            raw: statement,
        })
    }

    fn meta_value(&self, key: &str) -> Result<Option<String>, String> {
        let mut statement =
            self.prepare("SELECT value FROM agent_workspace_meta WHERE key = ?1 LIMIT 1")?;
        statement.bind_text(1, key)?;
        match statement.step()? {
            sqlite3::SQLITE_ROW => statement.column_text(0).map(Some),
            sqlite3::SQLITE_DONE => Ok(None),
            _ => unreachable!(),
        }
    }

    fn set_meta_value(&self, key: &str, value: &str) -> Result<(), String> {
        let mut statement = self.prepare(
            "
            INSERT INTO agent_workspace_meta (key, value) VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            ",
        )?;
        statement.bind_text(1, key)?;
        statement.bind_text(2, value)?;
        statement.step_done()
    }
}

impl Drop for AgentWorkspaceDb {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                sqlite3::sqlite3_close(self.raw);
            }
        }
    }
}

impl AgentWorkspaceStatement<'_> {
    fn bind_text(&mut self, index: i32, value: &str) -> Result<(), String> {
        let value_cstr = CString::new(value)
            .map_err(|_| "SQLite bind value contains an embedded NUL byte".to_string())?;
        let status = unsafe {
            sqlite3::sqlite3_bind_text(
                self.raw,
                index,
                value_cstr.as_ptr(),
                -1,
                sqlite3::SQLITE_TRANSIENT(),
            )
        };
        if status == sqlite3::SQLITE_OK {
            Ok(())
        } else {
            Err(format!(
                "Failed to bind SQLite text parameter: {}",
                sqlite_error_message(self.db.raw)
            ))
        }
    }

    fn step(&mut self) -> Result<i32, String> {
        let status = unsafe { sqlite3::sqlite3_step(self.raw) };
        match status {
            sqlite3::SQLITE_ROW | sqlite3::SQLITE_DONE => Ok(status),
            _ => Err(format!(
                "SQLite statement execution failed: {}",
                sqlite_error_message(self.db.raw)
            )),
        }
    }

    fn step_done(&mut self) -> Result<(), String> {
        match self.step()? {
            sqlite3::SQLITE_DONE => Ok(()),
            sqlite3::SQLITE_ROW => Err("SQLite statement unexpectedly returned a row".to_string()),
            _ => unreachable!(),
        }
    }

    fn column_text(&self, index: i32) -> Result<String, String> {
        let value = unsafe { sqlite3::sqlite3_column_text(self.raw, index) };
        if value.is_null() {
            return Err("SQLite column was NULL when text was expected".to_string());
        }

        Ok(unsafe { CStr::from_ptr(value.cast()) }
            .to_string_lossy()
            .into_owned())
    }
}

impl Drop for AgentWorkspaceStatement<'_> {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                sqlite3::sqlite3_finalize(self.raw);
            }
        }
    }
}

fn sqlite_error_message(raw: *mut sqlite3::sqlite3) -> String {
    if raw.is_null() {
        return "Unknown SQLite error".to_string();
    }

    unsafe { CStr::from_ptr(sqlite3::sqlite3_errmsg(raw)) }
        .to_string_lossy()
        .into_owned()
}

fn sqlite_exec_error_message(error_message: *mut i8) -> String {
    if error_message.is_null() {
        return "Unknown SQLite error".to_string();
    }

    let message = unsafe { CStr::from_ptr(error_message) }
        .to_string_lossy()
        .into_owned();
    unsafe {
        sqlite3::sqlite3_free(error_message.cast());
    }
    message
}

fn load_legacy_agent_workspace_state(path: &Path) -> Result<PersistedAgentWorkspaceState, String> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(PersistedAgentWorkspaceState::default());
        }
        Err(error) => {
            return Err(format!(
                "Failed to read legacy agent workspace state '{}': {}",
                path.display(),
                error
            ));
        }
    };

    let mut state: PersistedAgentWorkspaceState = serde_json::from_str(&contents)
        .map_err(|error| format!("Invalid legacy agent workspace JSON: {}", error))?;
    if state.version != LEGACY_AGENT_WORKSPACE_STATE_VERSION {
        return Err(format!(
            "Unsupported legacy agent workspace state version {}",
            state.version
        ));
    }
    for thread in &mut state.threads {
        thread.linked_tab_id = None;
    }
    Ok(state)
}

fn decode_agent_workspace_state(contents: &str) -> Result<PersistedAgentWorkspaceState, String> {
    let mut state: PersistedAgentWorkspaceState = serde_json::from_str(contents)
        .map_err(|error| format!("Invalid agent workspace database payload: {}", error))?;
    if state.version != AGENT_WORKSPACE_SCHEMA_VERSION {
        return Err(format!(
            "Unsupported agent workspace schema version {}",
            state.version
        ));
    }
    for thread in &mut state.threads {
        thread.linked_tab_id = None;
    }
    Ok(state)
}

fn load_agent_workspace_state_from_db(
    db: &AgentWorkspaceDb,
) -> Result<Option<PersistedAgentWorkspaceState>, String> {
    db.meta_value(AGENT_WORKSPACE_STATE_ROW_KEY)?
        .map(|contents| decode_agent_workspace_state(&contents))
        .transpose()
}

fn store_agent_workspace_state_to_db(
    db: &AgentWorkspaceDb,
    state: &PersistedAgentWorkspaceState,
) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(state)
        .map_err(|error| format!("Failed to encode agent workspace state: {}", error))?;
    db.set_meta_value(AGENT_WORKSPACE_STATE_ROW_KEY, &contents)
}

fn load_or_migrate_agent_workspace_state(
    db: &AgentWorkspaceDb,
    legacy_path: &Path,
) -> Result<PersistedAgentWorkspaceState, String> {
    if let Some(state) = load_agent_workspace_state_from_db(db)? {
        return Ok(state);
    }

    if legacy_path.exists() {
        let state = load_legacy_agent_workspace_state(legacy_path)?;
        store_agent_workspace_state_to_db(db, &state)?;
        return Ok(state);
    }

    Ok(PersistedAgentWorkspaceState::default())
}

pub(super) fn clamp_agent_sidebar_width(width: f32) -> f32 {
    width.clamp(AGENT_SIDEBAR_MIN_WIDTH, AGENT_SIDEBAR_MAX_WIDTH)
}

impl TerminalView {
    pub(in super::super) fn agent_sidebar_width(&self) -> f32 {
        if self.should_render_agent_sidebar() {
            self.agent_sidebar_width
        } else {
            0.0
        }
    }

    pub(in super::super) fn terminal_left_sidebar_width(&self) -> f32 {
        self.tab_strip_sidebar_width() + self.agent_sidebar_width()
    }

    pub(super) fn should_render_agent_sidebar(&self) -> bool {
        self.agent_sidebar_enabled && self.agent_sidebar_open
    }

    fn close_agent_git_panel(&mut self) {
        self.agent_git_panel = AgentGitPanelState::default();
        self.agent_git_panel_input_mode = None;
        self.agent_git_panel_input.clear();
    }

    pub(super) fn cancel_agent_git_panel_input(&mut self, cx: &mut Context<Self>) {
        if self.agent_git_panel_input_mode.take().is_some()
            || !self.agent_git_panel_input.text().is_empty()
        {
            self.agent_git_panel_input.clear();
            self.inline_input_selecting = false;
            cx.notify();
        }
    }

    fn begin_agent_git_panel_input(
        &mut self,
        mode: AgentGitPanelInputMode,
        initial_text: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.agent_git_panel_input_mode = Some(mode);
        self.agent_git_panel_input.set_text(initial_text.into());
        self.inline_input_selecting = true;
        cx.notify();
    }

    fn agent_git_panel_matches_target_path(&self, path: &str) -> bool {
        if !self.agent_git_panel.open {
            return false;
        }

        if let Some(repo_root) = self.agent_git_panel.repo_root.as_deref() {
            let repo_root = Path::new(repo_root);
            let target = Path::new(path);
            if target == repo_root || target.starts_with(repo_root) {
                return true;
            }
        }

        self.agent_git_panel.source_path.as_deref() == Some(path)
    }

    fn run_git_command(repo_root: &str, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .args(args)
            .output()
            .map_err(|error| {
                format!(
                    "Failed to run git {}: {}",
                    args.first().copied().unwrap_or("command"),
                    error
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                format!("git {} failed", args.first().copied().unwrap_or("command"))
            } else {
                stderr
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn run_git_diff_command(repo_root: &str, args: &[&str]) -> Result<String, String> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_root)
            .args(args)
            .output()
            .map_err(|error| format!("Failed to run git diff: {}", error))?;
        let code = output.status.code().unwrap_or_default();
        if !(output.status.success() || code == 1) {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(if stderr.is_empty() {
                "git diff failed".to_string()
            } else {
                stderr
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    fn parse_agent_git_branch_summary(branch_line: &str) -> (Option<String>, usize, usize) {
        let current_branch = branch_line
            .split("...")
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty() && *value != "HEAD (no branch)")
            .map(str::to_string);
        let mut ahead = 0usize;
        let mut behind = 0usize;
        if let Some(start) = branch_line.find('[')
            && let Some(end_rel) = branch_line[start + 1..].find(']')
        {
            let details = &branch_line[start + 1..start + 1 + end_rel];
            for part in details.split(',') {
                let trimmed = part.trim();
                if let Some(value) = trimmed.strip_prefix("ahead ") {
                    ahead = value.parse::<usize>().unwrap_or_default();
                }
                if let Some(value) = trimmed.strip_prefix("behind ") {
                    behind = value.parse::<usize>().unwrap_or_default();
                }
            }
        }
        (current_branch, ahead, behind)
    }

    fn parse_agent_git_history(output: &str) -> Vec<AgentGitHistoryEntry> {
        output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| AgentGitHistoryEntry {
                summary: line.to_string(),
            })
            .collect()
    }

    fn parse_agent_git_stashes(output: &str) -> Vec<AgentGitStashEntry> {
        output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let mut parts = line.splitn(2, ' ');
                AgentGitStashEntry {
                    name: parts.next().unwrap_or_default().to_string(),
                    summary: parts.next().unwrap_or_default().to_string(),
                }
            })
            .collect()
    }

    fn load_agent_git_panel_snapshot(path: &str) -> Result<AgentGitPanelSnapshot, String> {
        let repo_root = Self::run_git_command(path, &["rev-parse", "--show-toplevel"])?
            .trim()
            .to_string();
        let status_output = Self::run_git_command(
            repo_root.as_str(),
            &[
                "status",
                "--porcelain=v1",
                "--branch",
                "--untracked-files=all",
            ],
        )?;
        let mut branch = None;
        let mut current_branch = None;
        let mut ahead = 0usize;
        let mut behind = 0usize;
        let mut entries = Vec::new();
        for line in status_output.lines() {
            if let Some(branch_line) = line.strip_prefix("## ") {
                branch = Some(branch_line.to_string());
                let parsed = Self::parse_agent_git_branch_summary(branch_line);
                current_branch = parsed.0;
                ahead = parsed.1;
                behind = parsed.2;
                continue;
            }
            if line.len() < 3 {
                continue;
            }
            entries.push(AgentGitPanelEntry::from_status_line(&line[..2], &line[3..]));
        }

        let last_commit =
            Self::run_git_command(repo_root.as_str(), &["log", "-1", "--pretty=%h %s"])
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
        let project_history = Self::parse_agent_git_history(&Self::run_git_command(
            repo_root.as_str(),
            &["log", "-n", "8", "--pretty=%h %s"],
        )?);
        let branches = Self::run_git_command(
            repo_root.as_str(),
            &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
        )?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
        let stashes = Self::parse_agent_git_stashes(&Self::run_git_command(
            repo_root.as_str(),
            &["stash", "list", "--format=%gd %s"],
        )?);
        let dirty_count = entries.len();

        Ok(AgentGitPanelSnapshot {
            repo_root,
            branch,
            current_branch,
            ahead,
            behind,
            dirty_count,
            last_commit,
            entries,
            project_history,
            branches,
            stashes,
        })
    }

    fn load_agent_git_panel_preview(
        repo_root: &str,
        entry: &AgentGitPanelEntry,
    ) -> Result<AgentGitPanelPreviewSnapshot, String> {
        let diff = if entry.is_untracked() {
            let absolute = Path::new(repo_root).join(entry.repo_path.as_str());
            Self::run_git_diff_command(
                repo_root,
                &[
                    "diff",
                    "--no-index",
                    "--unified=3",
                    "--",
                    "/dev/null",
                    absolute.to_string_lossy().as_ref(),
                ],
            )?
        } else {
            Self::run_git_diff_command(
                repo_root,
                &[
                    "diff",
                    "--no-ext-diff",
                    "--unified=3",
                    "HEAD",
                    "--",
                    entry.repo_path.as_str(),
                ],
            )?
        };
        let history = Self::parse_agent_git_history(&Self::run_git_command(
            repo_root,
            &[
                "log",
                "-n",
                "8",
                "--pretty=%h %s",
                "--",
                entry.repo_path.as_str(),
            ],
        )?);
        Ok(AgentGitPanelPreviewSnapshot {
            diff_lines: diff.lines().map(str::to_string).collect(),
            history,
        })
    }

    fn refresh_agent_git_panel(&mut self, cx: &mut Context<Self>) {
        let (Some(source_path), Some(label)) = (
            self.agent_git_panel.source_path.clone(),
            self.agent_git_panel.label.clone(),
        ) else {
            return;
        };

        self.agent_git_panel.open = true;
        self.agent_git_panel.loading = true;
        self.agent_git_panel.error = None;
        cx.notify();

        let source_path_for_load = source_path.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result =
                smol::unblock(move || Self::load_agent_git_panel_snapshot(&source_path_for_load))
                    .await;
            let _ = cx.update(|cx| {
                this.update(cx, |view, cx| {
                    if !view.agent_git_panel.open
                        || view.agent_git_panel.source_path.as_deref() != Some(source_path.as_str())
                    {
                        return;
                    }

                    view.agent_git_panel.loading = false;
                    match result {
                        Ok(snapshot) => {
                            let selected = view.agent_git_panel.selected_repo_path.clone();
                            view.agent_git_panel.repo_root = Some(snapshot.repo_root);
                            view.agent_git_panel.branch = snapshot.branch;
                            view.agent_git_panel.current_branch = snapshot.current_branch;
                            view.agent_git_panel.ahead = snapshot.ahead;
                            view.agent_git_panel.behind = snapshot.behind;
                            view.agent_git_panel.dirty_count = snapshot.dirty_count;
                            view.agent_git_panel.last_commit = snapshot.last_commit;
                            view.agent_git_panel.project_history = snapshot.project_history;
                            view.agent_git_panel.branches = snapshot.branches;
                            view.agent_git_panel.stashes = snapshot.stashes;
                            view.agent_git_panel.error = None;
                            view.agent_git_panel.entries = snapshot.entries;
                            if let Some(selected_path) = selected {
                                if view
                                    .agent_git_panel
                                    .entries
                                    .iter()
                                    .any(|entry| entry.repo_path == selected_path)
                                {
                                    view.select_agent_git_panel_entry(selected_path.as_str(), cx);
                                } else {
                                    view.clear_agent_git_panel_preview();
                                }
                            }
                        }
                        Err(error) => {
                            view.agent_git_panel.repo_root = None;
                            view.agent_git_panel.branch = None;
                            view.agent_git_panel.current_branch = None;
                            view.agent_git_panel.ahead = 0;
                            view.agent_git_panel.behind = 0;
                            view.agent_git_panel.dirty_count = 0;
                            view.agent_git_panel.last_commit = None;
                            view.agent_git_panel.project_history.clear();
                            view.agent_git_panel.branches.clear();
                            view.agent_git_panel.stashes.clear();
                            view.agent_git_panel.entries.clear();
                            view.agent_git_panel.error = Some(error);
                            view.clear_agent_git_panel_preview();
                        }
                    }
                    cx.notify();
                })
            });
        })
        .detach();
        let _ = label;
    }

    fn clear_agent_git_panel_preview(&mut self) {
        self.agent_git_panel.selected_repo_path = None;
        self.agent_git_panel.preview_loading = false;
        self.agent_git_panel.preview_error = None;
        self.agent_git_panel.preview_diff_lines.clear();
        self.agent_git_panel.preview_history.clear();
    }

    fn select_agent_git_panel_entry(&mut self, repo_path: &str, cx: &mut Context<Self>) {
        let Some(repo_root) = self.agent_git_panel.repo_root.clone() else {
            return;
        };
        let Some(entry) = self
            .agent_git_panel
            .entries
            .iter()
            .find(|entry| entry.repo_path == repo_path)
            .cloned()
        else {
            return;
        };

        if self.agent_git_panel.selected_repo_path.as_deref() == Some(repo_path)
            && !self.agent_git_panel.preview_loading
        {
            self.clear_agent_git_panel_preview();
            cx.notify();
            return;
        }

        self.agent_git_panel.selected_repo_path = Some(repo_path.to_string());
        self.agent_git_panel.preview_loading = true;
        self.agent_git_panel.preview_error = None;
        self.agent_git_panel.preview_diff_lines.clear();
        self.agent_git_panel.preview_history.clear();
        cx.notify();

        let selected_repo_path = repo_path.to_string();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result =
                smol::unblock(move || Self::load_agent_git_panel_preview(&repo_root, &entry)).await;
            let _ = cx.update(|cx| {
                this.update(cx, |view, cx| {
                    if view.agent_git_panel.selected_repo_path.as_deref()
                        != Some(selected_repo_path.as_str())
                    {
                        return;
                    }
                    view.agent_git_panel.preview_loading = false;
                    match result {
                        Ok(preview) => {
                            view.agent_git_panel.preview_error = None;
                            view.agent_git_panel.preview_diff_lines = preview.diff_lines;
                            view.agent_git_panel.preview_history = preview.history;
                        }
                        Err(error) => {
                            view.agent_git_panel.preview_error = Some(error);
                            view.agent_git_panel.preview_diff_lines.clear();
                            view.agent_git_panel.preview_history.clear();
                        }
                    }
                    cx.notify();
                })
            });
        })
        .detach();
    }

    fn open_agent_git_panel_for_path(
        &mut self,
        source_path: &str,
        label: String,
        cx: &mut Context<Self>,
    ) {
        self.agent_git_panel.open = true;
        self.agent_git_panel.source_path = Some(source_path.to_string());
        self.agent_git_panel.label = Some(label);
        self.agent_git_panel.selected_repo_path = None;
        self.agent_git_panel.preview_loading = false;
        self.agent_git_panel.preview_error = None;
        self.agent_git_panel.preview_diff_lines.clear();
        self.agent_git_panel.preview_history.clear();
        self.refresh_agent_git_panel(cx);
    }

    fn toggle_agent_git_panel_for_path(
        &mut self,
        source_path: &str,
        label: String,
        cx: &mut Context<Self>,
    ) {
        if self.agent_git_panel_matches_target_path(source_path) {
            self.close_agent_git_panel();
            cx.notify();
            return;
        }
        self.open_agent_git_panel_for_path(source_path, label, cx);
    }

    fn toggle_agent_git_panel_for_project(&mut self, project_id: &str, cx: &mut Context<Self>) {
        let Some(project) = self
            .agent_projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| (project.root_path.clone(), project.name.clone()))
        else {
            return;
        };

        self.toggle_agent_git_panel_for_path(
            project.0.as_str(),
            format!("Project · {}", project.1),
            cx,
        );
    }

    fn toggle_agent_git_panel_for_thread(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        let Some((working_dir, title)) = self
            .agent_threads
            .iter()
            .find(|thread| thread.id == thread_id)
            .map(|thread| {
                (
                    thread.working_dir.clone(),
                    self.agent_thread_display_title(thread),
                )
            })
        else {
            return;
        };

        self.toggle_agent_git_panel_for_path(
            working_dir.as_str(),
            format!("Thread · {}", title),
            cx,
        );
    }

    fn agent_git_entries_for_filter(&self) -> Vec<AgentGitPanelEntry> {
        self.agent_git_panel
            .entries
            .iter()
            .filter(|entry| match self.agent_git_panel.filter {
                AgentGitPanelFilter::All => true,
                AgentGitPanelFilter::Staged => entry.is_staged(),
                AgentGitPanelFilter::Unstaged => entry.is_unstaged(),
                AgentGitPanelFilter::Untracked => entry.is_untracked(),
            })
            .cloned()
            .collect()
    }

    fn set_agent_git_panel_filter(&mut self, filter: AgentGitPanelFilter, cx: &mut Context<Self>) {
        if self.agent_git_panel.filter == filter {
            return;
        }
        self.agent_git_panel.filter = filter;
        cx.notify();
    }

    fn open_agent_git_file(&self, repo_path: &str) -> Result<(), String> {
        let repo_root = self
            .agent_git_panel
            .repo_root
            .as_deref()
            .ok_or_else(|| "Git panel is not ready".to_string())?;
        let path = Path::new(repo_root).join(repo_path);

        #[cfg(target_os = "macos")]
        let status = Command::new("open").arg(&path).status();
        #[cfg(target_os = "linux")]
        let status = Command::new("xdg-open").arg(&path).status();
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let status = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Opening files is unsupported on this platform",
        ));

        status
            .map_err(|error| format!("Failed to open '{}': {}", path.display(), error))?
            .success()
            .then_some(())
            .ok_or_else(|| format!("Failed to open '{}'", path.display()))
    }

    fn shell_quote(value: &str) -> String {
        format!("'{}'", value.replace('\'', "'\\''"))
    }

    fn open_agent_git_full_diff(
        &mut self,
        repo_path: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let repo_root = self
            .agent_git_panel
            .repo_root
            .clone()
            .ok_or_else(|| "Git panel is not ready".to_string())?;
        let command = if self
            .agent_git_panel
            .entries
            .iter()
            .find(|entry| entry.repo_path == repo_path)
            .is_some_and(AgentGitPanelEntry::is_untracked)
        {
            let full_path = Path::new(repo_root.as_str()).join(repo_path);
            format!(
                "git -C {} diff --no-index --unified=20 -- /dev/null {} | less -R\n",
                Self::shell_quote(repo_root.as_str()),
                Self::shell_quote(full_path.to_string_lossy().as_ref())
            )
        } else {
            format!(
                "git -C {} diff --no-ext-diff --unified=20 HEAD -- {} | less -R\n",
                Self::shell_quote(repo_root.as_str()),
                Self::shell_quote(repo_path)
            )
        };

        self.add_tab_with_working_dir(Some(repo_root.as_str()), cx);
        if let Some(tab) = self.tabs.get(self.active_tab)
            && let Some(terminal) = tab.active_terminal()
        {
            terminal.write_input(command.as_bytes());
            cx.notify();
            Ok(())
        } else {
            Err("Failed to open diff tab".to_string())
        }
    }

    fn run_agent_git_mutation(
        &mut self,
        args: Vec<String>,
        success_message: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some(repo_root) = self.agent_git_panel.repo_root.clone() else {
            termy_toast::error("Git panel is not ready");
            self.notify_overlay(cx);
            return;
        };

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let result = smol::unblock(move || {
                let output = Command::new("git")
                    .arg("-C")
                    .arg(repo_root.as_str())
                    .args(args.iter().map(String::as_str))
                    .output()
                    .map_err(|error| format!("Failed to run git: {}", error))?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    return Err(if stderr.is_empty() {
                        "Git command failed".to_string()
                    } else {
                        stderr
                    });
                }
                Ok(())
            })
            .await;

            let _ = cx.update(|cx| {
                this.update(cx, |view, cx| match result {
                    Ok(()) => {
                        termy_toast::success(success_message);
                        view.refresh_agent_git_panel(cx);
                        view.notify_overlay(cx);
                    }
                    Err(error) => {
                        termy_toast::error(error);
                        view.notify_overlay(cx);
                    }
                })
            });
        })
        .detach();
    }

    fn discard_agent_git_entry(&mut self, entry: AgentGitPanelEntry, cx: &mut Context<Self>) {
        let Some(repo_root) = self.agent_git_panel.repo_root.clone() else {
            termy_toast::error("Git panel is not ready");
            self.notify_overlay(cx);
            return;
        };

        let repo_path = entry.repo_path.clone();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let title = if entry.is_untracked() {
                "Discard Untracked File".to_string()
            } else {
                "Discard File Changes".to_string()
            };
            let message = format!("Discard changes for '{}' ?", entry.path);
            let confirmed =
                smol::unblock(move || termy_native_sdk::confirm(&title, &message)).await;
            if !confirmed {
                return;
            }

            let result = smol::unblock(move || {
                if entry.is_untracked() {
                    let output = Command::new("git")
                        .arg("-C")
                        .arg(repo_root.as_str())
                        .args(["clean", "-f", "--", repo_path.as_str()])
                        .output()
                        .map_err(|error| format!("Failed to run git clean: {}", error))?;
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                        return Err(if stderr.is_empty() {
                            "git clean failed".to_string()
                        } else {
                            stderr
                        });
                    }
                    return Ok(());
                }

                let output = Command::new("git")
                    .arg("-C")
                    .arg(repo_root.as_str())
                    .args([
                        "restore",
                        "--staged",
                        "--worktree",
                        "--source=HEAD",
                        "--",
                        repo_path.as_str(),
                    ])
                    .output()
                    .map_err(|error| format!("Failed to run git restore: {}", error))?;
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    return Err(if stderr.is_empty() {
                        "git restore failed".to_string()
                    } else {
                        stderr
                    });
                }
                Ok(())
            })
            .await;

            let _ = cx.update(|cx| {
                this.update(cx, |view, cx| match result {
                    Ok(()) => {
                        termy_toast::success("Discarded file changes");
                        view.refresh_agent_git_panel(cx);
                        view.notify_overlay(cx);
                    }
                    Err(error) => {
                        termy_toast::error(error);
                        view.notify_overlay(cx);
                    }
                })
            });
        })
        .detach();
    }

    pub(super) fn commit_agent_git_panel_input(&mut self, cx: &mut Context<Self>) {
        let Some(mode) = self.agent_git_panel_input_mode else {
            return;
        };
        let value = self.agent_git_panel_input.text().trim().to_string();
        if value.is_empty() {
            termy_toast::error("Input cannot be empty");
            self.notify_overlay(cx);
            return;
        }
        self.cancel_agent_git_panel_input(cx);
        match mode {
            AgentGitPanelInputMode::Commit => {
                self.run_agent_git_mutation(
                    vec!["commit".to_string(), "-m".to_string(), value],
                    "Created commit",
                    cx,
                );
            }
            AgentGitPanelInputMode::CreateBranch => {
                self.run_agent_git_mutation(
                    vec!["checkout".to_string(), "-b".to_string(), value],
                    "Created branch",
                    cx,
                );
            }
            AgentGitPanelInputMode::SaveStash => {
                self.run_agent_git_mutation(
                    vec![
                        "stash".to_string(),
                        "push".to_string(),
                        "-m".to_string(),
                        value,
                    ],
                    "Saved stash",
                    cx,
                );
            }
        }
    }

    fn persisted_agent_workspace_db_path() -> Result<PathBuf, String> {
        let config_path = crate::config::ensure_config_file().map_err(|error| error.to_string())?;
        let parent = config_path
            .parent()
            .ok_or_else(|| format!("Invalid config path '{}'", config_path.display()))?;
        Ok(parent.join(AGENT_WORKSPACE_DB_FILE))
    }

    fn legacy_agent_workspace_json_path() -> Result<PathBuf, String> {
        let db_path = Self::persisted_agent_workspace_db_path()?;
        let parent = db_path
            .parent()
            .ok_or_else(|| format!("Invalid agent workspace path '{}'", db_path.display()))?;
        Ok(parent.join(LEGACY_AGENT_WORKSPACE_STATE_FILE))
    }

    fn load_persisted_agent_workspace_state() -> Result<PersistedAgentWorkspaceState, String> {
        let db_path = Self::persisted_agent_workspace_db_path()?;
        let legacy_path = Self::legacy_agent_workspace_json_path()?;
        let db = AgentWorkspaceDb::open(&db_path)?;
        load_or_migrate_agent_workspace_state(&db, &legacy_path)
    }

    fn store_persisted_agent_workspace_state(&self) -> Result<(), String> {
        let path = Self::persisted_agent_workspace_db_path()?;
        let state = PersistedAgentWorkspaceState {
            version: AGENT_WORKSPACE_SCHEMA_VERSION,
            sidebar_open: self.agent_sidebar_open,
            active_project_id: self.active_agent_project_id.clone(),
            collapsed_project_ids: {
                let mut ids = self
                    .collapsed_agent_project_ids
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>();
                ids.sort();
                ids
            },
            projects: self.agent_projects.clone(),
            threads: self
                .agent_threads
                .iter()
                .cloned()
                .map(|mut thread| {
                    thread.linked_tab_id = None;
                    thread
                })
                .collect(),
        };
        let db = AgentWorkspaceDb::open(&path)?;
        store_agent_workspace_state_to_db(&db, &state)
    }

    pub(super) fn restore_persisted_agent_workspace(&mut self) {
        match Self::load_persisted_agent_workspace_state() {
            Ok(state) => {
                self.agent_projects = state.projects;
                self.agent_threads = state.threads;
                self.collapsed_agent_project_ids = state
                    .collapsed_project_ids
                    .into_iter()
                    .filter(|project_id| {
                        self.agent_projects
                            .iter()
                            .any(|project| project.id == *project_id)
                    })
                    .collect();
                self.active_agent_project_id = state.active_project_id.filter(|project_id| {
                    self.agent_projects
                        .iter()
                        .any(|project| &project.id == project_id)
                });
                self.agent_sidebar_open =
                    if self.agent_projects.is_empty() && self.agent_threads.is_empty() {
                        self.agent_sidebar_enabled
                    } else {
                        state.sidebar_open
                    };
                if self.active_agent_project_id.is_none() {
                    self.active_agent_project_id = self
                        .sorted_agent_projects()
                        .first()
                        .map(|project| project.id.clone());
                }
            }
            Err(error) => {
                log::error!("Failed to restore agent workspace: {}", error);
                self.agent_sidebar_open = self.agent_sidebar_enabled;
            }
        }
    }

    pub(super) fn sync_persisted_agent_workspace(&self) {
        if let Err(error) = self.store_persisted_agent_workspace_state() {
            log::error!("Failed to persist agent workspace: {}", error);
        }
    }

    pub(super) fn toggle_agent_sidebar(&mut self, cx: &mut Context<Self>) {
        if !self.agent_sidebar_enabled {
            termy_toast::info(
                "Enable agent_sidebar_enabled in config.txt to use the agent workspace",
            );
            self.notify_overlay(cx);
            return;
        }

        self.agent_sidebar_open = !self.agent_sidebar_open;
        if !self.agent_sidebar_open {
            self.agent_sidebar_search_active = false;
            self.cancel_rename_agent_project(cx);
            self.cancel_rename_agent_thread(cx);
            self.hovered_agent_thread_id = None;
            self.close_agent_git_panel();
        }
        self.sync_persisted_agent_workspace();
        cx.notify();
    }

    pub(super) fn normalized_agent_working_dir(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        self.preferred_working_dir_for_new_agent_session(cx)
            .or_else(|| {
                resolve_launch_working_directory(
                    self.configured_working_dir.as_deref(),
                    self.terminal_runtime.working_dir_fallback,
                )
                .map(|path| path.to_string_lossy().into_owned())
            })
            .or_else(|| Self::user_home_dir().map(|path| path.to_string_lossy().into_owned()))
    }

    fn preferred_working_dir_for_new_agent_session(
        &mut self,
        cx: &mut Context<Self>,
    ) -> Option<String> {
        let active_tab = self.active_tab;
        let prompt_cwd = self
            .tabs
            .get(active_tab)
            .and_then(|tab| tab.last_prompt_cwd.clone());
        let process_cwd = self
            .tabs
            .get(active_tab)
            .and_then(TerminalTab::active_terminal)
            .and_then(Terminal::child_pid)
            .and_then(|pid| {
                self.cached_or_queued_working_dir_for_child_pid(pid, cx)
                    .or_else(|| {
                        let value = Self::working_dir_for_child_pid_blocking(pid);
                        self.complete_child_working_dir_lookup(pid, value.clone());
                        value
                    })
            });
        let title_cwd = self
            .tabs
            .get(active_tab)
            .and_then(|tab| {
                [
                    tab.explicit_title.as_deref(),
                    tab.shell_title.as_deref(),
                    Some(tab.title.as_str()),
                ]
                .into_iter()
                .flatten()
                .find_map(Self::working_dir_title_candidate)
            })
            .map(|candidate| candidate.to_string());

        Self::resolve_preferred_working_directory(
            None,
            prompt_cwd.as_deref(),
            process_cwd.as_deref(),
            title_cwd.as_deref(),
            self.configured_working_dir.as_deref(),
            self.terminal_runtime.working_dir_fallback,
        )
    }

    fn agent_project_name_for_path(path: &str) -> String {
        let path_obj = Path::new(path);
        path_obj
            .file_name()
            .and_then(|segment| segment.to_str())
            .map(str::trim)
            .filter(|segment| !segment.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| Self::display_working_directory_for_prompt(path_obj))
    }

    fn ensure_agent_project(&mut self, root_path: &str) -> String {
        let normalized = normalize_working_directory_candidate(Some(root_path))
            .unwrap_or_else(|| root_path.trim().to_string());
        let now = now_unix_ms();

        if let Some(project) = self
            .agent_projects
            .iter_mut()
            .find(|project| project.root_path == normalized)
        {
            project.updated_at_ms = now;
            return project.id.clone();
        }

        let project_id = next_agent_entity_id("project");
        self.agent_projects.push(AgentProject {
            id: project_id.clone(),
            name: Self::agent_project_name_for_path(&normalized),
            root_path: normalized,
            pinned: false,
            created_at_ms: now,
            updated_at_ms: now,
        });
        project_id
    }

    fn touch_agent_project(&mut self, project_id: &str) -> Option<String> {
        let project = self
            .agent_projects
            .iter_mut()
            .find(|project| project.id == project_id)?;
        project.updated_at_ms = now_unix_ms();
        Some(project.root_path.clone())
    }

    fn set_agent_project_pinned(
        &mut self,
        project_id: &str,
        pinned: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(project) = self
            .agent_projects
            .iter_mut()
            .find(|project| project.id == project_id)
        else {
            return false;
        };

        if project.pinned == pinned {
            return false;
        }

        project.pinned = pinned;
        self.sync_persisted_agent_workspace();
        cx.notify();
        true
    }

    fn set_agent_thread_pinned(
        &mut self,
        thread_id: &str,
        pinned: bool,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(thread) = self
            .agent_threads
            .iter_mut()
            .find(|thread| thread.id == thread_id)
        else {
            return false;
        };

        if thread.pinned == pinned {
            return false;
        }

        thread.pinned = pinned;
        thread.updated_at_ms = now_unix_ms();
        self.sync_persisted_agent_workspace();
        cx.notify();
        true
    }

    fn set_agent_sidebar_filter(&mut self, filter: AgentSidebarFilter, cx: &mut Context<Self>) {
        if self.agent_sidebar_filter == filter {
            return;
        }

        self.agent_sidebar_filter = filter;
        cx.notify();
    }

    fn are_all_agent_projects_collapsed(&self) -> bool {
        !self.agent_projects.is_empty()
            && self.collapsed_agent_project_ids.len() >= self.agent_projects.len()
    }

    fn set_all_agent_projects_collapsed(&mut self, collapsed: bool, cx: &mut Context<Self>) {
        if collapsed {
            self.collapsed_agent_project_ids = self
                .agent_projects
                .iter()
                .map(|project| project.id.clone())
                .collect();
            self.cancel_rename_agent_project(cx);
            self.cancel_rename_agent_thread(cx);
        } else {
            self.collapsed_agent_project_ids.clear();
        }

        self.sync_persisted_agent_workspace();
        cx.notify();
    }

    pub(super) fn begin_rename_agent_project(&mut self, project_id: &str, cx: &mut Context<Self>) {
        let Some(initial_name) = self
            .agent_projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| project.name.clone())
        else {
            return;
        };

        if self.is_command_palette_open() {
            self.close_command_palette(cx);
        }
        if self.search_open {
            self.close_search(cx);
        }
        if self.renaming_tab.is_some() {
            self.cancel_rename_tab(cx);
        }
        if self.renaming_agent_project_id.is_some() {
            self.cancel_rename_agent_project(cx);
        }
        if self.renaming_agent_thread_id.is_some() {
            self.cancel_rename_agent_thread(cx);
        }
        self.agent_sidebar_search_active = false;
        self.active_agent_project_id = Some(project_id.to_string());
        self.collapsed_agent_project_ids.remove(project_id);
        self.renaming_agent_project_id = Some(project_id.to_string());
        self.agent_project_rename_input.set_text(initial_name);
        self.reset_cursor_blink_phase();
        self.inline_input_selecting = false;
        cx.notify();
    }

    pub(super) fn commit_rename_agent_project(&mut self, cx: &mut Context<Self>) {
        let Some(project_id) = self.renaming_agent_project_id.clone() else {
            return;
        };
        let Some(project) = self
            .agent_projects
            .iter_mut()
            .find(|project| project.id == project_id)
        else {
            self.cancel_rename_agent_project(cx);
            return;
        };

        let trimmed = self.agent_project_rename_input.text().trim();
        project.name = if trimmed.is_empty() {
            Self::agent_project_name_for_path(&project.root_path)
        } else {
            Self::truncate_tab_title(trimmed)
        };
        project.updated_at_ms = now_unix_ms();
        self.sync_persisted_agent_workspace();
        self.cancel_rename_agent_project(cx);
    }

    pub(super) fn cancel_rename_agent_project(&mut self, cx: &mut Context<Self>) {
        if self.renaming_agent_project_id.take().is_some()
            || !self.agent_project_rename_input.text().is_empty()
        {
            self.agent_project_rename_input.clear();
            self.inline_input_selecting = false;
            cx.notify();
        }
    }

    fn create_agent_thread_for_active_tab(
        &mut self,
        agent: command_palette::AiAgentPreset,
        project_id: String,
        working_dir: &str,
    ) -> Option<String> {
        let now = now_unix_ms();
        let thread_id = next_agent_entity_id("thread");
        let thread_title = format!(
            "{} {}",
            agent.title(),
            Self::agent_project_name_for_path(working_dir)
        );
        let tab_id = self.tabs.get(self.active_tab)?.id;

        self.tabs.get_mut(self.active_tab)?.agent_thread_id = Some(thread_id.clone());
        self.agent_threads.push(AgentThread {
            id: thread_id.clone(),
            project_id: project_id.clone(),
            agent,
            title: thread_title.clone(),
            custom_title: None,
            pinned: false,
            launch_command: agent.launch_command().to_string(),
            working_dir: working_dir.to_string(),
            last_seen_title: Some(thread_title),
            last_seen_command: Some(agent.launch_command().to_string()),
            last_status_label: None,
            last_status_detail: None,
            created_at_ms: now,
            updated_at_ms: now,
            linked_tab_id: Some(tab_id),
        });
        self.active_agent_project_id = Some(project_id);
        if self.agent_sidebar_enabled {
            self.agent_sidebar_open = true;
        }
        Some(thread_id)
    }

    fn resume_saved_agent_thread(
        &mut self,
        thread_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let Some(thread_index) = self
            .agent_threads
            .iter()
            .position(|thread| thread.id == thread_id)
        else {
            return Err("Agent thread no longer exists".to_string());
        };

        if let Some(tab_index) = self.agent_threads[thread_index]
            .linked_tab_id
            .and_then(|tab_id| self.tab_index_by_id(tab_id))
        {
            self.switch_tab(tab_index, cx);
            return Ok(());
        }

        let command = self.agent_threads[thread_index].launch_command.clone();
        let working_dir = self.agent_threads[thread_index].working_dir.clone();
        let project_id = self.agent_threads[thread_index].project_id.clone();
        let previous_tab_count = self.tabs.len();
        self.add_tab_with_working_dir(Some(working_dir.as_str()), cx);

        if self.tabs.len() == previous_tab_count {
            return Err("Failed to create a tab for the saved thread".to_string());
        }

        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return Err("Failed to access the new agent tab".to_string());
        };
        let Some(terminal) = tab.active_terminal() else {
            return Err("Failed to access the new agent terminal".to_string());
        };

        let mut command_input = command.clone();
        if !command_input.ends_with('\n') {
            command_input.push('\n');
        }
        terminal.write_input(command_input.as_bytes());

        let now = now_unix_ms();
        let tab_id = tab.id;
        tab.agent_thread_id = Some(thread_id.to_string());
        let thread = &mut self.agent_threads[thread_index];
        thread.linked_tab_id = Some(tab_id);
        thread.updated_at_ms = now;
        thread.last_seen_command = Some(command);
        self.active_agent_project_id = Some(project_id);
        self.sync_persisted_agent_workspace();
        cx.notify();
        Ok(())
    }

    pub(super) fn launch_ai_agent_from_palette(
        &mut self,
        agent: command_palette::AiAgentPreset,
        project_id: Option<&str>,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let (project_id, working_dir) = match project_id {
            Some(project_id) => {
                let working_dir = self
                    .touch_agent_project(project_id)
                    .ok_or_else(|| "The selected project no longer exists".to_string())?;
                (project_id.to_string(), working_dir)
            }
            None => {
                let working_dir = self.normalized_agent_working_dir(cx).ok_or_else(|| {
                    "Could not resolve a working directory for the agent".to_string()
                })?;
                let project_id = self.ensure_agent_project(&working_dir);
                (project_id, working_dir)
            }
        };
        let previous_tab_count = self.tabs.len();
        self.add_tab_with_working_dir(Some(working_dir.as_str()), cx);

        if self.tabs.len() == previous_tab_count {
            return Err("Failed to create a tab for the agent".to_string());
        }

        let Some(tab) = self.tabs.get_mut(self.active_tab) else {
            return Err("Failed to access the new agent tab".to_string());
        };
        let Some(terminal) = tab.active_terminal() else {
            return Err("Failed to access the new agent terminal".to_string());
        };

        let mut command_input = agent.launch_command().to_string();
        if !command_input.ends_with('\n') {
            command_input.push('\n');
        }
        terminal.write_input(command_input.as_bytes());

        self.create_agent_thread_for_active_tab(agent, project_id, &working_dir)
            .ok_or_else(|| "Failed to link the new agent thread".to_string())?;
        self.sync_persisted_agent_workspace();
        cx.notify();
        Ok(())
    }

    fn detach_agent_thread_from_live_tab(&mut self, thread_id: &str) {
        let linked_tab_id = self
            .agent_threads
            .iter()
            .find(|thread| thread.id == thread_id)
            .and_then(|thread| thread.linked_tab_id);
        let Some(tab_id) = linked_tab_id else {
            return;
        };
        let Some(tab_index) = self.tab_index_by_id(tab_id) else {
            return;
        };
        if let Some(tab) = self.tabs.get_mut(tab_index)
            && tab.agent_thread_id.as_deref() == Some(thread_id)
        {
            tab.agent_thread_id = None;
        }
    }

    pub(super) fn delete_agent_thread(&mut self, thread_id: &str) -> Result<(), String> {
        let Some(thread_index) = self
            .agent_threads
            .iter()
            .position(|thread| thread.id == thread_id)
        else {
            return Err("Agent thread no longer exists".to_string());
        };

        let project_id = self.agent_threads[thread_index].project_id.clone();
        self.detach_agent_thread_from_live_tab(thread_id);
        self.agent_threads.remove(thread_index);
        if let Some(project) = self
            .agent_projects
            .iter_mut()
            .find(|project| project.id == project_id)
        {
            project.updated_at_ms = now_unix_ms();
        }

        self.sync_persisted_agent_workspace();
        Ok(())
    }

    pub(super) fn delete_agent_project(&mut self, project_id: &str) -> Result<usize, String> {
        let Some(project_index) = self
            .agent_projects
            .iter()
            .position(|project| project.id == project_id)
        else {
            return Err("Agent project no longer exists".to_string());
        };

        let thread_ids = self
            .agent_threads
            .iter()
            .filter(|thread| thread.project_id == project_id)
            .map(|thread| thread.id.clone())
            .collect::<Vec<_>>();
        for thread_id in &thread_ids {
            self.detach_agent_thread_from_live_tab(thread_id);
        }

        let removed_threads = thread_ids.len();
        self.agent_threads
            .retain(|thread| thread.project_id != project_id);
        self.agent_projects.remove(project_index);

        if self.active_agent_project_id.as_deref() == Some(project_id) {
            self.active_agent_project_id = self
                .sorted_agent_projects()
                .first()
                .map(|project| project.id.clone());
        }

        self.sync_persisted_agent_workspace();
        Ok(removed_threads)
    }

    pub(super) fn agent_thread_archive_snapshot_for_tab(
        &self,
        index: usize,
    ) -> Option<(
        Option<String>,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
    )> {
        let tab = self.tabs.get(index)?;
        let (status_label, status_detail) = tab
            .agent_thread_id
            .as_deref()
            .and_then(|thread_id| {
                self.agent_threads
                    .iter()
                    .find(|thread| thread.id == thread_id)
                    .map(|thread| self.agent_thread_status_presentation(thread))
            })
            .map(|status| (Some(status.label), status.detail))
            .unwrap_or_default();

        Some((
            tab.agent_thread_id.clone(),
            tab.title.clone(),
            tab.current_command.clone(),
            status_label,
            status_detail,
        ))
    }

    pub(super) fn archive_agent_thread_snapshot(
        &mut self,
        thread_id: Option<&str>,
        title: &str,
        current_command: Option<&str>,
        status_label: Option<&str>,
        status_detail: Option<&str>,
    ) {
        let Some(thread_id) = thread_id else {
            return;
        };
        let Some(thread) = self
            .agent_threads
            .iter_mut()
            .find(|thread| thread.id == thread_id)
        else {
            return;
        };

        thread.linked_tab_id = None;
        thread.last_seen_title = Some(title.to_string());
        thread.last_seen_command = current_command.map(ToOwned::to_owned);
        thread.last_status_label = status_label.map(ToOwned::to_owned);
        thread.last_status_detail = status_detail.map(ToOwned::to_owned);
        thread.updated_at_ms = now_unix_ms();
        if let Some(project) = self
            .agent_projects
            .iter_mut()
            .find(|project| project.id == thread.project_id)
        {
            project.updated_at_ms = thread.updated_at_ms;
        }
        self.sync_persisted_agent_workspace();
    }

    pub(super) fn sync_agent_workspace_to_active_tab(&mut self) {
        let Some(project_id) = self
            .tabs
            .get(self.active_tab)
            .and_then(|tab| tab.agent_thread_id.as_deref())
            .and_then(|thread_id| {
                self.agent_threads
                    .iter()
                    .find(|thread| thread.id == thread_id)
                    .map(|thread| thread.project_id.clone())
            })
        else {
            return;
        };

        self.active_agent_project_id = Some(project_id);
    }

    fn begin_agent_sidebar_search(&mut self, cx: &mut Context<Self>) {
        if self.is_command_palette_open() {
            self.close_command_palette(cx);
        }
        if self.search_open {
            self.close_search(cx);
        }
        if self.renaming_tab.is_some() {
            self.cancel_rename_tab(cx);
        }
        if self.renaming_agent_thread_id.is_some() {
            self.cancel_rename_agent_thread(cx);
        }

        self.agent_sidebar_search_active = true;
        self.reset_cursor_blink_phase();
        self.inline_input_selecting = false;
        cx.notify();
    }

    pub(super) fn dismiss_agent_sidebar_search(&mut self, cx: &mut Context<Self>) {
        let had_query = !self.agent_sidebar_search_input.text().is_empty();
        let was_active = self.agent_sidebar_search_active;
        if !had_query && !was_active {
            return;
        }

        if had_query {
            self.agent_sidebar_search_input.clear();
        } else {
            self.agent_sidebar_search_active = false;
        }
        self.inline_input_selecting = false;
        cx.notify();
    }

    fn thread_project_id(&self, thread_id: &str) -> Option<&str> {
        self.agent_threads
            .iter()
            .find(|thread| thread.id == thread_id)
            .map(|thread| thread.project_id.as_str())
    }

    pub(super) fn begin_rename_agent_thread(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        let Some(initial_title) = self
            .agent_threads
            .iter()
            .find(|thread| thread.id == thread_id)
            .map(|thread| {
                thread
                    .custom_title
                    .clone()
                    .unwrap_or_else(|| self.agent_thread_display_title(thread))
            })
        else {
            return;
        };

        if self.is_command_palette_open() {
            self.close_command_palette(cx);
        }
        if self.search_open {
            self.close_search(cx);
        }
        if self.renaming_agent_project_id.is_some() {
            self.cancel_rename_agent_project(cx);
        }
        self.agent_sidebar_search_active = false;

        self.renaming_agent_thread_id = Some(thread_id.to_string());
        self.agent_thread_rename_input.set_text(initial_title);
        self.reset_cursor_blink_phase();
        self.inline_input_selecting = false;
        cx.notify();
    }

    pub(super) fn commit_rename_agent_thread(&mut self, cx: &mut Context<Self>) {
        let Some(thread_id) = self.renaming_agent_thread_id.clone() else {
            return;
        };
        let Some(thread) = self
            .agent_threads
            .iter_mut()
            .find(|thread| thread.id == thread_id)
        else {
            self.cancel_rename_agent_thread(cx);
            return;
        };

        let trimmed = self.agent_thread_rename_input.text().trim();
        thread.custom_title = (!trimmed.is_empty()).then(|| Self::truncate_tab_title(trimmed));
        thread.updated_at_ms = now_unix_ms();
        self.sync_persisted_agent_workspace();
        self.cancel_rename_agent_thread(cx);
    }

    pub(super) fn cancel_rename_agent_thread(&mut self, cx: &mut Context<Self>) {
        if self.renaming_agent_thread_id.take().is_some()
            || !self.agent_thread_rename_input.text().is_empty()
        {
            self.agent_thread_rename_input.clear();
            self.inline_input_selecting = false;
            cx.notify();
        }
    }

    fn toggle_agent_project_collapsed(&mut self, project_id: &str, cx: &mut Context<Self>) {
        if self.collapsed_agent_project_ids.contains(project_id) {
            self.collapsed_agent_project_ids.remove(project_id);
        } else {
            self.collapsed_agent_project_ids
                .insert(project_id.to_string());
            if self.renaming_agent_project_id.as_deref() == Some(project_id) {
                self.cancel_rename_agent_project(cx);
            }
            if self
                .renaming_agent_thread_id
                .as_deref()
                .and_then(|thread_id| self.thread_project_id(thread_id))
                == Some(project_id)
            {
                self.cancel_rename_agent_thread(cx);
            }
        }
        self.sync_persisted_agent_workspace();
        cx.notify();
    }

    fn agent_thread_delete_confirm_params(
        thread: &AgentThread,
        display_title: &str,
    ) -> (String, String) {
        let thread_title = display_title;
        let message = if thread.linked_tab_id.is_some() {
            format!(
                "Delete the thread \"{}\" from the sidebar?\n\nThe terminal tab stays open, but it will no longer be tracked as an agent thread.",
                thread_title
            )
        } else {
            format!("Delete the saved thread \"{}\"?", thread_title)
        };
        ("Delete Agent Thread".to_string(), message)
    }

    fn agent_project_delete_confirm_params(
        project: &AgentProject,
        thread_count: usize,
    ) -> (String, String) {
        let message = if thread_count == 0 {
            format!(
                "Delete the project \"{}\"?\n\nIts folder reference will be removed from the agent sidebar.",
                project.name
            )
        } else {
            format!(
                "Delete the project \"{}\" and its {} thread(s)?\n\nOpen terminal tabs stay open, but they will no longer be tracked in the agent sidebar.",
                project.name, thread_count
            )
        };
        ("Delete Agent Project".to_string(), message)
    }

    pub(super) fn open_ai_agents_palette_for_project_from_sidebar(
        &mut self,
        project_id: Option<String>,
        cx: &mut Context<Self>,
    ) {
        self.command_palette.set_agent_launch_project_id(project_id);
        self.open_command_palette_in_mode(command_palette::CommandPaletteMode::Agents, cx);
    }

    fn schedule_agent_project_context_menu(&mut self, project_id: String, cx: &mut Context<Self>) {
        let Some((project_pinned, project_is_collapsed, git_panel_visible)) = self
            .agent_projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| {
                (
                    project.pinned,
                    self.collapsed_agent_project_ids
                        .contains(project.id.as_str()),
                    self.agent_git_panel_matches_target_path(project.root_path.as_str()),
                )
            })
        else {
            return;
        };

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let action = smol::unblock(move || {
                termy_native_sdk::show_agent_project_context_menu(
                    project_pinned,
                    project_is_collapsed,
                    git_panel_visible,
                )
            })
            .await;
            let Some(action) = action else {
                return;
            };
            match action {
                termy_native_sdk::AgentProjectContextMenuAction::NewSession => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            view.open_ai_agents_palette_for_project_from_sidebar(
                                Some(project_id.clone()),
                                cx,
                            );
                        })
                    });
                }
                termy_native_sdk::AgentProjectContextMenuAction::RenameProject => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            view.begin_rename_agent_project(project_id.as_str(), cx);
                        })
                    });
                }
                termy_native_sdk::AgentProjectContextMenuAction::ToggleGitPanel => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            view.toggle_agent_git_panel_for_project(project_id.as_str(), cx);
                        })
                    });
                }
                termy_native_sdk::AgentProjectContextMenuAction::Pin => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            let _ = view.set_agent_project_pinned(project_id.as_str(), true, cx);
                        })
                    });
                }
                termy_native_sdk::AgentProjectContextMenuAction::Unpin => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            let _ = view.set_agent_project_pinned(project_id.as_str(), false, cx);
                        })
                    });
                }
                termy_native_sdk::AgentProjectContextMenuAction::RevealProject => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            match view.reveal_agent_project(project_id.as_str()) {
                                Ok(()) => {
                                    termy_toast::success("Revealed project folder");
                                    view.notify_overlay(cx);
                                }
                                Err(error) => {
                                    termy_toast::error(error);
                                    view.notify_overlay(cx);
                                }
                            }
                        })
                    });
                }
                termy_native_sdk::AgentProjectContextMenuAction::CopyPath => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            match view.copy_agent_project_path(project_id.as_str(), cx) {
                                Ok(()) => {
                                    termy_toast::success("Copied project path");
                                    view.notify_overlay(cx);
                                }
                                Err(error) => {
                                    termy_toast::error(error);
                                    view.notify_overlay(cx);
                                }
                            }
                        })
                    });
                }
                termy_native_sdk::AgentProjectContextMenuAction::CollapseProject => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            view.collapsed_agent_project_ids.insert(project_id.clone());
                            view.sync_persisted_agent_workspace();
                            cx.notify();
                        })
                    });
                }
                termy_native_sdk::AgentProjectContextMenuAction::ExpandProject => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            view.collapsed_agent_project_ids.remove(project_id.as_str());
                            view.sync_persisted_agent_workspace();
                            cx.notify();
                        })
                    });
                }
                termy_native_sdk::AgentProjectContextMenuAction::DeleteProject => {
                    let confirm_params = cx.update(|cx| {
                        this.update(cx, |view, _cx| {
                            let project = view
                                .agent_projects
                                .iter()
                                .find(|project| project.id == project_id)?;
                            let thread_count = view.project_thread_count(project_id.as_str());
                            Some(Self::agent_project_delete_confirm_params(
                                project,
                                thread_count,
                            ))
                        })
                        .ok()
                        .flatten()
                    });
                    let Some((title, message)) = confirm_params else {
                        return;
                    };
                    let confirmed =
                        smol::unblock(move || termy_native_sdk::confirm(&title, &message)).await;
                    if !confirmed {
                        return;
                    }
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            let project_name = view
                                .agent_projects
                                .iter()
                                .find(|p| p.id == project_id)
                                .map(|p| p.name.clone());
                            match view.delete_agent_project(project_id.as_str()) {
                                Ok(_) => {
                                    termy_toast::success(format!(
                                        "Deleted project \"{}\"",
                                        project_name.unwrap_or_default()
                                    ));
                                    view.notify_overlay(cx);
                                    cx.notify();
                                }
                                Err(error) => {
                                    termy_toast::error(error);
                                    view.notify_overlay(cx);
                                }
                            }
                        })
                    });
                }
            }
        })
        .detach();
    }

    fn schedule_agent_thread_context_menu(&mut self, thread_id: String, cx: &mut Context<Self>) {
        let Some((has_live_session, thread_pinned, git_panel_visible)) = self
            .agent_threads
            .iter()
            .find(|thread| thread.id == thread_id)
            .map(|thread| {
                (
                    self.agent_thread_has_live_session(thread),
                    thread.pinned,
                    self.agent_git_panel_matches_target_path(thread.working_dir.as_str()),
                )
            })
        else {
            return;
        };

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let action = smol::unblock(move || {
                termy_native_sdk::show_agent_thread_context_menu(
                    has_live_session,
                    thread_pinned,
                    git_panel_visible,
                )
            })
            .await;
            let Some(action) = action else {
                return;
            };
            match action {
                termy_native_sdk::AgentThreadContextMenuAction::RestartSession => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            match view.restart_agent_thread_session(thread_id.as_str(), cx) {
                                Ok(()) => {
                                    termy_toast::success("Restarted agent session");
                                    view.notify_overlay(cx);
                                    cx.notify();
                                }
                                Err(error) => {
                                    termy_toast::error(error);
                                    view.notify_overlay(cx);
                                }
                            }
                        })
                    });
                }
                termy_native_sdk::AgentThreadContextMenuAction::CloseSession => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            match view.close_agent_thread_session(thread_id.as_str(), cx) {
                                Ok(()) => {
                                    termy_toast::success("Closed agent session");
                                    view.notify_overlay(cx);
                                    cx.notify();
                                }
                                Err(error) => {
                                    termy_toast::error(error);
                                    view.notify_overlay(cx);
                                }
                            }
                        })
                    });
                }
                termy_native_sdk::AgentThreadContextMenuAction::RenameThread => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            view.begin_rename_agent_thread(thread_id.as_str(), cx);
                        })
                    });
                }
                termy_native_sdk::AgentThreadContextMenuAction::ToggleGitPanel => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            view.toggle_agent_git_panel_for_thread(thread_id.as_str(), cx);
                        })
                    });
                }
                termy_native_sdk::AgentThreadContextMenuAction::Pin => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            let _ = view.set_agent_thread_pinned(thread_id.as_str(), true, cx);
                        })
                    });
                }
                termy_native_sdk::AgentThreadContextMenuAction::Unpin => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            let _ = view.set_agent_thread_pinned(thread_id.as_str(), false, cx);
                        })
                    });
                }
                termy_native_sdk::AgentThreadContextMenuAction::DeleteThread => {
                    let _ = cx.update(|cx| {
                        this.update(cx, |view, cx| {
                            view.request_delete_agent_thread(thread_id.as_str(), cx);
                        })
                    });
                }
            }
        })
        .detach();
    }

    fn request_delete_agent_thread(&mut self, thread_id: &str, cx: &mut Context<Self>) {
        let Some(confirm_params) = self
            .agent_threads
            .iter()
            .find(|thread| thread.id == thread_id)
            .map(|thread| {
                let display_title = self.agent_thread_display_title(thread);
                Self::agent_thread_delete_confirm_params(thread, &display_title)
            })
        else {
            return;
        };

        let thread_id = thread_id.to_string();
        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            let (title, message) = confirm_params;
            let confirmed =
                smol::unblock(move || termy_native_sdk::confirm(&title, &message)).await;
            if !confirmed {
                return;
            }

            let _ = cx.update(|cx| {
                this.update(cx, |view, cx| {
                    match view.delete_agent_thread(thread_id.as_str()) {
                        Ok(()) => {
                            termy_toast::success("Deleted agent thread");
                            view.notify_overlay(cx);
                            cx.notify();
                        }
                        Err(error) => {
                            termy_toast::error(error);
                            view.notify_overlay(cx);
                        }
                    }
                })
            });
        })
        .detach();
    }

    fn close_agent_thread_session(
        &mut self,
        thread_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let Some(thread) = self
            .agent_threads
            .iter()
            .find(|thread| thread.id == thread_id)
        else {
            return Err("Agent thread no longer exists".to_string());
        };

        let Some(tab_id) = thread.linked_tab_id else {
            return Err("This thread has no open session to close".to_string());
        };

        let Some(tab_index) = self.tab_index_by_id(tab_id) else {
            self.detach_agent_thread_from_live_tab(thread_id);
            self.sync_persisted_agent_workspace();
            return Err("This thread's session is no longer open".to_string());
        };

        if self.tabs.get(tab_index).is_some_and(|tab| tab.pinned) {
            return Err("Pinned tabs must be unpinned before closing".to_string());
        }

        if self.runtime_kind() == RuntimeKind::Native && self.tabs.len() <= 1 {
            return Err("Can't close the only open tab".to_string());
        }

        self.close_tab(tab_index, cx);
        Ok(())
    }

    fn restart_agent_thread_session(
        &mut self,
        thread_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let Some(thread_index) = self
            .agent_threads
            .iter()
            .position(|thread| thread.id == thread_id)
        else {
            return Err("Agent thread no longer exists".to_string());
        };

        if let Some(tab_index) = self.agent_threads[thread_index]
            .linked_tab_id
            .and_then(|tab_id| self.tab_index_by_id(tab_id))
        {
            if self.tabs.get(tab_index).is_some_and(|tab| tab.pinned) {
                return Err("Pinned tabs must be unpinned before restarting".to_string());
            }

            if self.runtime_kind() == RuntimeKind::Native && self.tabs.len() <= 1 {
                if let Some((thread_id, title, current_command, status_label, status_detail)) =
                    self.agent_thread_archive_snapshot_for_tab(tab_index)
                {
                    self.archive_agent_thread_snapshot(
                        thread_id.as_deref(),
                        title.as_str(),
                        current_command.as_deref(),
                        status_label.as_deref(),
                        status_detail.as_deref(),
                    );
                }
                if let Some(tab) = self.tabs.get_mut(tab_index)
                    && tab.agent_thread_id.as_deref() == Some(thread_id)
                {
                    tab.agent_thread_id = None;
                }
            } else {
                self.close_tab(tab_index, cx);
            }
        }

        self.resume_saved_agent_thread(thread_id, cx)
    }

    fn copy_agent_project_path(
        &mut self,
        project_id: &str,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let Some(project_path) = self
            .agent_projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| project.root_path.clone())
        else {
            return Err("Agent project no longer exists".to_string());
        };

        cx.write_to_clipboard(ClipboardItem::new_string(project_path));
        Ok(())
    }

    fn reveal_agent_project(&mut self, project_id: &str) -> Result<(), String> {
        let Some(project_path) = self
            .agent_projects
            .iter()
            .find(|project| project.id == project_id)
            .map(|project| project.root_path.clone())
        else {
            return Err("Agent project no longer exists".to_string());
        };

        let path = Path::new(&project_path);
        if !path.exists() {
            return Err(format!("Project path '{}' no longer exists", project_path));
        }

        #[cfg(target_os = "macos")]
        let status = Command::new("open").arg("-R").arg(path).status();
        #[cfg(target_os = "linux")]
        let status = Command::new("xdg-open").arg(path).status();
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        let status = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Reveal is unsupported on this platform",
        ));

        match status {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(format!("Reveal command exited with status {}", status)),
            Err(error) => Err(format!("Failed to reveal project path: {}", error)),
        }
    }

    fn agent_thread_has_live_session(&self, thread: &AgentThread) -> bool {
        thread
            .linked_tab_id
            .and_then(|tab_id| self.tab_index_by_id(tab_id))
            .is_some()
    }

    fn agent_thread_shows_activity(&self, thread: &AgentThread, is_active: bool) -> bool {
        !is_active
            && matches!(
                self.agent_thread_runtime_status(thread),
                AgentThreadRuntimeStatus::Busy
            )
    }

    fn agent_thread_runtime_status(&self, thread: &AgentThread) -> AgentThreadRuntimeStatus {
        let Some(tab_id) = thread.linked_tab_id else {
            return AgentThreadRuntimeStatus::Saved;
        };
        let Some(tab_index) = self.tab_index_by_id(tab_id) else {
            return AgentThreadRuntimeStatus::Saved;
        };
        let Some(tab) = self.tabs.get(tab_index) else {
            return AgentThreadRuntimeStatus::Saved;
        };

        if tab.running_process
            || tab
                .current_command
                .as_deref()
                .is_some_and(|command| !command.trim().is_empty())
        {
            AgentThreadRuntimeStatus::Busy
        } else {
            AgentThreadRuntimeStatus::Ready
        }
    }

    fn extract_agent_status_line(
        grid: &alacritty_terminal::grid::Grid<alacritty_terminal::term::cell::Cell>,
        line_idx: i32,
    ) -> Option<String> {
        use alacritty_terminal::index::{Column, Line};

        let line = Line(line_idx);
        let cols = grid.columns();
        let total_lines = grid.total_lines();
        if line_idx < -(total_lines as i32 - grid.screen_lines() as i32)
            || line_idx >= grid.screen_lines() as i32
        {
            return None;
        }

        let mut text = String::with_capacity(cols);
        for col in 0..cols {
            let cell = &grid[line][Column(col)];
            let c = cell.c;
            if c == '\0' || cell.flags.contains(Flags::WIDE_CHAR_SPACER) || c.is_control() {
                text.push(' ');
            } else {
                text.push(c);
            }
        }

        Some(text.trim_end().to_string())
    }

    fn normalize_agent_status_line(line: &str) -> Option<String> {
        let normalized = line.split_whitespace().collect::<Vec<_>>().join(" ");
        let normalized = normalized.trim();
        if normalized.is_empty() || !normalized.chars().any(|ch| ch.is_alphanumeric()) {
            return None;
        }
        Some(normalized.to_string())
    }

    fn collect_visible_agent_status_lines(terminal: &Terminal) -> Vec<String> {
        let mut lines = Vec::new();
        let rows = i32::from(terminal.size().rows.max(1));
        let start_line = (rows - AGENT_STATUS_VISIBLE_LINE_COUNT).max(0);
        let _ = terminal.with_grid(|grid| {
            for line_idx in start_line..rows {
                if let Some(line) = Self::extract_agent_status_line(grid, line_idx)
                    .and_then(|line| Self::normalize_agent_status_line(&line))
                {
                    lines.push(line);
                }
            }
        });
        lines
    }

    fn find_last_status_line(lines: &[String], needles: &[&str]) -> Option<String> {
        lines.iter().rev().find_map(|line| {
            let normalized = line.to_ascii_lowercase();
            needles
                .iter()
                .any(|needle| normalized.contains(needle))
                .then(|| line.clone())
        })
    }

    fn detect_generic_agent_status(lines: &[String]) -> Option<AgentThreadStatusPresentation> {
        if let Some(line) = Self::find_last_status_line(
            lines,
            &["error", "failed", "unable to", "panic", "denied", "refused"],
        ) {
            return Some(AgentThreadStatusPresentation {
                label: "error".to_string(),
                detail: Some(line),
                tone: AgentThreadStatusTone::Error,
            });
        }

        if let Some(line) = Self::find_last_status_line(
            lines,
            &[
                "approval",
                "approve",
                "permission",
                "permissions",
                "allow",
                "confirm",
            ],
        ) {
            return Some(AgentThreadStatusPresentation {
                label: "approval".to_string(),
                detail: Some(line),
                tone: AgentThreadStatusTone::Warning,
            });
        }

        if let Some(line) = Self::find_last_status_line(
            lines,
            &["thinking", "analyzing", "reasoning", "planning", "working"],
        ) {
            return Some(AgentThreadStatusPresentation {
                label: "thinking".to_string(),
                detail: Some(line),
                tone: AgentThreadStatusTone::Active,
            });
        }

        if let Some(line) = Self::find_last_status_line(
            lines,
            &[
                "running",
                "executing",
                "editing",
                "searching",
                "reading",
                "writing",
                "patching",
                "tool",
            ],
        ) {
            return Some(AgentThreadStatusPresentation {
                label: "tool".to_string(),
                detail: Some(line),
                tone: AgentThreadStatusTone::Active,
            });
        }

        None
    }

    fn detect_provider_status(
        agent: command_palette::AiAgentPreset,
        lines: &[String],
    ) -> Option<AgentThreadStatusPresentation> {
        match agent {
            command_palette::AiAgentPreset::Pi => {
                if let Some(line) =
                    Self::find_last_status_line(lines, &["no-model", "no models available"])
                {
                    return Some(AgentThreadStatusPresentation {
                        label: "setup".to_string(),
                        detail: Some(line),
                        tone: AgentThreadStatusTone::Warning,
                    });
                }
            }
            command_palette::AiAgentPreset::Claude => {
                if let Some(line) = Self::find_last_status_line(
                    lines,
                    &[
                        "unable to connect to anthropic services",
                        "api.anthropic.com",
                    ],
                ) {
                    return Some(AgentThreadStatusPresentation {
                        label: "error".to_string(),
                        detail: Some(line),
                        tone: AgentThreadStatusTone::Error,
                    });
                }
                if let Some(line) = Self::find_last_status_line(lines, &["welcome to claude code"])
                {
                    return Some(AgentThreadStatusPresentation {
                        label: "starting".to_string(),
                        detail: Some(line),
                        tone: AgentThreadStatusTone::Active,
                    });
                }
            }
            command_palette::AiAgentPreset::OpenCode => {
                if let Some(line) = Self::find_last_status_line(lines, &["models.dev"]) {
                    return Some(AgentThreadStatusPresentation {
                        label: "error".to_string(),
                        detail: Some(line),
                        tone: AgentThreadStatusTone::Error,
                    });
                }
                if let Some(generic) = Self::detect_generic_agent_status(lines) {
                    return Some(generic);
                }
                if let Some(line) = Self::find_last_status_line(lines, &["ask anything"]) {
                    let detail = Self::find_last_status_line(lines, &["opencode zen", "build "])
                        .or(Some(line));
                    return Some(AgentThreadStatusPresentation {
                        label: "ready".to_string(),
                        detail,
                        tone: AgentThreadStatusTone::Active,
                    });
                }
                if let Some(line) = Self::find_last_status_line(lines, &["mcp:", "servers"]) {
                    return Some(AgentThreadStatusPresentation {
                        label: "ready".to_string(),
                        detail: Some(line),
                        tone: AgentThreadStatusTone::Active,
                    });
                }
            }
            command_palette::AiAgentPreset::Codex => {
                if let Some(line) = Self::find_last_status_line(
                    lines,
                    &["otel exporter", "panicked", "could not create"],
                ) {
                    return Some(AgentThreadStatusPresentation {
                        label: "error".to_string(),
                        detail: Some(line),
                        tone: AgentThreadStatusTone::Error,
                    });
                }
            }
            command_palette::AiAgentPreset::Cursor => {}
        }

        if let Some(generic) = Self::detect_generic_agent_status(lines) {
            return Some(generic);
        }

        match agent {
            command_palette::AiAgentPreset::Pi => {
                Self::find_last_status_line(lines, &["%/", "(auto)", "no-model"]).map(|line| {
                    AgentThreadStatusPresentation {
                        label: "ready".to_string(),
                        detail: Some(line),
                        tone: AgentThreadStatusTone::Active,
                    }
                })
            }
            command_palette::AiAgentPreset::Claude => None,
            command_palette::AiAgentPreset::Cursor => None,
            command_palette::AiAgentPreset::OpenCode => None,
            command_palette::AiAgentPreset::Codex => None,
        }
    }

    fn live_agent_thread_fallback_detail(&self, thread: &AgentThread, tab: &TerminalTab) -> String {
        if let Some(command) = tab.current_command.as_deref()
            && !command.trim().is_empty()
        {
            return command.to_string();
        }

        format!(
            "{} attached",
            Self::display_working_directory_for_prompt(Path::new(&thread.working_dir))
        )
    }

    fn saved_agent_thread_detail(&self, thread: &AgentThread) -> Option<String> {
        match (
            thread.last_status_label.as_deref(),
            thread.last_status_detail.as_deref(),
        ) {
            (Some(label), Some(detail)) => Some(format!("Last {}: {}", label, detail)),
            (Some(label), None) => Some(format!("Last {}", label)),
            (None, Some(detail)) => Some(detail.to_string()),
            (None, None) => None,
        }
    }

    fn agent_thread_status_presentation(
        &self,
        thread: &AgentThread,
    ) -> AgentThreadStatusPresentation {
        if let Some(tab) = thread
            .linked_tab_id
            .and_then(|tab_id| self.tab_index_by_id(tab_id))
            .and_then(|index| self.tabs.get(index))
        {
            if let Some(terminal) = tab.active_terminal() {
                let lines = Self::collect_visible_agent_status_lines(terminal);
                if let Some(mut provider_status) =
                    Self::detect_provider_status(thread.agent, &lines)
                {
                    if provider_status.detail.is_none() {
                        provider_status.detail =
                            Some(self.live_agent_thread_fallback_detail(thread, tab));
                    }
                    return provider_status;
                }
            }

            let runtime_status = self.agent_thread_runtime_status(thread);
            return AgentThreadStatusPresentation {
                label: runtime_status.label().to_string(),
                detail: Some(self.live_agent_thread_fallback_detail(thread, tab)),
                tone: AgentThreadStatusTone::Active,
            };
        }

        AgentThreadStatusPresentation {
            label: AgentThreadRuntimeStatus::Saved.label().to_string(),
            detail: self
                .saved_agent_thread_detail(thread)
                .or_else(|| thread.last_seen_command.clone())
                .or_else(|| {
                    Some(Self::display_working_directory_for_prompt(Path::new(
                        &thread.working_dir,
                    )))
                }),
            tone: AgentThreadStatusTone::Muted,
        }
    }

    fn agent_thread_display_title(&self, thread: &AgentThread) -> String {
        thread
            .custom_title
            .clone()
            .or_else(|| {
                thread
                    .linked_tab_id
                    .and_then(|tab_id| self.tab_index_by_id(tab_id))
                    .and_then(|index| self.tabs.get(index))
                    .map(|tab| tab.title.clone())
            })
            .or_else(|| thread.last_seen_title.clone())
            .unwrap_or_else(|| thread.title.clone())
    }

    fn project_thread_count(&self, project_id: &str) -> usize {
        self.agent_threads
            .iter()
            .filter(|thread| thread.project_id == project_id)
            .count()
    }

    fn sorted_agent_projects(&self) -> Vec<&AgentProject> {
        let mut projects = self.agent_projects.iter().collect::<Vec<_>>();
        projects.sort_by_key(|project| (!project.pinned, std::cmp::Reverse(project.updated_at_ms)));
        projects
    }

    fn sorted_agent_threads_for_project(&self, project_id: &str) -> Vec<&AgentThread> {
        let mut threads = self
            .agent_threads
            .iter()
            .filter(|thread| thread.project_id == project_id)
            .collect::<Vec<_>>();
        threads.sort_by_key(|thread| (!thread.pinned, std::cmp::Reverse(thread.updated_at_ms)));
        threads
    }

    fn agent_sidebar_search_terms(&self) -> Vec<String> {
        self.agent_sidebar_search_input
            .text()
            .split_whitespace()
            .map(|term| term.trim().to_ascii_lowercase())
            .filter(|term| !term.is_empty())
            .collect()
    }

    fn agent_sidebar_query_matches_text(text: &str, terms: &[String]) -> bool {
        if terms.is_empty() {
            return true;
        }

        let normalized = text.to_ascii_lowercase();
        terms.iter().all(|term| normalized.contains(term))
    }

    fn agent_sidebar_project_matches_terms(project: &AgentProject, terms: &[String]) -> bool {
        Self::agent_sidebar_query_matches_text(
            format!("{} {}", project.name, project.root_path).as_str(),
            terms,
        )
    }

    fn agent_sidebar_thread_matches_terms(&self, thread: &AgentThread, terms: &[String]) -> bool {
        if terms.is_empty() {
            return true;
        }

        let mut haystack = vec![
            self.agent_thread_display_title(thread),
            thread.title.clone(),
            thread.agent.title().to_string(),
            thread.agent.keywords().to_string(),
            thread.launch_command.clone(),
            thread.working_dir.clone(),
        ];
        if let Some(custom_title) = thread.custom_title.as_deref() {
            haystack.push(custom_title.to_string());
        }
        if let Some(title) = thread.last_seen_title.as_deref() {
            haystack.push(title.to_string());
        }
        if let Some(command) = thread.last_seen_command.as_deref() {
            haystack.push(command.to_string());
        }
        if let Some(label) = thread.last_status_label.as_deref() {
            haystack.push(label.to_string());
        }
        if let Some(detail) = thread.last_status_detail.as_deref() {
            haystack.push(detail.to_string());
        }

        Self::agent_sidebar_query_matches_text(haystack.join("\n").as_str(), terms)
    }

    fn agent_sidebar_thread_matches_filter(
        &self,
        project: &AgentProject,
        thread: &AgentThread,
    ) -> bool {
        match self.agent_sidebar_filter {
            AgentSidebarFilter::All => true,
            AgentSidebarFilter::Live => self.agent_thread_has_live_session(thread),
            AgentSidebarFilter::Saved => !self.agent_thread_has_live_session(thread),
            AgentSidebarFilter::Busy => {
                matches!(
                    self.agent_thread_runtime_status(thread),
                    AgentThreadRuntimeStatus::Busy
                )
            }
            AgentSidebarFilter::Pinned => project.pinned || thread.pinned,
        }
    }

    fn filtered_agent_projects_for_sidebar(&self) -> Vec<(&AgentProject, Vec<&AgentThread>)> {
        let terms = self.agent_sidebar_search_terms();

        self.sorted_agent_projects()
            .into_iter()
            .filter_map(|project| {
                let project_matches = Self::agent_sidebar_project_matches_terms(project, &terms);
                let mut threads = self.sorted_agent_threads_for_project(project.id.as_str());

                threads.retain(|thread| self.agent_sidebar_thread_matches_filter(project, thread));

                if !terms.is_empty() && !project_matches {
                    threads
                        .retain(|thread| self.agent_sidebar_thread_matches_terms(thread, &terms));
                }

                let project_matches_filter = match self.agent_sidebar_filter {
                    AgentSidebarFilter::Pinned => project.pinned,
                    AgentSidebarFilter::All => true,
                    _ => false,
                };

                if ((!terms.is_empty() && !project_matches) || !project_matches_filter)
                    && threads.is_empty()
                {
                    return None;
                }

                Some((project, threads))
            })
            .collect()
    }

    pub(super) fn open_first_matching_agent_thread(&mut self, cx: &mut Context<Self>) {
        let Some(thread_id) = self
            .filtered_agent_projects_for_sidebar()
            .into_iter()
            .flat_map(|(_, threads)| threads.into_iter())
            .map(|thread| thread.id.clone())
            .next()
        else {
            return;
        };

        let linked_tab_id = self
            .agent_threads
            .iter()
            .find(|thread| thread.id == thread_id)
            .and_then(|thread| thread.linked_tab_id);

        if let Some(tab_index) = linked_tab_id.and_then(|tab_id| self.tab_index_by_id(tab_id)) {
            self.switch_tab(tab_index, cx);
        } else if let Err(error) = self.resume_saved_agent_thread(thread_id.as_str(), cx) {
            termy_toast::error(error);
            self.notify_overlay(cx);
            return;
        }

        self.agent_sidebar_search_active = false;
        cx.notify();
    }

    fn agent_thread_relative_age(updated_at_ms: u64) -> String {
        let now = now_unix_ms();
        let elapsed_seconds = now
            .saturating_sub(updated_at_ms)
            .checked_div(1000)
            .unwrap_or_default();

        match elapsed_seconds {
            0..=59 => "now".to_string(),
            60..=3599 => format!("{}m", elapsed_seconds / 60),
            3600..=86_399 => format!("{}h", elapsed_seconds / 3600),
            86_400..=604_799 => format!("{}d", elapsed_seconds / 86_400),
            604_800..=2_592_000 => format!("{}w", elapsed_seconds / 604_800),
            _ => format!("{}mo", elapsed_seconds / 2_592_000),
        }
    }

    fn compact_agent_thread_detail(
        status: &AgentThreadStatusPresentation,
        is_active: bool,
    ) -> Option<String> {
        match status.tone {
            AgentThreadStatusTone::Error | AgentThreadStatusTone::Warning => {
                status.detail.clone().or_else(|| Some(status.label.clone()))
            }
            AgentThreadStatusTone::Active if is_active => match status.label.as_str() {
                "thinking" => Some("Thinking".to_string()),
                "tool" => Some("Using tools".to_string()),
                "approval" => Some("Approval required".to_string()),
                "starting" => Some("Starting".to_string()),
                _ => None,
            },
            AgentThreadStatusTone::Muted => None,
            AgentThreadStatusTone::Active => None,
        }
    }

    fn render_agent_project_glyph(stroke: gpui::Rgba, bg: gpui::Rgba) -> AnyElement {
        div()
            .relative()
            .flex_none()
            .w(px(12.0))
            .h(px(10.0))
            .child(
                div()
                    .absolute()
                    .left(px(1.0))
                    .top(px(1.0))
                    .w(px(4.0))
                    .h(px(2.0))
                    .bg(bg)
                    .border_1()
                    .border_color(stroke),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(2.0))
                    .w(px(11.0))
                    .h(px(7.0))
                    .bg(bg)
                    .border_1()
                    .border_color(stroke),
            )
            .into_any_element()
    }

    fn render_agent_sidebar_avatar(
        agent: command_palette::AiAgentPreset,
        dark_surface: bool,
        border: gpui::Rgba,
        bg: gpui::Rgba,
        text: gpui::Rgba,
    ) -> AnyElement {
        let fallback_label = agent.fallback_label();
        div()
            .flex_none()
            .size(px(14.0))
            .p(px(1.0))
            .bg(bg)
            .border_1()
            .border_color(border)
            .child(
                img(Path::new(agent.image_asset_path(dark_surface)))
                    .size_full()
                    .object_fit(ObjectFit::Contain)
                    .with_fallback(move || {
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(7.0))
                            .text_color(text)
                            .child(fallback_label)
                            .into_any_element()
                    }),
            )
            .into_any_element()
    }

    fn render_agent_status_badge(
        label: &str,
        tone: AgentThreadStatusTone,
        border: gpui::Rgba,
        bg: gpui::Rgba,
        text: gpui::Rgba,
        muted: gpui::Rgba,
        warning: gpui::Rgba,
        error: gpui::Rgba,
    ) -> AnyElement {
        let badge_text = match tone {
            AgentThreadStatusTone::Active => text,
            AgentThreadStatusTone::Warning => warning,
            AgentThreadStatusTone::Error => error,
            AgentThreadStatusTone::Muted => muted,
        };

        div()
            .flex_none()
            .h(px(14.0))
            .px(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_size(px(9.0))
            .text_color(badge_text)
            .child(label.to_ascii_lowercase())
            .into_any_element()
    }

    fn render_agent_sidebar_chip(
        label: impl Into<SharedString>,
        border: gpui::Rgba,
        bg: gpui::Rgba,
        text: gpui::Rgba,
    ) -> AnyElement {
        let label: SharedString = label.into();
        div()
            .flex_none()
            .h(px(14.0))
            .px(px(4.0))
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_color(border)
            .bg(bg)
            .text_size(px(8.5))
            .text_color(text)
            .child(label)
            .into_any_element()
    }

    fn render_agent_activity_dot(color: gpui::Rgba) -> AnyElement {
        div()
            .flex_none()
            .size(px(5.0))
            .rounded(px(2.5))
            .bg(color)
            .into_any_element()
    }

    fn render_agent_sidebar_new_session_icon(stroke: gpui::Rgba, bg: gpui::Rgba) -> AnyElement {
        div()
            .relative()
            .flex_none()
            .w(px(17.0))
            .h(px(14.0))
            .child(
                div()
                    .absolute()
                    .left(px(1.0))
                    .top(px(2.0))
                    .w(px(5.0))
                    .h(px(3.0))
                    .bg(bg)
                    .border_1()
                    .border_color(stroke),
            )
            .child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(4.0))
                    .w(px(11.0))
                    .h(px(8.0))
                    .bg(bg)
                    .border_1()
                    .border_color(stroke),
            )
            .child(
                div()
                    .absolute()
                    .right(px(1.0))
                    .top(px(3.0))
                    .w(px(1.5))
                    .h(px(7.0))
                    .bg(stroke),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top(px(6.0))
                    .w(px(5.0))
                    .h(px(1.5))
                    .bg(stroke),
            )
            .into_any_element()
    }

    fn render_agent_sidebar_hide_icon(stroke: gpui::Rgba) -> AnyElement {
        div()
            .relative()
            .flex_none()
            .w(px(15.0))
            .h(px(12.0))
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top(px(1.0))
                    .w(px(12.0))
                    .h(px(1.5))
                    .bg(stroke),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top(px(5.0))
                    .w(px(9.0))
                    .h(px(1.5))
                    .bg(stroke),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .top(px(9.0))
                    .w(px(6.0))
                    .h(px(1.5))
                    .bg(stroke),
            )
            .into_any_element()
    }

    pub(super) fn render_agent_git_panel(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.agent_git_panel.open {
            return None;
        }

        let overlay_style = self.overlay_style();
        let panel_bg = overlay_style.chrome_panel_background_with_floor(0.96, 0.88);
        let input_bg = overlay_style.chrome_panel_background_with_floor(0.74, 0.72);
        let selected_bg = overlay_style.panel_cursor(0.10);
        let text = overlay_style.panel_foreground(0.94);
        let muted = overlay_style.panel_foreground(0.62);
        let border = resolve_chrome_stroke_color(
            panel_bg,
            self.colors.foreground,
            self.chrome_contrast_profile().stroke_mix,
        );
        let success = self.colors.ansi[10];
        let warning = self.colors.ansi[11];
        let danger = self.colors.ansi[9];
        let info = self.colors.ansi[12];
        let entries = self.agent_git_entries_for_filter();
        let has_entries = !entries.is_empty();
        let label = self
            .agent_git_panel
            .label
            .clone()
            .unwrap_or_else(|| "Git Changes".to_string());
        let repo_root = self.agent_git_panel.repo_root.clone();
        let repo_name = repo_root
            .as_deref()
            .and_then(|path| Path::new(path).file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| label.clone());
        let branch = self.agent_git_panel.branch.clone();
        let current_branch = self.agent_git_panel.current_branch.clone();
        let current_branch_badge = current_branch.clone();
        let current_branch_for_branches = current_branch.clone();
        let ahead = self.agent_git_panel.ahead;
        let behind = self.agent_git_panel.behind;
        let dirty_count = self.agent_git_panel.dirty_count;
        let last_commit = self.agent_git_panel.last_commit.clone();
        let loading = self.agent_git_panel.loading;
        let error = self.agent_git_panel.error.clone();
        let selected_repo_path = self.agent_git_panel.selected_repo_path.clone();
        let preview_loading = self.agent_git_panel.preview_loading;
        let preview_error = self.agent_git_panel.preview_error.clone();
        let preview_diff_lines = self.agent_git_panel.preview_diff_lines.clone();
        let preview_history = self.agent_git_panel.preview_history.clone();
        let project_history = self.agent_git_panel.project_history.clone();
        let branches = self.agent_git_panel.branches.clone();
        let stashes = self.agent_git_panel.stashes.clone();
        let input_mode = self.agent_git_panel_input_mode;
        let input_text = self.agent_git_panel_input.text().to_string();
        let input_bar = input_mode.map(|mode| {
            div()
                .px(px(10.0))
                .pb(px(8.0))
                .flex_none()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .text_size(px(10.5))
                        .text_color(muted)
                        .child(mode.title()),
                )
                .child(
                    div()
                        .relative()
                        .h(px(28.0))
                        .px(px(8.0))
                        .flex()
                        .items_center()
                        .border_1()
                        .border_color(border)
                        .bg(input_bg)
                        .children((input_text.trim().is_empty()).then(|| {
                            div()
                                .truncate()
                                .text_size(px(11.0))
                                .text_color(muted)
                                .child(mode.placeholder())
                                .into_any_element()
                        }))
                        .child(self.render_inline_input_layer(
                            Font {
                                family: self.font_family.clone(),
                                weight: FontWeight::NORMAL,
                                ..Default::default()
                            },
                            px(11.0),
                            text.into(),
                            selected_bg.into(),
                            InlineInputAlignment::Left,
                            cx,
                        )),
                )
                .child(
                    div()
                        .flex()
                        .gap(px(4.0))
                        .child(
                            div()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _event, _window, cx| {
                                        view.commit_agent_git_panel_input(cx);
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    mode.action_label(),
                                    border,
                                    input_bg,
                                    text,
                                )),
                        )
                        .child(
                            div()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _event, _window, cx| {
                                        view.cancel_agent_git_panel_input(cx);
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    "cancel", border, input_bg, muted,
                                )),
                        ),
                )
                .into_any_element()
        });

        let body = if loading {
            div()
                .px(px(10.0))
                .py(px(10.0))
                .text_size(px(11.0))
                .text_color(muted)
                .child("Loading git changes...")
                .into_any_element()
        } else if let Some(error) = error {
            div()
                .px(px(10.0))
                .py(px(10.0))
                .text_size(px(11.0))
                .text_color(muted)
                .child(error)
                .into_any_element()
        } else {
            let file_rows = entries
                .into_iter()
                .map(|entry| {
                    let repo_path = entry.repo_path.clone();
                    let row_repo_path = repo_path.clone();
                    let is_selected = selected_repo_path.as_deref() == Some(repo_path.as_str());
                    let status_color = if entry.is_untracked() || entry.status.contains('A') {
                        success
                    } else if entry.status.contains('D') || entry.status.contains('U') {
                        danger
                    } else if entry.status.contains('R') {
                        info
                    } else {
                        warning
                    };
                    let preview = is_selected.then(|| {
                        let open_repo_path = repo_path.clone();
                        let diff_repo_path = repo_path.clone();
                        let stage_repo_path = repo_path.clone();
                        let unstage_repo_path = repo_path.clone();
                        let discard_entry = entry.clone();
                        let preview_body = if preview_loading {
                            div()
                                .text_size(px(10.5))
                                .text_color(muted)
                                .child("Loading diff preview...")
                                .into_any_element()
                        } else if let Some(error) = preview_error.clone() {
                            div()
                                .text_size(px(10.5))
                                .text_color(muted)
                                .child(error)
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(6.0))
                                .child(
                                    div()
                                        .flex()
                                        .flex_wrap()
                                        .gap(px(4.0))
                                        .child(
                                            div()
                                                .cursor_pointer()
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(
                                                        move |view, _event, _window, cx| {
                                                            match view.open_agent_git_file(
                                                                open_repo_path.as_str(),
                                                            ) {
                                                                Ok(()) => {
                                                                    termy_toast::success(
                                                                        "Opened file",
                                                                    );
                                                                    view.notify_overlay(cx);
                                                                }
                                                                Err(error) => {
                                                                    termy_toast::error(error);
                                                                    view.notify_overlay(cx);
                                                                }
                                                            }
                                                            cx.stop_propagation();
                                                        },
                                                    ),
                                                )
                                                .child(Self::render_agent_sidebar_chip(
                                                    "open", border, input_bg, text,
                                                )),
                                        )
                                        .child(
                                            div()
                                                .cursor_pointer()
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(
                                                        move |view, _event, _window, cx| {
                                                            match view.open_agent_git_full_diff(
                                                                diff_repo_path.as_str(),
                                                                cx,
                                                            ) {
                                                                Ok(()) => {
                                                                    termy_toast::success(
                                                                        "Opened diff tab",
                                                                    );
                                                                    view.notify_overlay(cx);
                                                                }
                                                                Err(error) => {
                                                                    termy_toast::error(error);
                                                                    view.notify_overlay(cx);
                                                                }
                                                            }
                                                            cx.stop_propagation();
                                                        },
                                                    ),
                                                )
                                                .child(Self::render_agent_sidebar_chip(
                                                    "full diff",
                                                    border,
                                                    input_bg,
                                                    text,
                                                )),
                                        )
                                        .children(
                                            (entry.is_untracked() || entry.is_unstaged()).then(
                                                || {
                                                    div()
                                                        .cursor_pointer()
                                                        .on_mouse_down(
                                                            MouseButton::Left,
                                                            cx.listener(
                                                                move |view, _event, _window, cx| {
                                                                    view.run_agent_git_mutation(
                                                                        vec![
                                                                            "add".to_string(),
                                                                            "--".to_string(),
                                                                            stage_repo_path.clone(),
                                                                        ],
                                                                        "Staged file",
                                                                        cx,
                                                                    );
                                                                    cx.stop_propagation();
                                                                },
                                                            ),
                                                        )
                                                        .child(Self::render_agent_sidebar_chip(
                                                            "stage", border, input_bg, success,
                                                        ))
                                                        .into_any_element()
                                                },
                                            ),
                                        )
                                        .children(entry.is_staged().then(|| {
                                            div()
                                                .cursor_pointer()
                                                .on_mouse_down(
                                                    MouseButton::Left,
                                                    cx.listener(
                                                        move |view, _event, _window, cx| {
                                                            view.run_agent_git_mutation(
                                                                vec![
                                                                    "restore".to_string(),
                                                                    "--staged".to_string(),
                                                                    "--".to_string(),
                                                                    unstage_repo_path.clone(),
                                                                ],
                                                                "Unstaged file",
                                                                cx,
                                                            );
                                                            cx.stop_propagation();
                                                        },
                                                    ),
                                                )
                                                .child(Self::render_agent_sidebar_chip(
                                                    "unstage", border, input_bg, warning,
                                                ))
                                                .into_any_element()
                                        }))
                                        .children(
                                            (entry.is_untracked()
                                                || entry.is_unstaged()
                                                || entry.is_deleted())
                                            .then(|| {
                                                div()
                                                    .cursor_pointer()
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(
                                                            move |view, _event, _window, cx| {
                                                                view.discard_agent_git_entry(
                                                                    discard_entry.clone(),
                                                                    cx,
                                                                );
                                                                cx.stop_propagation();
                                                            },
                                                        ),
                                                    )
                                                    .child(Self::render_agent_sidebar_chip(
                                                        "discard", border, input_bg, danger,
                                                    ))
                                                    .into_any_element()
                                            }),
                                        ),
                                )
                                .children((!preview_diff_lines.is_empty()).then(|| {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(1.0))
                                        .children(preview_diff_lines.iter().take(60).map(|line| {
                                            let tone = if line.starts_with('+')
                                                && !line.starts_with("+++")
                                            {
                                                success
                                            } else if line.starts_with('-')
                                                && !line.starts_with("---")
                                            {
                                                danger
                                            } else if line.starts_with("@@") {
                                                warning
                                            } else {
                                                muted
                                            };
                                            div()
                                                .truncate()
                                                .text_size(px(10.0))
                                                .text_color(tone)
                                                .child(line.clone())
                                                .into_any_element()
                                        }))
                                        .into_any_element()
                                }))
                                .children((preview_history.is_empty()).then(|| {
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(muted)
                                        .child("No file history yet.")
                                        .into_any_element()
                                }))
                                .children((!preview_history.is_empty()).then(|| {
                                    div()
                                        .flex()
                                        .flex_col()
                                        .gap(px(3.0))
                                        .child(
                                            div()
                                                .text_size(px(10.0))
                                                .text_color(muted)
                                                .child("History"),
                                        )
                                        .children(preview_history.iter().take(6).map(|entry| {
                                            div()
                                                .truncate()
                                                .text_size(px(10.0))
                                                .text_color(text)
                                                .child(entry.summary.clone())
                                                .into_any_element()
                                        }))
                                        .into_any_element()
                                }))
                                .into_any_element()
                        };

                        div()
                            .mx(px(10.0))
                            .mb(px(8.0))
                            .px(px(8.0))
                            .py(px(8.0))
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            .border_1()
                            .border_color(border)
                            .bg(selected_bg)
                            .child(preview_body)
                            .into_any_element()
                    });

                    div()
                        .w_full()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .w_full()
                                .px(px(10.0))
                                .py(px(6.0))
                                .flex()
                                .items_start()
                                .gap(px(6.0))
                                .bg(if is_selected { selected_bg } else { panel_bg })
                                .cursor_pointer()
                                .border_b_1()
                                .border_color(border)
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |view, _event, _window, cx| {
                                        view.select_agent_git_panel_entry(
                                            row_repo_path.as_str(),
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    entry.badge_label(),
                                    border,
                                    input_bg,
                                    status_color,
                                ))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .flex()
                                        .flex_col()
                                        .gap(px(2.0))
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(11.0))
                                                .text_color(text)
                                                .child(entry.path.clone()),
                                        )
                                        .child(
                                            div()
                                                .truncate()
                                                .text_size(px(10.0))
                                                .text_color(muted)
                                                .child(entry.status.clone()),
                                        ),
                                ),
                        )
                        .children(preview)
                        .into_any_element()
                })
                .collect::<Vec<_>>();

            div()
                .w_full()
                .flex()
                .flex_col()
                .children((!has_entries).then(|| {
                    div()
                        .px(px(10.0))
                        .py(px(10.0))
                        .text_size(px(11.0))
                        .text_color(muted)
                        .child("No files match the current filter.")
                        .into_any_element()
                }))
                .children(file_rows)
                .children(
                    (selected_repo_path.is_none() && !project_history.is_empty()).then(|| {
                        div()
                            .px(px(10.0))
                            .py(px(8.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(muted)
                                    .child("Recent commits"),
                            )
                            .children(project_history.iter().take(8).map(|entry| {
                                div()
                                    .truncate()
                                    .text_size(px(10.5))
                                    .text_color(text)
                                    .child(entry.summary.clone())
                                    .into_any_element()
                            }))
                            .into_any_element()
                    }),
                )
                .into_any_element()
        };

        Some(
            div()
                .id("agent-git-panel")
                .w(px(AGENT_GIT_PANEL_WIDTH))
                .h_full()
                .flex_none()
                .flex()
                .flex_col()
                .bg(panel_bg)
                .border_l_1()
                .border_color(border)
                .child(
                    div()
                        .px(px(10.0))
                        .py(px(8.0))
                        .flex_none()
                        .flex()
                        .justify_between()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex_1()
                                .min_w(px(0.0))
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(11.0))
                                        .text_color(text)
                                        .child("Git Changes"),
                                )
                                .child(
                                    div()
                                        .truncate()
                                        .text_size(px(10.0))
                                        .text_color(muted)
                                        .child(label),
                                ),
                        )
                        .child(
                            div()
                                .flex()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|view, _event, _window, cx| {
                                                view.refresh_agent_git_panel(cx);
                                                cx.stop_propagation();
                                            }),
                                        )
                                        .child(Self::render_agent_sidebar_chip(
                                            "refresh", border, input_bg, text,
                                        )),
                                )
                                .child(
                                    div()
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|view, _event, _window, cx| {
                                                view.close_agent_git_panel();
                                                cx.notify();
                                                cx.stop_propagation();
                                            }),
                                        )
                                        .child(Self::render_agent_sidebar_chip(
                                            "hide", border, input_bg, muted,
                                        )),
                                ),
                        ),
                )
                .child(
                    div()
                        .px(px(10.0))
                        .pb(px(8.0))
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .truncate()
                                .text_size(px(11.0))
                                .text_color(text)
                                .child(repo_name),
                        )
                        .children(branch.map(|branch| {
                            div()
                                .truncate()
                                .text_size(px(10.0))
                                .text_color(muted)
                                .child(branch)
                                .into_any_element()
                        }))
                        .children(last_commit.map(|commit| {
                            div()
                                .truncate()
                                .text_size(px(10.0))
                                .text_color(muted)
                                .child(commit)
                                .into_any_element()
                        }))
                        .child(
                            div()
                                .flex()
                                .flex_wrap()
                                .gap(px(4.0))
                                .children(current_branch_badge.map(|branch| {
                                    Self::render_agent_sidebar_chip(branch, border, input_bg, text)
                                        .into_any_element()
                                }))
                                .children((ahead > 0).then(|| {
                                    Self::render_agent_sidebar_chip(
                                        format!("ahead {}", ahead),
                                        border,
                                        input_bg,
                                        success,
                                    )
                                    .into_any_element()
                                }))
                                .children((behind > 0).then(|| {
                                    Self::render_agent_sidebar_chip(
                                        format!("behind {}", behind),
                                        border,
                                        input_bg,
                                        warning,
                                    )
                                    .into_any_element()
                                }))
                                .children((dirty_count > 0).then(|| {
                                    Self::render_agent_sidebar_chip(
                                        format!("dirty {}", dirty_count),
                                        border,
                                        input_bg,
                                        danger,
                                    )
                                    .into_any_element()
                                })),
                        ),
                )
                .child(
                    div()
                        .px(px(10.0))
                        .pb(px(6.0))
                        .flex_none()
                        .flex()
                        .flex_wrap()
                        .gap(px(4.0))
                        .children(AgentGitPanelFilter::ALL.into_iter().map(|filter| {
                            let is_selected = self.agent_git_panel.filter == filter;
                            div()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |view, _event, _window, cx| {
                                        view.set_agent_git_panel_filter(filter, cx);
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    filter.label(),
                                    border,
                                    if is_selected { selected_bg } else { input_bg },
                                    if is_selected { text } else { muted },
                                ))
                                .into_any_element()
                        })),
                )
                .child(
                    div()
                        .px(px(10.0))
                        .pb(px(8.0))
                        .flex_none()
                        .flex()
                        .flex_wrap()
                        .gap(px(4.0))
                        .child(
                            div()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _event, _window, cx| {
                                        view.run_agent_git_mutation(
                                            vec!["add".to_string(), "-A".to_string()],
                                            "Staged all changes",
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    "stage all",
                                    border,
                                    input_bg,
                                    success,
                                )),
                        )
                        .child(
                            div()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _event, _window, cx| {
                                        view.run_agent_git_mutation(
                                            vec![
                                                "restore".to_string(),
                                                "--staged".to_string(),
                                                ".".to_string(),
                                            ],
                                            "Unstaged all changes",
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    "unstage all",
                                    border,
                                    input_bg,
                                    warning,
                                )),
                        )
                        .child(
                            div()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _event, _window, cx| {
                                        view.begin_agent_git_panel_input(
                                            AgentGitPanelInputMode::Commit,
                                            "",
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    "commit", border, input_bg, text,
                                )),
                        )
                        .child(
                            div()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _event, _window, cx| {
                                        view.begin_agent_git_panel_input(
                                            AgentGitPanelInputMode::CreateBranch,
                                            "",
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    "new branch",
                                    border,
                                    input_bg,
                                    info,
                                )),
                        )
                        .child(
                            div()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _event, _window, cx| {
                                        view.begin_agent_git_panel_input(
                                            AgentGitPanelInputMode::SaveStash,
                                            "",
                                            cx,
                                        );
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    "save stash",
                                    border,
                                    input_bg,
                                    muted,
                                )),
                        ),
                )
                .children(input_bar)
                .children((!branches.is_empty()).then(|| {
                    div()
                        .px(px(10.0))
                        .pb(px(8.0))
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(
                            div()
                                .text_size(px(10.0))
                                .text_color(muted)
                                .child("Branches"),
                        )
                        .child(div().flex().flex_wrap().gap(px(4.0)).children(
                            branches.into_iter().map(|branch_name| {
                                let is_current = current_branch_for_branches.as_deref()
                                    == Some(branch_name.as_str());
                                if is_current {
                                    Self::render_agent_sidebar_chip(
                                        branch_name,
                                        border,
                                        selected_bg,
                                        text,
                                    )
                                    .into_any_element()
                                } else {
                                    let checkout_branch = branch_name.clone();
                                    div()
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |view, _event, _window, cx| {
                                                view.run_agent_git_mutation(
                                                    vec![
                                                        "checkout".to_string(),
                                                        checkout_branch.clone(),
                                                    ],
                                                    "Switched branch",
                                                    cx,
                                                );
                                                cx.stop_propagation();
                                            }),
                                        )
                                        .child(Self::render_agent_sidebar_chip(
                                            branch_name,
                                            border,
                                            input_bg,
                                            muted,
                                        ))
                                        .into_any_element()
                                }
                            }),
                        ))
                        .into_any_element()
                }))
                .children((!stashes.is_empty()).then(|| {
                    div()
                        .px(px(10.0))
                        .pb(px(8.0))
                        .flex_none()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .child(div().text_size(px(10.0)).text_color(muted).child("Stashes"))
                        .children(stashes.into_iter().take(5).map(|stash| {
                            let apply_name = stash.name.clone();
                            let pop_name = stash.name.clone();
                            div()
                                .flex()
                                .items_center()
                                .gap(px(4.0))
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .truncate()
                                        .text_size(px(10.0))
                                        .text_color(text)
                                        .child(format!("{} {}", stash.name, stash.summary)),
                                )
                                .child(
                                    div()
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |view, _event, _window, cx| {
                                                view.run_agent_git_mutation(
                                                    vec![
                                                        "stash".to_string(),
                                                        "apply".to_string(),
                                                        apply_name.clone(),
                                                    ],
                                                    "Applied stash",
                                                    cx,
                                                );
                                                cx.stop_propagation();
                                            }),
                                        )
                                        .child(Self::render_agent_sidebar_chip(
                                            "apply", border, input_bg, text,
                                        )),
                                )
                                .child(
                                    div()
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |view, _event, _window, cx| {
                                                view.run_agent_git_mutation(
                                                    vec![
                                                        "stash".to_string(),
                                                        "pop".to_string(),
                                                        pop_name.clone(),
                                                    ],
                                                    "Popped stash",
                                                    cx,
                                                );
                                                cx.stop_propagation();
                                            }),
                                        )
                                        .child(Self::render_agent_sidebar_chip(
                                            "pop", border, input_bg, warning,
                                        )),
                                )
                                .into_any_element()
                        }))
                        .into_any_element()
                }))
                .child(
                    div()
                        .id("agent-git-panel-scroll")
                        .flex_1()
                        .overflow_y_scroll()
                        .child(body),
                )
                .into_any_element(),
        )
    }

    pub(super) fn render_agent_sidebar(&mut self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.should_render_agent_sidebar() {
            return None;
        }

        let overlay_style = self.overlay_style();
        let panel_bg = overlay_style.chrome_panel_background_with_floor(0.96, 0.88);
        let input_bg = overlay_style.chrome_panel_background_with_floor(0.74, 0.72);
        let transparent = overlay_style.transparent_background();
        let text = overlay_style.panel_foreground(0.94);
        let muted = overlay_style.panel_foreground(0.62);
        let border = resolve_chrome_stroke_color(
            panel_bg,
            self.colors.foreground,
            self.chrome_contrast_profile().stroke_mix,
        );
        let selected_bg = overlay_style.panel_cursor(0.10);
        let button_hover_bg = overlay_style.chrome_panel_cursor(0.14);
        let mut tooltip_bg = overlay_style.chrome_panel_background_with_floor(0.99, 0.94);
        tooltip_bg.a = 1.0;
        let tooltip_border = resolve_chrome_stroke_color(
            tooltip_bg,
            self.colors.foreground,
            self.chrome_contrast_profile().stroke_mix,
        );
        let tooltip_text = overlay_style.panel_foreground(0.98);
        let tooltip_muted = overlay_style.panel_foreground(0.74);
        let dark_surface = command_palette::AiAgentPreset::prefers_light_asset_variant(panel_bg);
        let active_thread_id = self
            .tabs
            .get(self.active_tab)
            .and_then(|tab| tab.agent_thread_id.as_deref())
            .map(str::to_string);
        let search_query = self.agent_sidebar_search_input.text().trim().to_string();
        let show_filtered_history = !search_query.is_empty();
        let has_non_default_filter = self.agent_sidebar_filter != AgentSidebarFilter::All;
        let filtered_projects = self.filtered_agent_projects_for_sidebar();
        let filtered_thread_count = filtered_projects
            .iter()
            .map(|(_, threads)| threads.len())
            .sum::<usize>();
        let history_thread_count = self.agent_threads.len();
        let history_summary = if show_filtered_history || has_non_default_filter {
            format!(
                "{} match{}",
                filtered_thread_count,
                if filtered_thread_count == 1 { "" } else { "es" }
            )
        } else {
            format!(
                "{} thread{}",
                history_thread_count,
                if history_thread_count == 1 { "" } else { "s" }
            )
        };
        let all_projects_collapsed = self.are_all_agent_projects_collapsed();
        let project_groups = filtered_projects
            .into_iter()
            .enumerate()
            .map(|(index, (project, project_threads))| {
                let project_id = project.id.clone();
                let project_context_menu_id = project.id.clone();
                let is_project_active =
                    self.active_agent_project_id.as_deref() == Some(project_id.as_str());
                let is_project_pinned = project.pinned;
                let is_renaming_project =
                    self.renaming_agent_project_id.as_deref() == Some(project_id.as_str());
                let allow_collapse_toggle = !show_filtered_history;
                let is_collapsed = allow_collapse_toggle
                    && self
                        .collapsed_agent_project_ids
                        .contains(project_id.as_str());

                let project_row = div()
                    .id(SharedString::from(format!("agent-project-{}", project.id)))
                    .w_full()
                    .h(px(AGENT_SIDEBAR_PROJECT_ROW_HEIGHT))
                    .px(px(10.0))
                    .mt(px(if index == 0 { 4.0 } else { 8.0 }))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_pointer()
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, _event, _window, cx| {
                            let was_active = view.active_agent_project_id.as_deref()
                                == Some(project_id.as_str());
                            if view.renaming_agent_project_id.as_deref()
                                != Some(project_id.as_str())
                            {
                                view.cancel_rename_agent_project(cx);
                            }
                            if view.renaming_agent_thread_id.is_some() {
                                view.cancel_rename_agent_thread(cx);
                            }
                            view.active_agent_project_id = Some(project_id.clone());
                            if allow_collapse_toggle && was_active {
                                view.toggle_agent_project_collapsed(project_id.as_str(), cx);
                            } else {
                                view.collapsed_agent_project_ids.remove(project_id.as_str());
                                view.sync_persisted_agent_workspace();
                                cx.notify();
                            }
                        }),
                    )
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |view, _event, _window, cx| {
                            view.schedule_agent_project_context_menu(
                                project_context_menu_id.clone(),
                                cx,
                            );
                            cx.stop_propagation();
                        }),
                    )
                    .child(
                        div()
                            .w(px(8.0))
                            .flex_none()
                            .text_size(px(8.0))
                            .text_color(muted)
                            .child(if is_collapsed { ">" } else { "v" }),
                    )
                    .child(Self::render_agent_project_glyph(
                        if is_project_active { text } else { muted },
                        panel_bg,
                    ))
                    .child(div().flex_1().min_w(px(0.0)).relative().h(px(16.0)).child(
                        if is_renaming_project {
                            self.render_inline_input_layer(
                                Font {
                                    family: self.font_family.clone(),
                                    weight: FontWeight::NORMAL,
                                    ..Default::default()
                                },
                                px(11.5),
                                text.into(),
                                selected_bg.into(),
                                InlineInputAlignment::Left,
                                cx,
                            )
                        } else {
                            div()
                                .truncate()
                                .text_size(px(11.5))
                                .text_color(if is_project_active { text } else { muted })
                                .child(project.name.clone())
                                .into_any_element()
                        },
                    ))
                    .children(is_project_pinned.then(|| {
                        Self::render_agent_sidebar_chip(
                            "pin",
                            border,
                            input_bg,
                            if is_project_active { text } else { muted },
                        )
                    }))
                    .into_any_element();

                let thread_rows = (show_filtered_history
                    || has_non_default_filter
                    || !is_collapsed)
                    .then_some(project_threads)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|thread| {
                        let thread_id = thread.id.clone();
                        let thread_context_menu_id = thread.id.clone();
                        let is_renaming_thread =
                            self.renaming_agent_thread_id.as_deref() == Some(thread_id.as_str());
                        let status = self.agent_thread_status_presentation(thread);
                        let is_active = active_thread_id.as_deref() == Some(thread_id.as_str());
                        let is_thread_pinned = thread.pinned;
                        let shows_activity = self.agent_thread_shows_activity(thread, is_active);
                        let title = self.agent_thread_display_title(thread);
                        let age = Self::agent_thread_relative_age(thread.updated_at_ms);
                        let detail = (!is_renaming_thread)
                            .then(|| {
                                Self::compact_agent_thread_detail(&status, is_active)
                                    .or_else(|| status.detail.clone())
                            })
                            .flatten();
                        let linked_tab_id = thread.linked_tab_id;

                        div()
                            .id(SharedString::from(format!("agent-thread-{}", thread.id)))
                            .w_full()
                            .px(px(10.0))
                            .py(px(if detail.is_some() || is_renaming_thread {
                                3.0
                            } else {
                                4.0
                            }))
                            .rounded(px(0.0))
                            .bg(if is_active || is_renaming_thread {
                                selected_bg
                            } else {
                                transparent
                            })
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |view, _event, _window, cx| {
                                    if view.renaming_agent_project_id.is_some() {
                                        view.cancel_rename_agent_project(cx);
                                    }
                                    view.agent_sidebar_search_active = false;
                                    if let Some(tab_index) = linked_tab_id
                                        .and_then(|tab_id| view.tab_index_by_id(tab_id))
                                    {
                                        view.switch_tab(tab_index, cx);
                                    } else if let Err(error) =
                                        view.resume_saved_agent_thread(thread_id.as_str(), cx)
                                    {
                                        termy_toast::error(error);
                                        view.notify_overlay(cx);
                                    }
                                    if view.renaming_agent_thread_id.as_deref()
                                        != Some(thread_id.as_str())
                                    {
                                        view.cancel_rename_agent_thread(cx);
                                    }
                                    cx.stop_propagation();
                                }),
                            )
                            .on_mouse_down(
                                MouseButton::Right,
                                cx.listener(move |view, _event, _window, cx| {
                                    view.schedule_agent_thread_context_menu(
                                        thread_context_menu_id.clone(),
                                        cx,
                                    );
                                    cx.stop_propagation();
                                }),
                            )
                            .child(
                                div()
                                    .w_full()
                                    .flex()
                                    .justify_between()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .flex_1()
                                            .min_w(px(0.0))
                                            .flex()
                                            .gap(px(6.0))
                                            .child(Self::render_agent_sidebar_avatar(
                                                thread.agent,
                                                dark_surface,
                                                border,
                                                input_bg,
                                                muted,
                                            ))
                                            .child(
                                                div()
                                                    .flex_1()
                                                    .min_w(px(0.0))
                                                    .flex()
                                                    .flex_col()
                                                    .gap(px(1.0))
                                                    .child(div().relative().h(px(15.0)).child(
                                                        if is_renaming_thread {
                                                            self.render_inline_input_layer(
                                                                Font {
                                                                    family: self
                                                                        .font_family
                                                                        .clone(),
                                                                    weight: FontWeight::NORMAL,
                                                                    ..Default::default()
                                                                },
                                                                px(12.0),
                                                                text.into(),
                                                                selected_bg.into(),
                                                                InlineInputAlignment::Left,
                                                                cx,
                                                            )
                                                        } else {
                                                            div()
                                                                .truncate()
                                                                .text_size(px(12.0))
                                                                .text_color(text)
                                                                .child(title)
                                                                .into_any_element()
                                                        },
                                                    ))
                                                    .child(
                                                        div()
                                                            .flex()
                                                            .items_center()
                                                            .gap(px(4.0))
                                                            .children(shows_activity.then(|| {
                                                                Self::render_agent_activity_dot(
                                                                    self.colors.ansi[11],
                                                                )
                                                            }))
                                                            .child(Self::render_agent_status_badge(
                                                                status.label.as_str(),
                                                                status.tone,
                                                                border,
                                                                input_bg,
                                                                text,
                                                                muted,
                                                                self.colors.ansi[11],
                                                                self.colors.ansi[9],
                                                            ))
                                                            .children(is_thread_pinned.then(|| {
                                                                Self::render_agent_sidebar_chip(
                                                                    "pin", border, input_bg, muted,
                                                                )
                                                            }))
                                                            .children(detail.map(|detail| {
                                                                div()
                                                                    .flex_1()
                                                                    .min_w(px(0.0))
                                                                    .truncate()
                                                                    .text_size(px(9.5))
                                                                    .text_color(muted)
                                                                    .child(detail)
                                                            })),
                                                    ),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .flex_none()
                                            .text_size(px(9.5))
                                            .text_color(muted)
                                            .child(age),
                                    ),
                            )
                            .into_any_element()
                    })
                    .collect::<Vec<_>>();

                div()
                    .w_full()
                    .flex()
                    .flex_col()
                    .child(project_row)
                    .children(thread_rows)
                    .into_any_element()
            })
            .collect::<Vec<_>>();

        let empty_state = project_groups.is_empty().then(|| {
            let message = if show_filtered_history {
                format!("No history matches \"{}\".", search_query)
            } else if has_non_default_filter {
                format!(
                    "No threads match the {} filter.",
                    self.agent_sidebar_filter.label().to_lowercase()
                )
            } else {
                "No threads yet. Start an agent to create a project.".to_string()
            };
            div()
                .px(px(10.0))
                .py(px(8.0))
                .text_size(px(11.0))
                .text_color(muted)
                .child(message)
                .into_any_element()
        });

        Some(
            div()
                .id("agent-sidebar")
                .relative()
                .w(px(self.agent_sidebar_width))
                .h_full()
                .flex_none()
                .flex()
                .flex_col()
                .bg(panel_bg)
                .border_r_1()
                .border_color(border)
                .child(
                    div()
                        .id("agent-sidebar-resize-handle")
                        .absolute()
                        .right(px(-4.0))
                        .top_0()
                        .bottom_0()
                        .w(px(8.0))
                        .cursor_col_resize()
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|view, _event: &MouseDownEvent, _window, cx| {
                                view.agent_sidebar_resize_drag =
                                    Some(AgentSidebarResizeDragState);
                                cx.stop_propagation();
                            }),
                        ),
                )
                .child(
                    div()
                        .h(px(AGENT_SIDEBAR_HEADER_HEIGHT))
                        .px(px(10.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(12.0))
                                .text_color(muted)
                                .child("Threads"),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(10.0))
                                .child(
                                    div()
                                        .id("agent-sidebar-new-thread")
                                        .w(px(20.0))
                                        .h(px(18.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(button_hover_bg))
                                        .tooltip(move |_window, cx| {
                                            cx.new(|_| {
                                                AgentSidebarTooltip::new(
                                                    "New thread",
                                                    "Open the agent picker and start a new thread.",
                                                    tooltip_bg,
                                                    tooltip_border,
                                                    tooltip_text,
                                                    tooltip_muted,
                                                )
                                            })
                                            .into()
                                        })
                                        .child(Self::render_agent_sidebar_new_session_icon(
                                            muted, panel_bg,
                                        ))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|view, _event, _window, cx| {
                                                view.open_command_palette_in_mode(
                                                    command_palette::CommandPaletteMode::AgentProjects,
                                                    cx,
                                                );
                                                cx.stop_propagation();
                                            }),
                                        ),
                                )
                                .child(
                                    div()
                                        .id("agent-sidebar-hide")
                                        .w(px(20.0))
                                        .h(px(18.0))
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(button_hover_bg))
                                        .tooltip(move |_window, cx| {
                                            cx.new(|_| {
                                                AgentSidebarTooltip::new(
                                                    "Hide Threads",
                                                    "Close the Threads sidebar.",
                                                    tooltip_bg,
                                                    tooltip_border,
                                                    tooltip_text,
                                                    tooltip_muted,
                                                )
                                            })
                                            .into()
                                        })
                                        .child(Self::render_agent_sidebar_hide_icon(muted))
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(|view, _event, _window, cx| {
                                                view.agent_sidebar_open = false;
                                                view.agent_sidebar_search_active = false;
                                                view.cancel_rename_agent_project(cx);
                                                view.cancel_rename_agent_thread(cx);
                                                view.hovered_agent_thread_id = None;
                                                view.close_agent_git_panel();
                                                view.sync_persisted_agent_workspace();
                                                cx.notify();
                                                cx.stop_propagation();
                                            }),
                                        ),
                                ),
                        ),
                )
                .child(
                    div()
                        .h(px(AGENT_SIDEBAR_SEARCH_HEIGHT))
                        .px(px(10.0))
                        .pb(px(4.0))
                        .flex_none()
                        .child(
                            div()
                                .id("agent-sidebar-search")
                                .relative()
                                .w_full()
                                .h_full()
                                .px(px(8.0))
                                .flex()
                                .items_center()
                                .border_1()
                                .border_color(border)
                                .bg(if self.agent_sidebar_search_active {
                                    selected_bg
                                } else {
                                    input_bg
                                })
                                .cursor(gpui::CursorStyle::IBeam)
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|view, _event, _window, cx| {
                                        view.begin_agent_sidebar_search(cx);
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(
                                    div()
                                        .relative()
                                        .flex_1()
                                        .h_full()
                                        .flex()
                                        .items_center()
                                        .children(
                                            (!self.agent_sidebar_search_active
                                                && self
                                                    .agent_sidebar_search_input
                                                    .text()
                                                    .trim()
                                                    .is_empty())
                                            .then(|| {
                                                div()
                                                    .truncate()
                                                    .text_size(px(11.0))
                                                    .text_color(muted)
                                                    .child("Search history")
                                                    .into_any_element()
                                            }),
                                        )
                                        .children(
                                            (!self.agent_sidebar_search_active
                                                && !self
                                                    .agent_sidebar_search_input
                                                    .text()
                                                    .trim()
                                                    .is_empty())
                                            .then(|| {
                                                div()
                                                    .truncate()
                                                    .text_size(px(11.0))
                                                    .text_color(text)
                                                    .child(
                                                        self.agent_sidebar_search_input
                                                            .text()
                                                            .to_string(),
                                                    )
                                                    .into_any_element()
                                            }),
                                        )
                                        .children(self.agent_sidebar_search_active.then(|| {
                                            self.render_inline_input_layer(
                                                Font {
                                                    family: self.font_family.clone(),
                                                    weight: FontWeight::NORMAL,
                                                    ..Default::default()
                                                },
                                                px(11.0),
                                                text.into(),
                                                selected_bg.into(),
                                                InlineInputAlignment::Left,
                                                cx,
                                            )
                                        })),
                                ),
                        ),
                )
                .child(
                    div()
                        .px(px(10.0))
                        .pb(px(2.0))
                        .flex_none()
                        .flex()
                        .flex_wrap()
                        .gap(px(4.0))
                        .children(AgentSidebarFilter::ALL.into_iter().map(|filter| {
                            let is_selected = self.agent_sidebar_filter == filter;
                            div()
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |view, _event, _window, cx| {
                                        view.set_agent_sidebar_filter(filter, cx);
                                        cx.stop_propagation();
                                    }),
                                )
                                .child(Self::render_agent_sidebar_chip(
                                    filter.label(),
                                    border,
                                    if is_selected { selected_bg } else { input_bg },
                                    if is_selected { text } else { muted },
                                ))
                                .into_any_element()
                        })),
                )
                .child(
                    div()
                        .px(px(10.0))
                        .pt(px(2.0))
                        .pb(px(2.0))
                        .flex_none()
                        .flex()
                        .justify_between()
                        .gap(px(6.0))
                        .child(
                            div()
                                .text_size(px(9.5))
                                .text_color(muted)
                                .child(if show_filtered_history {
                                    "Search Results"
                                } else {
                                    "History"
                                }),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(6.0))
                                .children((!show_filtered_history
                                    && !has_non_default_filter
                                    && !self.agent_projects.is_empty())
                                    .then(|| {
                                    div()
                                        .cursor_pointer()
                                        .on_mouse_down(
                                            MouseButton::Left,
                                            cx.listener(move |view, _event, _window, cx| {
                                                view.set_all_agent_projects_collapsed(!all_projects_collapsed, cx);
                                                cx.stop_propagation();
                                            }),
                                        )
                                        .child(Self::render_agent_sidebar_chip(
                                            if all_projects_collapsed { "expand" } else { "collapse" },
                                            border,
                                            input_bg,
                                            muted,
                                        ))
                                        .into_any_element()
                                }))
                                .child(
                                    div()
                                        .text_size(px(9.5))
                                        .text_color(muted)
                                        .child(history_summary),
                                ),
                        ),
                )
                .child(
                    div()
                        .id("agent-sidebar-scroll")
                        .flex_1()
                        .overflow_y_scroll()
                        .child(
                            div()
                                .w_full()
                                .pb(px(8.0))
                                .flex()
                                .flex_col()
                                .children(project_groups)
                                .children(empty_state),
                        ),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn lines(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    fn sample_workspace_state() -> PersistedAgentWorkspaceState {
        PersistedAgentWorkspaceState {
            version: AGENT_WORKSPACE_SCHEMA_VERSION,
            sidebar_open: true,
            active_project_id: Some("project-1".to_string()),
            collapsed_project_ids: vec!["project-2".to_string()],
            projects: vec![
                AgentProject {
                    id: "project-1".to_string(),
                    name: "termy".to_string(),
                    root_path: "/Users/lasse/dev/termy".to_string(),
                    pinned: true,
                    created_at_ms: 10,
                    updated_at_ms: 40,
                },
                AgentProject {
                    id: "project-2".to_string(),
                    name: "playground".to_string(),
                    root_path: "/Users/lasse/dev/playground".to_string(),
                    pinned: false,
                    created_at_ms: 20,
                    updated_at_ms: 30,
                },
            ],
            threads: vec![AgentThread {
                id: "thread-1".to_string(),
                project_id: "project-1".to_string(),
                agent: command_palette::AiAgentPreset::Codex,
                title: "Codex termy".to_string(),
                custom_title: Some("sqlite migration".to_string()),
                pinned: true,
                launch_command: "codex".to_string(),
                working_dir: "/Users/lasse/dev/termy".to_string(),
                last_seen_title: Some("sqlite migration".to_string()),
                last_seen_command: Some("cargo check".to_string()),
                last_status_label: Some("ready".to_string()),
                last_status_detail: Some("workspace synced".to_string()),
                created_at_ms: 11,
                updated_at_ms: 41,
                linked_tab_id: None,
            }],
        }
    }

    #[test]
    fn pi_status_detects_setup_when_no_model_is_available() {
        let status = TerminalView::detect_provider_status(
            command_palette::AiAgentPreset::Pi,
            &lines(&["/private/tmp", "0.0%/0 (auto) no-model"]),
        )
        .expect("status");

        assert_eq!(status.label, "setup");
        assert_eq!(status.detail.as_deref(), Some("0.0%/0 (auto) no-model"));
        assert_eq!(status.tone, AgentThreadStatusTone::Warning);
    }

    #[test]
    fn opencode_status_detects_ready_from_prompt_footer() {
        let status = TerminalView::detect_provider_status(
            command_palette::AiAgentPreset::OpenCode,
            &lines(&[
                "Ask anything... What is the tech stack of this project?",
                "Build Big Pickle OpenCode Zen",
                "/private/tmp 1.3.1",
            ]),
        )
        .expect("status");

        assert_eq!(status.label, "ready");
        assert_eq!(
            status.detail.as_deref(),
            Some("Build Big Pickle OpenCode Zen")
        );
        assert_eq!(status.tone, AgentThreadStatusTone::Active);
    }

    #[test]
    fn claude_status_detects_connectivity_failure() {
        let status = TerminalView::detect_provider_status(
            command_palette::AiAgentPreset::Claude,
            &lines(&[
                "Unable to connect to Anthropic services",
                "Failed to connect to api.anthropic.com: ECONNREFUSED",
            ]),
        )
        .expect("status");

        assert_eq!(status.label, "error");
        assert_eq!(
            status.detail.as_deref(),
            Some("Failed to connect to api.anthropic.com: ECONNREFUSED")
        );
        assert_eq!(status.tone, AgentThreadStatusTone::Error);
    }

    #[test]
    fn agent_workspace_sqlite_roundtrip_preserves_state() {
        let dir = tempdir().expect("tempdir");
        let path = dir.path().join("agents.sqlite3");
        let state = sample_workspace_state();

        let db = AgentWorkspaceDb::open(&path).expect("open db");
        store_agent_workspace_state_to_db(&db, &state).expect("store state");

        let loaded = load_agent_workspace_state_from_db(&db)
            .expect("load state")
            .expect("stored state");
        assert_eq!(loaded, state);
    }

    #[test]
    fn agent_workspace_migrates_legacy_json_into_sqlite() {
        let dir = tempdir().expect("tempdir");
        let db_path = dir.path().join("agents.sqlite3");
        let legacy_path = dir.path().join("agents.json");
        let state = sample_workspace_state();

        fs::write(
            &legacy_path,
            serde_json::to_string_pretty(&state).expect("serialize state"),
        )
        .expect("write legacy json");

        let db = AgentWorkspaceDb::open(&db_path).expect("open db");
        let loaded =
            load_or_migrate_agent_workspace_state(&db, &legacy_path).expect("migrate legacy state");
        assert_eq!(loaded, state);
    }

    #[test]
    fn agent_workspace_defaults_missing_pinned_flags_to_false() {
        let loaded = decode_agent_workspace_state(
            r#"{
  "version": 1,
  "sidebar_open": true,
  "active_project_id": "project-1",
  "collapsed_project_ids": [],
  "projects": [
    {
      "id": "project-1",
      "name": "termy",
      "root_path": "/Users/lasse/dev/termy",
      "created_at_ms": 10,
      "updated_at_ms": 40
    }
  ],
  "threads": [
    {
      "id": "thread-1",
      "project_id": "project-1",
      "agent": "codex",
      "title": "Codex termy",
      "launch_command": "codex",
      "working_dir": "/Users/lasse/dev/termy",
      "created_at_ms": 11,
      "updated_at_ms": 41
    }
  ]
}"#,
        )
        .expect("decode state");

        assert_eq!(loaded.projects.len(), 1);
        assert!(!loaded.projects[0].pinned);
        assert_eq!(loaded.threads.len(), 1);
        assert!(!loaded.threads[0].pinned);
    }
}
