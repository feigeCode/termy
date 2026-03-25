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
            last_session_id: None,
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
