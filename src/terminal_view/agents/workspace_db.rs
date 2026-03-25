use super::*;

pub(super) struct AgentWorkspaceDb {
    raw: *mut sqlite3::sqlite3,
}

struct AgentWorkspaceStatement<'db> {
    db: &'db AgentWorkspaceDb,
    raw: *mut sqlite3::sqlite3_stmt,
}

impl AgentWorkspaceDb {
    pub(super) fn open(path: &Path) -> Result<Self, String> {
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

pub(super) fn load_legacy_agent_workspace_state(
    path: &Path,
) -> Result<PersistedAgentWorkspaceState, String> {
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

pub(super) fn decode_agent_workspace_state(
    contents: &str,
) -> Result<PersistedAgentWorkspaceState, String> {
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

pub(super) fn load_agent_workspace_state_from_db(
    db: &AgentWorkspaceDb,
) -> Result<Option<PersistedAgentWorkspaceState>, String> {
    db.meta_value(AGENT_WORKSPACE_STATE_ROW_KEY)?
        .map(|contents| decode_agent_workspace_state(&contents))
        .transpose()
}

pub(super) fn store_agent_workspace_state_to_db(
    db: &AgentWorkspaceDb,
    state: &PersistedAgentWorkspaceState,
) -> Result<(), String> {
    let contents = serde_json::to_string_pretty(state)
        .map_err(|error| format!("Failed to encode agent workspace state: {}", error))?;
    db.set_meta_value(AGENT_WORKSPACE_STATE_ROW_KEY, &contents)
}

pub(super) fn load_or_migrate_agent_workspace_state(
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
