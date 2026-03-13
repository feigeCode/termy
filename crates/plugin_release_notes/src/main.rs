use serde::Deserialize;
use termy_plugin_core::{HostRpcMessage, PluginCapability, PluginLogLevel, PluginToastLevel};
use termy_plugin_sdk::{PluginMetadata, PluginSession};

const RELEASE_NOTES_URL: &str = "https://termy.run/api/changelogs";
const VIEW_RELEASE_NOTES_COMMAND_ID: &str = "view.releaseNotes";

#[derive(Debug, Deserialize)]
struct ChangelogResponse {
    posts: Vec<ChangelogPost>,
}

#[derive(Debug, Deserialize)]
struct ChangelogPost {
    id: String,
    title: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

fn fetch_latest_changelog() -> Result<ChangelogPost, String> {
    let response = ureq::get(RELEASE_NOTES_URL)
        .call()
        .map_err(|error| format!("request failed: {error}"))?;

    let payload: ChangelogResponse = response
        .into_json()
        .map_err(|error| format!("invalid changelog payload: {error}"))?;

    payload
        .posts
        .into_iter()
        .next()
        .ok_or_else(|| "no changelog posts returned".to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metadata = PluginMetadata::new("lasse.view-release-notes", "Release Notes", "0.1.0")
        .with_capabilities(vec![PluginCapability::CommandProvider]);
    let mut session = PluginSession::stdio(metadata)?;

    session.run_until_shutdown(|message, session| {
        match message {
            HostRpcMessage::Ping => {
                session.send_pong()?;
            }
            HostRpcMessage::InvokeCommand(invocation) => {
                if invocation.command_id == VIEW_RELEASE_NOTES_COMMAND_ID {
                    match fetch_latest_changelog() {
                        Ok(post) => {
                            session.send_log(
                                PluginLogLevel::Info,
                                format!(
                                    "latest changelog: {} [{}] ({})",
                                    post.title, post.id, post.created_at
                                ),
                            )?;
                            session.send_toast(
                                PluginToastLevel::Success,
                                format!("Latest release notes: {}", post.title),
                                Some(2800),
                            )?;
                        }
                        Err(error) => {
                            session.send_log(
                                PluginLogLevel::Error,
                                format!("failed to fetch latest changelog: {error}"),
                            )?;
                            session.send_toast(
                                PluginToastLevel::Error,
                                "Could not fetch latest release notes",
                                Some(2800),
                            )?;
                        }
                    }
                }
            }
            HostRpcMessage::Event(_) | HostRpcMessage::Hello(_) | HostRpcMessage::Shutdown => {}
        }
        Ok(())
    })?;

    Ok(())
}
