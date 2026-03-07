use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeepLinkRoute {
    Activate,
    Settings,
    OpenConfig,
    ThemeInstall,
}

impl DeepLinkRoute {
    pub(crate) fn parse(raw: &str) -> Result<(Self, Option<String>), String> {
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
                Ok((Self::ThemeInstall, Some(slug)))
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
    use super::{DeepLinkRoute, DeepLinkRoute::*};

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
    fn parses_theme_install_route() {
        assert_eq!(
            DeepLinkRoute::parse("termy://store/theme-install?slug=catppuccin-mocha"),
            Ok((ThemeInstall, Some("catppuccin-mocha".to_string())))
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
