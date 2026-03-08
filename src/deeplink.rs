use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeepLinkRoute {
    Activate,
    NewTab,
    AuthCallback,
    Settings,
    OpenConfig,
    ThemeInstall,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NewTabDeepLink {
    pub(crate) command: Option<String>,
    pub(crate) dir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AuthCallbackDeepLink {
    pub(crate) session_token: String,
    pub(crate) user_id: Option<String>,
    pub(crate) avatar_url: Option<String>,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DeepLinkArgument {
    NewTab(NewTabDeepLink),
    AuthCallback(AuthCallbackDeepLink),
    Value(String),
}

impl DeepLinkRoute {
    pub(crate) fn parse(raw: &str) -> Result<(Self, Option<DeepLinkArgument>), String> {
        let url = Url::parse(raw).map_err(|error| format!("Invalid Termy deeplink: {error}"))?;

        if url.scheme() != "termy" {
            return Err(format!(
                "Unsupported deeplink scheme \"{}\"; expected termy://",
                url.scheme()
            ));
        }

        if !url.username().is_empty() || url.password().is_some() || url.port().is_some() {
            return Err("Termy deeplinks do not support user info or ports".to_string());
        }

        let mut segments = Vec::new();
        if let Some(host) = url.host_str()
            && !host.is_empty()
        {
            segments.push(host);
        }
        segments.extend(
            url.path_segments()
                .into_iter()
                .flatten()
                .filter(|segment| !segment.is_empty()),
        );

        match segments.as_slice() {
            [] => Ok((Self::Activate, None)),
            ["new"] => {
                let command = url
                    .query_pairs()
                    .find_map(|(key, value)| {
                        key.eq_ignore_ascii_case("cmd").then(|| value.into_owned())
                    })
                    .filter(|value| !value.trim().is_empty());
                let dir = url
                    .query_pairs()
                    .find_map(|(key, value)| {
                        key.eq_ignore_ascii_case("dir").then(|| value.into_owned())
                    })
                    .filter(|value| !value.trim().is_empty());
                let argument = if command.is_none() && dir.is_none() {
                    None
                } else {
                    Some(DeepLinkArgument::NewTab(NewTabDeepLink { command, dir }))
                };
                Ok((Self::NewTab, argument))
            }
            ["auth", "callback"] => {
                let session_token = url
                    .query_pairs()
                    .find_map(|(key, value)| {
                        key.eq_ignore_ascii_case("session_token")
                            .then(|| value.into_owned())
                    })
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| {
                        "Auth callback deeplink requires ?session_token=<token>".to_string()
                    })?;
                let user_id = url
                    .query_pairs()
                    .find_map(|(key, value)| {
                        key.eq_ignore_ascii_case("id").then(|| value.into_owned())
                    })
                    .filter(|value| !value.trim().is_empty());
                let avatar_url = url
                    .query_pairs()
                    .find_map(|(key, value)| {
                        key.eq_ignore_ascii_case("avatar_url")
                            .then(|| value.into_owned())
                    })
                    .filter(|value| !value.trim().is_empty());
                let name = url
                    .query_pairs()
                    .find_map(|(key, value)| {
                        key.eq_ignore_ascii_case("name").then(|| value.into_owned())
                    })
                    .filter(|value| !value.trim().is_empty());
                Ok((
                    Self::AuthCallback,
                    Some(DeepLinkArgument::AuthCallback(AuthCallbackDeepLink {
                        session_token,
                        user_id,
                        avatar_url,
                        name,
                    })),
                ))
            }
            ["settings"] => Ok((Self::Settings, None)),
            ["open", "config"] => Ok((Self::OpenConfig, None)),
            ["store", "theme-install"] => {
                let slug = url
                    .query_pairs()
                    .find_map(|(key, value)| {
                        (key.eq_ignore_ascii_case("slug") && !value.trim().is_empty())
                            .then(|| value.into_owned())
                    })
                    .ok_or_else(|| {
                        "Theme install deeplink requires ?slug=<theme-slug>".to_string()
                    })?;
                Ok((Self::ThemeInstall, Some(DeepLinkArgument::Value(slug))))
            }
            _ => Err(format!(
                "Unsupported Termy deeplink route: {}",
                segments.join("/")
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AuthCallbackDeepLink, DeepLinkArgument, DeepLinkRoute, DeepLinkRoute::*, NewTabDeepLink,
    };

    #[test]
    fn parses_bare_scheme_as_activate_route() {
        assert_eq!(DeepLinkRoute::parse("termy://"), Ok((Activate, None)));
        assert_eq!(DeepLinkRoute::parse("termy:///"), Ok((Activate, None)));
    }

    #[test]
    fn parses_settings_route() {
        assert_eq!(
            DeepLinkRoute::parse("termy://settings"),
            Ok((Settings, None))
        );
        assert_eq!(
            DeepLinkRoute::parse("termy:///settings"),
            Ok((Settings, None))
        );
        assert_eq!(
            DeepLinkRoute::parse("termy://settings?tab=general#section"),
            Ok((Settings, None))
        );
    }

    #[test]
    fn parses_new_tab_route() {
        assert_eq!(DeepLinkRoute::parse("termy://new"), Ok((NewTab, None)));
        assert_eq!(
            DeepLinkRoute::parse("termy://new?cmd=git%20status"),
            Ok((
                NewTab,
                Some(DeepLinkArgument::NewTab(NewTabDeepLink {
                    command: Some("git status".to_string()),
                    dir: None,
                }))
            ))
        );
        assert_eq!(
            DeepLinkRoute::parse("termy://new?cmd=git%20status&dir=%2Ftmp%2Fdemo"),
            Ok((
                NewTab,
                Some(DeepLinkArgument::NewTab(NewTabDeepLink {
                    command: Some("git status".to_string()),
                    dir: Some("/tmp/demo".to_string()),
                }))
            ))
        );
    }

    #[test]
    fn parses_open_config_route() {
        assert_eq!(
            DeepLinkRoute::parse("termy://open/config"),
            Ok((OpenConfig, None))
        );
        assert_eq!(
            DeepLinkRoute::parse("termy:///open/config"),
            Ok((OpenConfig, None))
        );
        assert_eq!(
            DeepLinkRoute::parse("termy://open/config?source=browser#top"),
            Ok((OpenConfig, None))
        );
    }

    #[test]
    fn parses_auth_callback_route() {
        assert_eq!(
            DeepLinkRoute::parse(
                "termy://auth/callback?session_token=abc123&id=123e4567-e89b-12d3-a456-426614174000&avatar_url=https%3A%2F%2Fexample.com%2Fa.png&name=Lasse"
            ),
            Ok((
                AuthCallback,
                Some(DeepLinkArgument::AuthCallback(AuthCallbackDeepLink {
                    session_token: "abc123".to_string(),
                    user_id: Some("123e4567-e89b-12d3-a456-426614174000".to_string()),
                    avatar_url: Some("https://example.com/a.png".to_string()),
                    name: Some("Lasse".to_string()),
                }))
            ))
        );
    }

    #[test]
    fn parses_theme_install_route() {
        assert_eq!(
            DeepLinkRoute::parse("termy://store/theme-install?slug=catppuccin-mocha"),
            Ok((
                ThemeInstall,
                Some(DeepLinkArgument::Value("catppuccin-mocha".to_string()))
            ))
        );
    }

    #[test]
    fn rejects_theme_install_without_slug() {
        let error = DeepLinkRoute::parse("termy://store/theme-install")
            .expect_err("theme install without slug should be rejected");
        assert!(error.contains("requires ?slug"));
    }

    #[test]
    fn rejects_wrong_scheme() {
        let error =
            DeepLinkRoute::parse("https://settings").expect_err("scheme should be rejected");
        assert!(error.contains("Unsupported deeplink scheme"));
    }

    #[test]
    fn rejects_unknown_route() {
        let error =
            DeepLinkRoute::parse("termy://workspace").expect_err("route should be rejected");
        assert!(error.contains("Unsupported Termy deeplink route"));
    }

    #[test]
    fn parses_bare_scheme_with_query_and_fragment_as_activate_route() {
        assert_eq!(
            DeepLinkRoute::parse("termy://?source=browser#noop"),
            Ok((Activate, None))
        );
    }

    #[test]
    fn rejects_malformed_url() {
        let error =
            DeepLinkRoute::parse("termy://[").expect_err("malformed deeplink should be rejected");
        assert!(error.contains("Invalid Termy deeplink"));
    }
}
