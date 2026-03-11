use termy_plugin_core::{
    HostEvent, HostRpcMessage, PluginCapability, PluginLogLevel, PluginPanelAction,
    PluginToastLevel,
};
use termy_plugin_sdk::{PluginMetadata, PluginSession, PluginSessionError};

const PLUGIN_ID: &str = "example.status";
const PLUGIN_NAME: &str = "Status Example";
const PLUGIN_VERSION: &str = "0.1.0";
const REFRESH_COMMAND_ID: &str = "example.status.refresh";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct StatusPanelState {
    host_version: Option<String>,
    theme_id: Option<String>,
    active_tab_index: Option<usize>,
    active_tab_title: Option<String>,
    refresh_count: usize,
}

impl StatusPanelState {
    fn apply_event(&mut self, event: &HostEvent) -> &'static str {
        match event {
            HostEvent::AppStarted { host_version } => {
                self.host_version = Some(host_version.clone());
                "app_started"
            }
            HostEvent::ThemeChanged { theme_id } => {
                self.theme_id = Some(theme_id.clone());
                "theme_changed"
            }
            HostEvent::ActiveTabChanged {
                tab_index,
                tab_title,
            } => {
                self.active_tab_index = Some(*tab_index);
                self.active_tab_title = Some(tab_title.clone());
                "active_tab_changed"
            }
        }
    }

    fn mark_manual_refresh(&mut self) {
        self.refresh_count += 1;
    }

    fn panel_body(&self) -> String {
        let host_version = self.host_version.as_deref().unwrap_or("unknown");
        let theme_id = self.theme_id.as_deref().unwrap_or("unknown");
        let active_tab = match (self.active_tab_index, self.active_tab_title.as_deref()) {
            (Some(index), Some(title)) => format!("#{index}: {title}"),
            (Some(index), None) => format!("#{index}"),
            _ => "unknown".to_string(),
        };

        format!(
            "Host version: {host_version}\nTheme: {theme_id}\nActive tab: {active_tab}\nManual refreshes: {}",
            self.refresh_count
        )
    }

    fn publish<R, W>(&self, session: &mut PluginSession<R, W>) -> Result<(), PluginSessionError>
    where
        R: std::io::Read,
        W: std::io::Write,
    {
        session.send_panel_with_actions(
            PLUGIN_NAME,
            self.panel_body(),
            vec![PluginPanelAction {
                command_id: REFRESH_COMMAND_ID.to_string(),
                label: "Refresh panel".to_string(),
                enabled: true,
            }],
        )
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
    let mut state = StatusPanelState::default();
    state.publish(&mut session)?;

    session.run_until_shutdown(|message, session| {
        match message {
            HostRpcMessage::Ping => {
                session.send_pong()?;
            }
            HostRpcMessage::InvokeCommand(invocation)
                if invocation.command_id == REFRESH_COMMAND_ID =>
            {
                state.mark_manual_refresh();
                session.send_log(
                    PluginLogLevel::Info,
                    format!("manual refresh requested for {REFRESH_COMMAND_ID}"),
                )?;
                session.send_toast(
                    PluginToastLevel::Success,
                    format!("{PLUGIN_NAME} refreshed"),
                    Some(2000),
                )?;
                state.publish(session)?;
            }
            HostRpcMessage::InvokeCommand(_) => {}
            HostRpcMessage::Event(event) => {
                let event_name = state.apply_event(event);
                session.send_log(
                    PluginLogLevel::Info,
                    format!("observed host event `{event_name}`"),
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
    fn panel_body_includes_latest_state() {
        let mut state = StatusPanelState::default();
        state.apply_event(&HostEvent::AppStarted {
            host_version: "0.1.51".to_string(),
        });
        state.apply_event(&HostEvent::ThemeChanged {
            theme_id: "nord".to_string(),
        });
        state.apply_event(&HostEvent::ActiveTabChanged {
            tab_index: 2,
            tab_title: "server".to_string(),
        });
        state.mark_manual_refresh();

        let panel = state.panel_body();
        assert!(panel.contains("Host version: 0.1.51"));
        assert!(panel.contains("Theme: nord"));
        assert!(panel.contains("Active tab: #2: server"));
        assert!(panel.contains("Manual refreshes: 1"));
    }

    #[test]
    fn apply_event_returns_event_name() {
        let mut state = StatusPanelState::default();

        assert_eq!(
            state.apply_event(&HostEvent::ThemeChanged {
                theme_id: "gruvbox".to_string(),
            }),
            "theme_changed"
        );
        assert_eq!(state.theme_id.as_deref(), Some("gruvbox"));
    }
}
