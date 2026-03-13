use termy_plugin_core::{
    HostEvent, HostRpcMessage, PluginCapability, PluginLogLevel, PluginPanelAction,
    PluginToastLevel,
};
use termy_plugin_sdk::{PluginMetadata, PluginSession, PluginSessionError};

const PLUGIN_ID: &str = "example.full";
const PLUGIN_NAME: &str = "Full Example Plugin";
const PLUGIN_VERSION: &str = "0.1.0";
const REFRESH_COMMAND_ID: &str = "example.full.refresh";
const TOAST_COMMAND_ID: &str = "example.full.toast";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ExampleState {
    host_version: Option<String>,
    theme_id: Option<String>,
    active_tab_index: Option<usize>,
    active_tab_title: Option<String>,
    refresh_count: usize,
    toast_count: usize,
    last_event: Option<String>,
}

impl ExampleState {
    fn apply_event(&mut self, event: &HostEvent) {
        match event {
            HostEvent::AppStarted { host_version } => {
                self.host_version = Some(host_version.clone());
                self.last_event = Some("app_started".to_string());
            }
            HostEvent::ThemeChanged { theme_id } => {
                self.theme_id = Some(theme_id.clone());
                self.last_event = Some("theme_changed".to_string());
            }
            HostEvent::ActiveTabChanged {
                tab_index,
                tab_title,
            } => {
                self.active_tab_index = Some(*tab_index);
                self.active_tab_title = Some(tab_title.clone());
                self.last_event = Some("active_tab_changed".to_string());
            }
        }
    }

    fn mark_refresh(&mut self) {
        self.refresh_count += 1;
    }

    fn mark_toast(&mut self) {
        self.toast_count += 1;
    }

    fn panel_body(&self) -> String {
        let host_version = self.host_version.as_deref().unwrap_or("unknown");
        let theme_id = self.theme_id.as_deref().unwrap_or("unknown");
        let active_tab = match (self.active_tab_index, self.active_tab_title.as_deref()) {
            (Some(index), Some(title)) => format!("#{index}: {title}"),
            (Some(index), None) => format!("#{index}"),
            _ => "unknown".to_string(),
        };
        let last_event = self.last_event.as_deref().unwrap_or("none");

        format!(
            "Host version: {host_version}\nTheme: {theme_id}\nActive tab: {active_tab}\nLast event: {last_event}\nRefreshes: {}\nToasts sent: {}",
            self.refresh_count, self.toast_count
        )
    }

    fn panel_actions(&self) -> Vec<PluginPanelAction> {
        vec![
            PluginPanelAction {
                command_id: REFRESH_COMMAND_ID.to_string(),
                label: "Refresh panel".to_string(),
                enabled: true,
            },
            PluginPanelAction {
                command_id: TOAST_COMMAND_ID.to_string(),
                label: "Send toast".to_string(),
                enabled: true,
            },
        ]
    }

    fn publish<R, W>(&self, session: &mut PluginSession<R, W>) -> Result<(), PluginSessionError>
    where
        R: std::io::Read,
        W: std::io::Write,
    {
        session.send_panel_with_actions(PLUGIN_NAME, self.panel_body(), self.panel_actions())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metadata =
        PluginMetadata::new(PLUGIN_ID, PLUGIN_NAME, PLUGIN_VERSION).with_capabilities(vec![
            PluginCapability::CommandProvider,
            PluginCapability::EventSubscriber,
            PluginCapability::UiPanel,
        ]);
    let mut session = PluginSession::stdio(metadata)?;
    let mut state = ExampleState::default();
    state.publish(&mut session)?;

    session.run_until_shutdown(|message, session| {
        match message {
            HostRpcMessage::Ping => {
                session.send_pong()?;
            }
            HostRpcMessage::InvokeCommand(invocation)
                if invocation.command_id == REFRESH_COMMAND_ID =>
            {
                state.mark_refresh();
                session.send_log(
                    PluginLogLevel::Info,
                    format!("refresh requested via `{REFRESH_COMMAND_ID}`"),
                )?;
                state.publish(session)?;
            }
            HostRpcMessage::InvokeCommand(invocation)
                if invocation.command_id == TOAST_COMMAND_ID =>
            {
                state.mark_toast();
                session.send_log(
                    PluginLogLevel::Info,
                    format!("toast requested via `{TOAST_COMMAND_ID}`"),
                )?;
                session.send_toast(
                    PluginToastLevel::Success,
                    format!("{PLUGIN_NAME} handled a panel action"),
                    Some(2200),
                )?;
                state.publish(session)?;
            }
            HostRpcMessage::InvokeCommand(invocation) => {
                session.send_log(
                    PluginLogLevel::Warn,
                    format!("unhandled command `{}`", invocation.command_id),
                )?;
            }
            HostRpcMessage::Event(event) => {
                state.apply_event(event);
                session.send_log(
                    PluginLogLevel::Info,
                    format!("observed host event `{:?}`", event.subscription()),
                )?;
                state.publish(session)?;
            }
            HostRpcMessage::Hello(_) | HostRpcMessage::Shutdown => {}
        }

        Ok(())
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_body_includes_runtime_state() {
        let mut state = ExampleState::default();
        state.apply_event(&HostEvent::AppStarted {
            host_version: "0.1.51".to_string(),
        });
        state.apply_event(&HostEvent::ThemeChanged {
            theme_id: "nord".to_string(),
        });
        state.apply_event(&HostEvent::ActiveTabChanged {
            tab_index: 1,
            tab_title: "server".to_string(),
        });
        state.mark_refresh();
        state.mark_toast();

        let body = state.panel_body();
        assert!(body.contains("Host version: 0.1.51"));
        assert!(body.contains("Theme: nord"));
        assert!(body.contains("Active tab: #1: server"));
        assert!(body.contains("Refreshes: 1"));
        assert!(body.contains("Toasts sent: 1"));
    }

    #[test]
    fn panel_actions_match_manifest_commands() {
        let actions = ExampleState::default().panel_actions();

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].command_id, REFRESH_COMMAND_ID);
        assert_eq!(actions[1].command_id, TOAST_COMMAND_ID);
        assert!(actions.iter().all(|action| action.enabled));
    }
}
