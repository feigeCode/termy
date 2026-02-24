use crate::colors::TerminalColors;
use crate::config::{self, AppConfig, CursorStyle, TabTitleMode, set_config_value};
use crate::text_input::{TextInputAlignment, TextInputElement, TextInputProvider, TextInputState};
use gpui::{
    AnyElement, AsyncApp, Context, FocusHandle, Font, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Render,
    Rgba, ScrollAnchor, ScrollHandle, ScrollWheelEvent, SharedString, StatefulInteractiveElement,
    Styled, TextAlign, WeakEntity, Window, deferred, div, prelude::FluentBuilder, px,
};
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const SIDEBAR_WIDTH: f32 = 220.0;
const NUMERIC_INPUT_WIDTH: f32 = 220.0;
const NUMERIC_INPUT_HEIGHT: f32 = 34.0;
const NUMERIC_STEP_BUTTON_SIZE: f32 = 24.0;
const SETTINGS_CONFIG_WATCH_INTERVAL_MS: u64 = 750;
const SETTINGS_SEARCH_NAV_THROTTLE_MS: u64 = 110;
const SETTING_HIGHLIGHT_DURATION_MS: u64 = 900;
const SETTING_HIGHLIGHT_TICK_MS: u64 = 16;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum EditableField {
    Theme,
    BackgroundOpacity,
    FontFamily,
    FontSize,
    PaddingX,
    PaddingY,
    Shell,
    Term,
    Colorterm,
    ScrollbackHistory,
    ScrollMultiplier,
    TabFallbackTitle,
    WorkingDirectory,
    WindowWidth,
    WindowHeight,
}

#[derive(Clone, Debug)]
struct ActiveTextInput {
    field: EditableField,
    state: TextInputState,
    selecting: bool,
}

impl ActiveTextInput {
    fn new(field: EditableField, text: String) -> Self {
        Self {
            field,
            state: TextInputState::new(text),
            selecting: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SettingsSection {
    Appearance,
    Terminal,
    Tabs,
    Advanced,
}

#[derive(Clone, Copy, Debug)]
struct SearchableSetting {
    key: &'static str,
    section: SettingsSection,
    title: &'static str,
    description: &'static str,
    keywords: &'static [&'static str],
}

const SEARCHABLE_SETTINGS: &[SearchableSetting] = &[
    SearchableSetting {
        key: "theme",
        section: SettingsSection::Appearance,
        title: "Theme",
        description: "Current color scheme name",
        keywords: &["color", "scheme", "appearance"],
    },
    SearchableSetting {
        key: "background-blur",
        section: SettingsSection::Appearance,
        title: "Background Blur",
        description: "Enable blur effect for transparent backgrounds",
        keywords: &["blur", "transparent", "window"],
    },
    SearchableSetting {
        key: "background-opacity",
        section: SettingsSection::Appearance,
        title: "Background Opacity",
        description: "Window transparency (0-100%)",
        keywords: &["opacity", "transparency", "window"],
    },
    SearchableSetting {
        key: "font-family",
        section: SettingsSection::Appearance,
        title: "Font Family",
        description: "Font family used in terminal UI",
        keywords: &["font", "typeface", "text"],
    },
    SearchableSetting {
        key: "font-size",
        section: SettingsSection::Appearance,
        title: "Font Size",
        description: "Terminal font size in pixels",
        keywords: &["font", "size", "text", "zoom"],
    },
    SearchableSetting {
        key: "padding-x",
        section: SettingsSection::Appearance,
        title: "Horizontal Padding",
        description: "Left and right terminal padding",
        keywords: &["padding", "spacing", "left", "right"],
    },
    SearchableSetting {
        key: "padding-y",
        section: SettingsSection::Appearance,
        title: "Vertical Padding",
        description: "Top and bottom terminal padding",
        keywords: &["padding", "spacing", "top", "bottom"],
    },
    SearchableSetting {
        key: "cursor-blink",
        section: SettingsSection::Terminal,
        title: "Cursor Blink",
        description: "Enable blinking cursor animation",
        keywords: &["cursor", "blink", "animation"],
    },
    SearchableSetting {
        key: "cursor-style",
        section: SettingsSection::Terminal,
        title: "Cursor Style",
        description: "Shape of the terminal cursor",
        keywords: &["cursor", "block", "line"],
    },
    SearchableSetting {
        key: "shell",
        section: SettingsSection::Terminal,
        title: "Shell",
        description: "Executable for new sessions",
        keywords: &["shell", "bash", "zsh", "fish"],
    },
    SearchableSetting {
        key: "term",
        section: SettingsSection::Terminal,
        title: "TERM",
        description: "Terminal type for child apps",
        keywords: &["term", "terminal type", "env"],
    },
    SearchableSetting {
        key: "colorterm",
        section: SettingsSection::Terminal,
        title: "COLORTERM",
        description: "Color support advertisement",
        keywords: &["colorterm", "color", "env"],
    },
    SearchableSetting {
        key: "scrollback-history",
        section: SettingsSection::Terminal,
        title: "Scrollback History",
        description: "Lines to keep in buffer",
        keywords: &["scrollback", "history", "buffer", "lines"],
    },
    SearchableSetting {
        key: "scroll-multiplier",
        section: SettingsSection::Terminal,
        title: "Scroll Multiplier",
        description: "Mouse wheel scroll speed",
        keywords: &["scroll", "speed", "mouse", "wheel"],
    },
    SearchableSetting {
        key: "palette-keybinds",
        section: SettingsSection::Terminal,
        title: "Show Keybindings in Palette",
        description: "Display keyboard shortcuts in command palette",
        keywords: &["palette", "keybindings", "shortcuts", "command"],
    },
    SearchableSetting {
        key: "use-tabs",
        section: SettingsSection::Tabs,
        title: "Enable Tabs",
        description: "Show compact tab strip",
        keywords: &["tabs", "tab bar", "strip"],
    },
    SearchableSetting {
        key: "title-mode",
        section: SettingsSection::Tabs,
        title: "Title Mode",
        description: "How tab titles are determined",
        keywords: &[
            "tab", "title", "mode", "smart", "shell", "explicit", "static",
        ],
    },
    SearchableSetting {
        key: "shell-integration",
        section: SettingsSection::Tabs,
        title: "Shell Integration",
        description: "Export TERMY_* env vars for shell hooks",
        keywords: &["shell", "integration", "env", "hooks", "tab"],
    },
    SearchableSetting {
        key: "fallback-title",
        section: SettingsSection::Tabs,
        title: "Fallback Title",
        description: "Default when no other source available",
        keywords: &["fallback", "title", "tab"],
    },
    SearchableSetting {
        key: "working-directory",
        section: SettingsSection::Advanced,
        title: "Working Directory",
        description: "Initial directory for new sessions",
        keywords: &["working directory", "cwd", "startup", "path"],
    },
    SearchableSetting {
        key: "window-width",
        section: SettingsSection::Advanced,
        title: "Default Width",
        description: "Window width on startup",
        keywords: &["window", "width", "startup", "size"],
    },
    SearchableSetting {
        key: "window-height",
        section: SettingsSection::Advanced,
        title: "Default Height",
        description: "Window height on startup",
        keywords: &["window", "height", "startup", "size"],
    },
];

pub struct SettingsWindow {
    active_section: SettingsSection,
    config: AppConfig,
    config_path: Option<PathBuf>,
    config_fingerprint: Option<u64>,
    available_font_families: Vec<String>,
    focus_handle: FocusHandle,
    active_input: Option<ActiveTextInput>,
    content_scroll_handle: ScrollHandle,
    setting_scroll_anchors: HashMap<&'static str, ScrollAnchor>,
    sidebar_search_state: TextInputState,
    sidebar_search_active: bool,
    sidebar_search_selecting: bool,
    search_navigation_last_target: Option<&'static str>,
    search_navigation_last_jump_at: Option<Instant>,
    setting_highlight_key: Option<&'static str>,
    setting_highlight_started_at: Option<Instant>,
    setting_highlight_token: u64,
    colors: TerminalColors,
}

impl SettingsWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config = AppConfig::load_or_create();
        let config_path = config::ensure_config_file();
        let config_fingerprint = config_path.as_ref().and_then(Self::config_fingerprint);
        let config_change_rx = config::subscribe_config_changes();
        let mut available_font_families = window.text_system().all_font_names();
        available_font_families.sort_unstable_by_key(|font| font.to_ascii_lowercase());
        available_font_families.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        let colors = TerminalColors::from_theme(&config.theme, &config.colors);
        let content_scroll_handle = ScrollHandle::new();
        let setting_scroll_anchors = Self::build_setting_scroll_anchors(&content_scroll_handle);
        let view = Self {
            active_section: SettingsSection::Appearance,
            config,
            config_path,
            config_fingerprint,
            available_font_families,
            focus_handle: cx.focus_handle(),
            active_input: None,
            content_scroll_handle,
            setting_scroll_anchors,
            sidebar_search_state: TextInputState::new(String::new()),
            sidebar_search_active: false,
            sidebar_search_selecting: false,
            search_navigation_last_target: None,
            search_navigation_last_jump_at: None,
            setting_highlight_key: None,
            setting_highlight_started_at: None,
            setting_highlight_token: 0,
            colors,
        };

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            while config_change_rx.recv_async().await.is_ok() {
                while config_change_rx.try_recv().is_ok() {}
                let result = cx.update(|cx| {
                    this.update(cx, |view, cx| {
                        if view.reload_config_if_changed(cx) {
                            cx.notify();
                        }
                    })
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                smol::Timer::after(Duration::from_millis(SETTINGS_CONFIG_WATCH_INTERVAL_MS)).await;
                let result = cx.update(|cx| {
                    this.update(cx, |view, cx| {
                        if view.reload_config_if_changed(cx) {
                            cx.notify();
                        }
                    })
                });
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        view
    }

    fn config_fingerprint(path: &PathBuf) -> Option<u64> {
        let contents = fs::read(path).ok()?;
        let mut hasher = DefaultHasher::new();
        contents.hash(&mut hasher);
        Some(hasher.finish())
    }

    fn build_setting_scroll_anchors(
        content_scroll_handle: &ScrollHandle,
    ) -> HashMap<&'static str, ScrollAnchor> {
        SEARCHABLE_SETTINGS
            .iter()
            .map(|setting| {
                (
                    setting.key,
                    ScrollAnchor::for_handle(content_scroll_handle.clone()),
                )
            })
            .collect()
    }

    fn settings_section_label(section: SettingsSection) -> &'static str {
        match section {
            SettingsSection::Appearance => "Appearance",
            SettingsSection::Terminal => "Terminal",
            SettingsSection::Tabs => "Tabs",
            SettingsSection::Advanced => "Advanced",
        }
    }

    fn searchable_setting_by_key(key: &'static str) -> Option<&'static SearchableSetting> {
        SEARCHABLE_SETTINGS
            .iter()
            .find(|setting| setting.key == key)
    }

    fn setting_search_score(
        setting: &SearchableSetting,
        query: &str,
        terms: &[&str],
    ) -> Option<i32> {
        let title = setting.title.to_ascii_lowercase();
        let description = setting.description.to_ascii_lowercase();
        let section = Self::settings_section_label(setting.section).to_ascii_lowercase();
        let keywords = setting.keywords.join(" ").to_ascii_lowercase();
        let haystack = format!("{title} {description} {section} {keywords}");

        if !terms.iter().all(|term| haystack.contains(term)) {
            return None;
        }

        let mut score = 0;
        if title == query {
            score += 150;
        }
        if title.starts_with(query) {
            score += 95;
        } else if title.contains(query) {
            score += 60;
        }
        if description.contains(query) {
            score += 24;
        }
        if section.contains(query) {
            score += 18;
        }
        if keywords.contains(query) {
            score += 30;
        }

        for term in terms {
            if title.starts_with(term) {
                score += 20;
            } else if title.contains(term) {
                score += 10;
            }
            if keywords.contains(term) {
                score += 8;
            }
        }

        Some(score.max(1))
    }

    fn sidebar_search_results(&self, limit: usize) -> Vec<&'static SearchableSetting> {
        let query = self.sidebar_search_state.text().trim().to_ascii_lowercase();
        if query.is_empty() {
            return Vec::new();
        }

        let terms: Vec<&str> = query.split_whitespace().collect();
        let mut matches: Vec<(i32, &'static SearchableSetting)> = SEARCHABLE_SETTINGS
            .iter()
            .filter_map(|setting| {
                Self::setting_search_score(setting, &query, &terms).map(|score| (score, setting))
            })
            .collect();

        matches.sort_by(|(left_score, left_setting), (right_score, right_setting)| {
            right_score
                .cmp(left_score)
                .then_with(|| left_setting.title.cmp(right_setting.title))
        });

        matches
            .into_iter()
            .map(|(_, setting)| setting)
            .take(limit)
            .collect()
    }

    fn wrap_setting_with_scroll_anchor(
        &self,
        setting_key: &'static str,
        content: AnyElement,
    ) -> AnyElement {
        div()
            .id(SharedString::from(format!("setting-{setting_key}")))
            .anchor_scroll(self.setting_scroll_anchors.get(setting_key).cloned())
            .child(content)
            .into_any_element()
    }

    fn blur_sidebar_search(&mut self) {
        self.sidebar_search_active = false;
        self.sidebar_search_selecting = false;
        self.search_navigation_last_target = None;
        self.search_navigation_last_jump_at = None;
    }

    fn start_setting_highlight(&mut self, setting_key: &'static str, cx: &mut Context<Self>) {
        self.setting_highlight_key = Some(setting_key);
        self.setting_highlight_started_at = Some(Instant::now());
        self.setting_highlight_token = self.setting_highlight_token.wrapping_add(1);
        let token = self.setting_highlight_token;

        cx.spawn(async move |this: WeakEntity<Self>, cx: &mut AsyncApp| {
            loop {
                smol::Timer::after(Duration::from_millis(SETTING_HIGHLIGHT_TICK_MS)).await;

                let keep_animating = cx.update(|cx| {
                    this.update(cx, |view, cx| {
                        if view.setting_highlight_token != token {
                            return false;
                        }

                        let done = match view.setting_highlight_started_at {
                            Some(started_at) => {
                                started_at.elapsed()
                                    >= Duration::from_millis(SETTING_HIGHLIGHT_DURATION_MS)
                            }
                            None => true,
                        };

                        if done {
                            view.setting_highlight_key = None;
                            view.setting_highlight_started_at = None;
                        }

                        cx.notify();
                        !done
                    })
                    .unwrap_or(false)
                });

                if !keep_animating {
                    break;
                }
            }
        })
        .detach();
    }

    fn setting_highlight_intensity(&self, setting_key: &'static str) -> f32 {
        if self.setting_highlight_key != Some(setting_key) {
            return 0.0;
        }
        let Some(started_at) = self.setting_highlight_started_at else {
            return 0.0;
        };

        let elapsed = started_at.elapsed().as_secs_f32();
        let duration = Duration::from_millis(SETTING_HIGHLIGHT_DURATION_MS).as_secs_f32();
        if duration <= f32::EPSILON {
            return 0.0;
        }

        let t = (elapsed / duration).clamp(0.0, 1.0);
        (1.0 - t).powf(2.0)
    }

    fn apply_setting_highlight_bg(&self, base: Rgba, intensity: f32) -> Rgba {
        if intensity <= f32::EPSILON {
            return base;
        }

        let mut overlay = self.accent();
        overlay.a = (0.22 * intensity).clamp(0.0, 1.0);
        Self::composite_over(overlay, base)
    }

    fn apply_setting_highlight_border(&self, base: Rgba, intensity: f32) -> Rgba {
        if intensity <= f32::EPSILON {
            return base;
        }

        let mut overlay = self.accent();
        overlay.a = (0.82 * intensity).clamp(0.0, 1.0);
        Self::composite_over(overlay, base)
    }

    fn jump_to_setting(
        &mut self,
        setting_key: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(setting) = Self::searchable_setting_by_key(setting_key) else {
            return;
        };

        self.active_section = setting.section;
        self.active_input = None;
        self.sidebar_search_active = true;
        self.sidebar_search_selecting = false;
        self.start_setting_highlight(setting_key, cx);
        if !self.focus_handle.is_focused(window) {
            self.focus_handle.focus(window, cx);
        }

        if let Some(anchor) = self.setting_scroll_anchors.get(setting_key).cloned() {
            cx.on_next_frame(window, move |_, window, cx| {
                anchor.scroll_to(window, cx);
                cx.notify();
            });
        }

        cx.notify();
    }

    fn jump_to_first_search_result(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(first) = self.sidebar_search_results(1).into_iter().next() {
            self.jump_to_setting(first.key, window, cx);
        }
    }

    fn refresh_search_navigation(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.sidebar_search_active
            && self.active_input.is_none()
            && !self.sidebar_search_state.text().trim().is_empty()
        {
            let Some(first) = self.sidebar_search_results(1).into_iter().next() else {
                self.search_navigation_last_target = None;
                self.search_navigation_last_jump_at = None;
                cx.notify();
                return;
            };

            let now = Instant::now();
            let within_throttle = self.search_navigation_last_jump_at.is_some_and(|last| {
                now.duration_since(last) < Duration::from_millis(SETTINGS_SEARCH_NAV_THROTTLE_MS)
            });
            if self.search_navigation_last_target == Some(first.key) && within_throttle {
                cx.notify();
                return;
            }

            self.search_navigation_last_target = Some(first.key);
            self.search_navigation_last_jump_at = Some(now);
            self.jump_to_setting(first.key, window, cx);
        } else {
            self.search_navigation_last_target = None;
            self.search_navigation_last_jump_at = None;
            cx.notify();
        }
    }

    fn apply_runtime_config(&mut self, config: AppConfig) -> bool {
        self.colors = TerminalColors::from_theme(&config.theme, &config.colors);
        self.config = config;
        true
    }

    fn reload_config_if_changed(&mut self, _cx: &mut Context<Self>) -> bool {
        let path = match self.config_path.clone() {
            Some(path) => path,
            None => {
                self.config_path = config::ensure_config_file();
                match self.config_path.clone() {
                    Some(path) => path,
                    None => return false,
                }
            }
        };

        let Some(fingerprint) = Self::config_fingerprint(&path) else {
            return false;
        };

        if self.config_fingerprint == Some(fingerprint) {
            return false;
        }

        self.config_fingerprint = Some(fingerprint);
        let config = AppConfig::load_or_create();
        self.apply_runtime_config(config)
    }

    // Color helpers derived from terminal theme
    fn bg_primary(&self) -> Rgba {
        self.colors.background
    }

    fn bg_secondary(&self) -> Rgba {
        let mut c = self.colors.background;
        c.a = 0.7;
        c
    }

    fn bg_card(&self) -> Rgba {
        let mut c = self.colors.background;
        c.a = 0.5;
        c
    }

    fn bg_input(&self) -> Rgba {
        let mut c = self.colors.background;
        c.a = 0.3;
        c
    }

    fn bg_hover(&self) -> Rgba {
        let mut c = self.colors.foreground;
        c.a = 0.1;
        c
    }

    fn bg_active(&self) -> Rgba {
        let mut c = self.colors.foreground;
        c.a = 0.15;
        c
    }

    fn text_primary(&self) -> Rgba {
        self.colors.foreground
    }

    fn text_secondary(&self) -> Rgba {
        let mut c = self.colors.foreground;
        c.a = 0.7;
        c
    }

    fn text_muted(&self) -> Rgba {
        let mut c = self.colors.foreground;
        c.a = 0.5;
        c
    }

    fn border_color(&self) -> Rgba {
        let mut c = self.colors.foreground;
        c.a = 0.15;
        c
    }

    fn accent(&self) -> Rgba {
        self.colors.cursor
    }

    fn accent_with_alpha(&self, alpha: f32) -> Rgba {
        let mut c = self.colors.cursor;
        c.a = alpha;
        c
    }

    fn srgb_to_linear(channel: f32) -> f32 {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    }

    fn composite_over(fg: Rgba, bg: Rgba) -> Rgba {
        let fg_alpha = fg.a.clamp(0.0, 1.0);
        Rgba {
            r: (fg_alpha * fg.r + (1.0 - fg_alpha) * bg.r).clamp(0.0, 1.0),
            g: (fg_alpha * fg.g + (1.0 - fg_alpha) * bg.g).clamp(0.0, 1.0),
            b: (fg_alpha * fg.b + (1.0 - fg_alpha) * bg.b).clamp(0.0, 1.0),
            a: 1.0,
        }
    }

    fn relative_luminance(color: Rgba, backdrop: Rgba) -> f32 {
        let composited = Self::composite_over(color, backdrop);
        let r = Self::srgb_to_linear(composited.r);
        let g = Self::srgb_to_linear(composited.g);
        let b = Self::srgb_to_linear(composited.b);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    fn contrast_ratio(a: Rgba, b: Rgba, backdrop: Rgba) -> f32 {
        let l1 = Self::relative_luminance(a, backdrop);
        let l2 = Self::relative_luminance(b, backdrop);
        let (lighter, darker) = if l1 >= l2 { (l1, l2) } else { (l2, l1) };
        (lighter + 0.05) / (darker + 0.05)
    }

    fn contrasting_text_for_fill(&self, fill: Rgba, backdrop: Rgba) -> Rgba {
        let mut primary = self.text_primary();
        primary.a = 1.0;
        let mut dark = self.bg_primary();
        dark.a = 1.0;
        let mut backdrop = backdrop;
        backdrop.a = 1.0;
        let composited_fill = Self::composite_over(fill, backdrop);

        if Self::contrast_ratio(primary, composited_fill, backdrop)
            >= Self::contrast_ratio(dark, composited_fill, backdrop)
        {
            primary
        } else {
            dark
        }
    }

    fn render_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .bg(self.bg_secondary())
            .border_r_1()
            .border_color(self.border_color())
            .flex()
            .flex_col()
            .child(
                div().px_5().pt_10().pb_2().child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(self.text_muted())
                        .child("SETTINGS"),
                ),
            )
            .child(self.render_sidebar_search(cx))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .px_3()
                    .child(self.render_sidebar_item("Appearance", SettingsSection::Appearance, cx))
                    .child(self.render_sidebar_item("Terminal", SettingsSection::Terminal, cx))
                    .child(self.render_sidebar_item("Tabs", SettingsSection::Tabs, cx))
                    .child(self.render_sidebar_item("Advanced", SettingsSection::Advanced, cx)),
            )
    }

    fn render_sidebar_search(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let query_text = self.sidebar_search_state.text().to_string();
        let has_query = !query_text.trim().is_empty();
        let is_active = self.sidebar_search_active;
        let text_secondary = self.text_secondary();
        let text_muted = self.text_muted();
        let bg_input = self.bg_input();
        let border_color = self.border_color();
        let accent = self.accent();

        let search_content = if is_active {
            let font = Font {
                family: self.config.font_family.clone().into(),
                ..Font::default()
            };
            TextInputElement::new(
                cx.entity(),
                self.focus_handle.clone(),
                font,
                px(13.0),
                text_secondary.into(),
                self.accent_with_alpha(0.3).into(),
                TextInputAlignment::Left,
            )
            .into_any_element()
        } else if has_query {
            div()
                .text_sm()
                .text_color(text_secondary)
                .child(query_text.clone())
                .into_any_element()
        } else {
            div()
                .text_sm()
                .text_color(text_muted)
                .child("Search settings...")
                .into_any_element()
        };

        let search_container = div().id("settings-sidebar-search").px_3().pb_3().child(
            div()
                .id("settings-sidebar-search-input")
                .h(px(36.0))
                .px_3()
                .rounded_lg()
                .bg(bg_input)
                .border_1()
                .border_color(if is_active {
                    accent.into()
                } else {
                    border_color
                })
                .overflow_hidden()
                .cursor_text()
                .flex()
                .items_center()
                .child(
                    div()
                        .w_full()
                        .h(px(20.0))
                        .overflow_hidden()
                        .child(search_content),
                )
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|view, event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        view.active_input = None;
                        view.sidebar_search_active = true;

                        let index = view
                            .sidebar_search_state
                            .character_index_for_point(event.position);
                        if event.modifiers.shift {
                            view.sidebar_search_state.select_to_utf16(index);
                        } else {
                            view.sidebar_search_state.set_cursor_utf16(index);
                        }
                        view.sidebar_search_selecting = true;
                        view.focus_handle.focus(window, cx);
                        cx.notify();
                    }),
                )
                .on_mouse_move(cx.listener(|view, event: &MouseMoveEvent, _window, cx| {
                    if !view.sidebar_search_selecting || !event.dragging() {
                        return;
                    }
                    let index = view
                        .sidebar_search_state
                        .character_index_for_point(event.position);
                    view.sidebar_search_state.select_to_utf16(index);
                    cx.notify();
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|view, _event: &MouseUpEvent, _window, cx| {
                        if view.sidebar_search_selecting {
                            view.sidebar_search_selecting = false;
                            cx.notify();
                        }
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(|view, _event: &MouseUpEvent, _window, cx| {
                        if view.sidebar_search_selecting {
                            view.sidebar_search_selecting = false;
                            cx.notify();
                        }
                    }),
                ),
        );

        search_container
    }

    fn render_sidebar_item(
        &self,
        label: &'static str,
        section: SettingsSection,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.active_section == section;
        let active_bg = self.bg_active();
        let hover_bg = self.bg_hover();
        let text_primary = self.text_primary();
        let text_secondary = self.text_secondary();
        let accent = self.accent();

        div()
            .id(SharedString::from(label))
            .px_3()
            .py(px(10.0))
            .rounded_lg()
            .cursor_pointer()
            .flex()
            .items_center()
            .gap_3()
            .bg(if is_active {
                active_bg
            } else {
                Rgba {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }
            })
            .hover(|s| s.bg(hover_bg))
            .child(
                div()
                    .text_sm()
                    .font_weight(if is_active {
                        gpui::FontWeight::MEDIUM
                    } else {
                        gpui::FontWeight::NORMAL
                    })
                    .text_color(if is_active {
                        text_primary
                    } else {
                        text_secondary
                    })
                    .child(label),
            )
            .when(is_active, |s| {
                s.child(
                    div()
                        .ml_auto()
                        .w(px(3.0))
                        .h(px(16.0))
                        .rounded(px(2.0))
                        .bg(accent),
                )
            })
            .on_click(cx.listener(move |view, _, _, cx| {
                view.active_section = section;
                view.active_input = None;
                view.blur_sidebar_search();
                cx.notify();
            }))
    }

    fn editable_field_value(&self, field: EditableField) -> String {
        match field {
            EditableField::Theme => self.config.theme.clone(),
            EditableField::BackgroundOpacity => format!(
                "{}",
                (self.config.background_opacity * 100.0).round() as i32
            ),
            EditableField::FontFamily => self.config.font_family.clone(),
            EditableField::FontSize => format!("{}", self.config.font_size.round() as i32),
            EditableField::PaddingX => format!("{}", self.config.padding_x.round() as i32),
            EditableField::PaddingY => format!("{}", self.config.padding_y.round() as i32),
            EditableField::Shell => self.config.shell.clone().unwrap_or_default(),
            EditableField::Term => self.config.term.clone(),
            EditableField::Colorterm => self.config.colorterm.clone().unwrap_or_default(),
            EditableField::ScrollbackHistory => self.config.scrollback_history.to_string(),
            EditableField::ScrollMultiplier => format!("{}", self.config.mouse_scroll_multiplier),
            EditableField::TabFallbackTitle => self.config.tab_title.fallback.clone(),
            EditableField::WorkingDirectory => self.config.working_dir.clone().unwrap_or_default(),
            EditableField::WindowWidth => format!("{}", self.config.window_width.round() as i32),
            EditableField::WindowHeight => format!("{}", self.config.window_height.round() as i32),
        }
    }

    fn apply_editable_field(&mut self, field: EditableField, raw: &str) -> Result<(), String> {
        let value = raw.trim();
        match field {
            EditableField::Theme => {
                if value.is_empty() {
                    return Err("Theme cannot be empty".to_string());
                }
                let message = crate::config::set_theme_in_config(value)?;
                let canonical_theme = message
                    .strip_prefix("Theme set to ")
                    .unwrap_or(value)
                    .to_string();
                self.config.theme = canonical_theme;
                Ok(())
            }
            EditableField::BackgroundOpacity => {
                let parsed = value
                    .trim_end_matches('%')
                    .parse::<f32>()
                    .map_err(|_| "Background opacity must be a number from 0 to 100".to_string())?;
                let opacity = (parsed / 100.0).clamp(0.0, 1.0);
                self.config.background_opacity = opacity;
                set_config_value("background_opacity", &format!("{:.3}", opacity))
            }
            EditableField::FontFamily => {
                if value.is_empty() {
                    return Err("Font family cannot be empty".to_string());
                }
                self.config.font_family = value.to_string();
                set_config_value("font_family", value)
            }
            EditableField::FontSize => {
                let parsed = value
                    .parse::<f32>()
                    .map_err(|_| "Font size must be a positive number".to_string())?;
                if parsed <= 0.0 {
                    return Err("Font size must be greater than 0".to_string());
                }
                self.config.font_size = parsed;
                set_config_value("font_size", &format!("{}", parsed))
            }
            EditableField::PaddingX => {
                let parsed = value
                    .parse::<f32>()
                    .map_err(|_| "Horizontal padding must be a number".to_string())?;
                if parsed < 0.0 {
                    return Err("Horizontal padding cannot be negative".to_string());
                }
                self.config.padding_x = parsed;
                set_config_value("padding_x", &format!("{}", parsed))
            }
            EditableField::PaddingY => {
                let parsed = value
                    .parse::<f32>()
                    .map_err(|_| "Vertical padding must be a number".to_string())?;
                if parsed < 0.0 {
                    return Err("Vertical padding cannot be negative".to_string());
                }
                self.config.padding_y = parsed;
                set_config_value("padding_y", &format!("{}", parsed))
            }
            EditableField::Shell => {
                if value.is_empty() {
                    self.config.shell = None;
                    set_config_value("shell", "none")
                } else {
                    self.config.shell = Some(value.to_string());
                    set_config_value("shell", value)
                }
            }
            EditableField::Term => {
                if value.is_empty() {
                    return Err("TERM cannot be empty".to_string());
                }
                self.config.term = value.to_string();
                set_config_value("term", value)
            }
            EditableField::Colorterm => {
                if value.is_empty() {
                    self.config.colorterm = None;
                    set_config_value("colorterm", "none")
                } else {
                    self.config.colorterm = Some(value.to_string());
                    set_config_value("colorterm", value)
                }
            }
            EditableField::ScrollbackHistory => {
                let parsed = value
                    .parse::<usize>()
                    .map_err(|_| "Scrollback history must be a positive integer".to_string())?;
                let parsed = parsed.min(100_000);
                self.config.scrollback_history = parsed;
                set_config_value("scrollback_history", &parsed.to_string())
            }
            EditableField::ScrollMultiplier => {
                let parsed = value
                    .parse::<f32>()
                    .map_err(|_| "Scroll multiplier must be a number".to_string())?;
                if !parsed.is_finite() {
                    return Err("Scroll multiplier must be finite".to_string());
                }
                let parsed = parsed.clamp(0.1, 1000.0);
                self.config.mouse_scroll_multiplier = parsed;
                set_config_value("mouse_scroll_multiplier", &parsed.to_string())
            }
            EditableField::TabFallbackTitle => {
                if value.is_empty() {
                    return Err("Fallback title cannot be empty".to_string());
                }
                self.config.tab_title.fallback = value.to_string();
                set_config_value("tab_title_fallback", value)
            }
            EditableField::WorkingDirectory => {
                if value.is_empty() {
                    self.config.working_dir = None;
                    set_config_value("working_dir", "none")
                } else {
                    self.config.working_dir = Some(value.to_string());
                    set_config_value("working_dir", value)
                }
            }
            EditableField::WindowWidth => {
                let parsed = value
                    .parse::<f32>()
                    .map_err(|_| "Default width must be a positive number".to_string())?;
                if parsed <= 0.0 {
                    return Err("Default width must be greater than 0".to_string());
                }
                self.config.window_width = parsed;
                set_config_value("window_width", &parsed.to_string())
            }
            EditableField::WindowHeight => {
                let parsed = value
                    .parse::<f32>()
                    .map_err(|_| "Default height must be a positive number".to_string())?;
                if parsed <= 0.0 {
                    return Err("Default height must be greater than 0".to_string());
                }
                self.config.window_height = parsed;
                set_config_value("window_height", &parsed.to_string())
            }
        }
    }

    fn begin_editing_field(
        &mut self,
        field: EditableField,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.blur_sidebar_search();
        self.active_input = Some(ActiveTextInput::new(
            field,
            self.editable_field_value(field),
        ));
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn is_numeric_field(field: EditableField) -> bool {
        matches!(
            field,
            EditableField::BackgroundOpacity
                | EditableField::FontSize
                | EditableField::PaddingX
                | EditableField::PaddingY
                | EditableField::ScrollbackHistory
                | EditableField::ScrollMultiplier
                | EditableField::WindowWidth
                | EditableField::WindowHeight
        )
    }

    fn uses_text_input_for_field(field: EditableField) -> bool {
        !Self::is_numeric_field(field)
    }

    fn step_numeric_field(&mut self, field: EditableField, delta: i32, cx: &mut Context<Self>) {
        let result = match field {
            EditableField::BackgroundOpacity => {
                let next = (self.config.background_opacity + (delta as f32 * 0.05)).clamp(0.0, 1.0);
                self.config.background_opacity = next;
                set_config_value("background_opacity", &format!("{:.3}", next))
            }
            EditableField::FontSize => {
                let next = (self.config.font_size + delta as f32).max(1.0);
                self.config.font_size = next;
                set_config_value("font_size", &next.to_string())
            }
            EditableField::PaddingX => {
                let next = (self.config.padding_x + delta as f32).max(0.0);
                self.config.padding_x = next;
                set_config_value("padding_x", &next.to_string())
            }
            EditableField::PaddingY => {
                let next = (self.config.padding_y + delta as f32).max(0.0);
                self.config.padding_y = next;
                set_config_value("padding_y", &next.to_string())
            }
            EditableField::ScrollbackHistory => {
                let next = (self.config.scrollback_history as i64 + (delta as i64 * 100))
                    .clamp(0, 100_000) as usize;
                self.config.scrollback_history = next;
                set_config_value("scrollback_history", &next.to_string())
            }
            EditableField::ScrollMultiplier => {
                let next =
                    (self.config.mouse_scroll_multiplier + (delta as f32 * 0.1)).clamp(0.1, 1000.0);
                self.config.mouse_scroll_multiplier = next;
                set_config_value("mouse_scroll_multiplier", &next.to_string())
            }
            EditableField::WindowWidth => {
                let next = (self.config.window_width + (delta as f32 * 20.0)).max(1.0);
                self.config.window_width = next;
                set_config_value("window_width", &next.to_string())
            }
            EditableField::WindowHeight => {
                let next = (self.config.window_height + (delta as f32 * 20.0)).max(1.0);
                self.config.window_height = next;
                set_config_value("window_height", &next.to_string())
            }
            _ => Ok(()),
        };

        if let Err(error) = result {
            termy_toast::error(error);
        }
        self.active_input = None;
        cx.notify();
    }

    fn ordered_theme_ids_for_settings(&self) -> Vec<String> {
        let mut theme_ids: Vec<String> = termy_themes::available_theme_ids()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect();
        theme_ids.push("shell-decide".to_string());

        if !theme_ids.iter().any(|theme| theme == &self.config.theme) {
            theme_ids.push(self.config.theme.clone());
        }

        theme_ids.sort_unstable();
        theme_ids.dedup();
        theme_ids
    }

    fn ordered_font_families_for_settings(&self) -> Vec<String> {
        let mut fonts = self.available_font_families.clone();
        if !fonts
            .iter()
            .any(|font| font.eq_ignore_ascii_case(&self.config.font_family))
        {
            fonts.push(self.config.font_family.clone());
        }
        fonts.sort_unstable_by_key(|font| font.to_ascii_lowercase());
        fonts.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
        fonts
    }

    fn filtered_theme_suggestions(&self, query: &str) -> Vec<String> {
        let normalized = query.trim().to_ascii_lowercase();
        let themes = self.ordered_theme_ids_for_settings();

        if normalized.is_empty() {
            return themes.into_iter().take(16).collect();
        }

        let mut matched = Vec::new();
        let mut rest = Vec::new();
        for theme in themes {
            let lower = theme.to_ascii_lowercase();
            if lower.contains(&normalized) || lower.replace('-', " ").contains(&normalized) {
                matched.push(theme);
            } else {
                rest.push(theme);
            }
        }
        matched.extend(rest);
        matched.into_iter().take(16).collect()
    }

    fn filtered_font_suggestions(&self, query: &str) -> Vec<String> {
        let normalized = query.trim().to_ascii_lowercase();
        let fonts = self.ordered_font_families_for_settings();
        let selected_font = self.config.font_family.trim().to_ascii_lowercase();

        // When the dropdown first opens, the input text equals the selected font.
        // Treat that like an empty query so users can browse the full installed list.
        if normalized.is_empty() || normalized == selected_font {
            return fonts;
        }

        fonts
            .into_iter()
            .filter(|font| font.to_ascii_lowercase().contains(&normalized))
            .collect()
    }

    fn apply_theme_selection(&mut self, theme_id: &str, cx: &mut Context<Self>) {
        if let Err(error) = self.apply_editable_field(EditableField::Theme, theme_id) {
            termy_toast::error(error);
        }
        self.active_input = None;
        cx.notify();
    }

    fn apply_font_selection(&mut self, font_family: &str, cx: &mut Context<Self>) {
        if let Err(error) = self.apply_editable_field(EditableField::FontFamily, font_family) {
            termy_toast::error(error);
        }
        self.active_input = None;
        cx.notify();
    }

    fn commit_active_input(&mut self, cx: &mut Context<Self>) {
        let Some(input) = self.active_input.take() else {
            return;
        };

        if let Err(error) = self.apply_editable_field(input.field, input.state.text()) {
            termy_toast::error(error);
            self.active_input = Some(input);
        }
        cx.notify();
    }

    fn cancel_active_input(&mut self, cx: &mut Context<Self>) {
        self.active_input = None;
        cx.notify();
    }

    fn render_content(&mut self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .w_full()
            .child(match self.active_section {
                SettingsSection::Appearance => {
                    self.render_appearance_section(cx).into_any_element()
                }
                SettingsSection::Terminal => self.render_terminal_section(cx).into_any_element(),
                SettingsSection::Tabs => self.render_tabs_section(cx).into_any_element(),
                SettingsSection::Advanced => self.render_advanced_section(cx).into_any_element(),
            })
            .into_any_element()
    }

    fn render_section_header(
        &self,
        title: &'static str,
        subtitle: &'static str,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .mb_6()
            .child(
                div()
                    .text_xl()
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(self.text_primary())
                    .child(title),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(self.text_muted())
                    .child(subtitle),
            )
    }

    fn render_group_header(&self, title: &'static str) -> impl IntoElement {
        div()
            .text_xs()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(self.text_muted())
            .mt_4()
            .mb_2()
            .child(title)
    }

    fn render_setting_row(
        &self,
        search_key: &'static str,
        id: &'static str,
        title: &'static str,
        description: &'static str,
        checked: bool,
        cx: &mut Context<Self>,
        on_toggle: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let highlight_intensity = self.setting_highlight_intensity(search_key);
        let bg_card = self.apply_setting_highlight_bg(self.bg_card(), highlight_intensity);
        let border_color =
            self.apply_setting_highlight_border(self.border_color(), highlight_intensity);

        let row = div()
            .flex()
            .items_center()
            .justify_between()
            .py_3()
            .px_4()
            .rounded_lg()
            .bg(bg_card)
            .border_1()
            .border_color(border_color)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(self.text_primary())
                            .child(title),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(self.text_muted())
                            .child(description),
                    ),
            )
            .child(self.render_switch(id, checked, cx, on_toggle));

        self.wrap_setting_with_scroll_anchor(search_key, row.into_any_element())
    }

    fn render_switch(
        &self,
        id: &'static str,
        checked: bool,
        cx: &mut Context<Self>,
        on_toggle: impl Fn(&mut Self, &mut Context<Self>) + 'static,
    ) -> impl IntoElement {
        let accent = self.accent();
        // Off state: use a more visible muted foreground color
        let mut bg_off = self.colors.foreground;
        bg_off.a = 0.25;
        let track_color = if checked { accent } else { bg_off };
        let knob_color = self.contrasting_text_for_fill(track_color, self.bg_card());

        div()
            .id(SharedString::from(id))
            .w(px(44.0))
            .h(px(24.0))
            .rounded(px(12.0))
            .bg(track_color)
            .cursor_pointer()
            .relative()
            .child(
                div()
                    .absolute()
                    .top(px(2.0))
                    .left(if checked { px(22.0) } else { px(2.0) })
                    .w(px(20.0))
                    .h(px(20.0))
                    .rounded_full()
                    .bg(knob_color)
                    .shadow_sm(),
            )
            .on_click(cx.listener(move |view, _, _, cx| {
                on_toggle(view, cx);
                cx.notify();
            }))
    }

    fn render_editable_row(
        &mut self,
        search_key: &'static str,
        field: EditableField,
        title: &'static str,
        description: &'static str,
        display_value: String,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let is_numeric = Self::is_numeric_field(field);
        let is_active = self
            .active_input
            .as_ref()
            .is_some_and(|input| input.field == field);
        let is_theme_field = field == EditableField::Theme;
        let is_font_field = field == EditableField::FontFamily;
        let accent_inner_border = is_numeric || is_theme_field || is_font_field;
        let highlight_intensity = self.setting_highlight_intensity(search_key);
        let theme_suggestions = if is_theme_field && is_active {
            let query = self
                .active_input
                .as_ref()
                .map(|input| input.state.text())
                .unwrap_or("");
            self.filtered_theme_suggestions(query)
        } else {
            Vec::new()
        };
        let font_suggestions = if is_font_field && is_active {
            let query = self
                .active_input
                .as_ref()
                .map(|input| input.state.text())
                .unwrap_or("");
            self.filtered_font_suggestions(query)
        } else {
            Vec::new()
        };
        let dropdown_options = if is_theme_field {
            theme_suggestions
        } else if is_font_field {
            font_suggestions
        } else {
            Vec::new()
        };

        // Cache colors for closures
        let text_secondary = self.text_secondary();
        let hover_bg = self.bg_hover();
        let input_bg = self.bg_input();
        let border_color =
            self.apply_setting_highlight_border(self.border_color(), highlight_intensity);
        let accent = self.accent();
        let bg_card = self.apply_setting_highlight_bg(self.bg_card(), highlight_intensity);
        let text_primary = self.text_primary();
        let text_muted = self.text_muted();

        let mut dropdown = None;
        let dropdown_open =
            is_active && (is_theme_field || is_font_field) && !dropdown_options.is_empty();
        if dropdown_open {
            let mut list = div().flex().flex_col().py_1();
            for (index, option) in dropdown_options.into_iter().enumerate() {
                let option_label = option.clone();
                let option_value = option.clone();
                list = list.child(
                    div()
                        .id(SharedString::from(if is_theme_field {
                            format!("theme-option-{index}")
                        } else {
                            format!("font-option-{index}")
                        }))
                        .px_3()
                        .py_1()
                        .text_sm()
                        .text_color(text_secondary)
                        .cursor_pointer()
                        .when(is_font_field, |s| s.font_family(option_value.clone()))
                        .hover(|this| this.bg(hover_bg))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |view, _event: &MouseDownEvent, _window, cx| {
                                cx.stop_propagation();
                                if is_theme_field {
                                    view.apply_theme_selection(&option_value, cx);
                                } else {
                                    view.apply_font_selection(&option_value, cx);
                                }
                            }),
                        )
                        .child(option_label),
                );
            }

            // Use a fully opaque background for the dropdown so it covers content below
            let dropdown_bg = self.bg_primary();
            dropdown = Some(
                deferred(
                    div()
                        .id(if is_theme_field {
                            "theme-suggestions-list"
                        } else {
                            "font-suggestions-list"
                        })
                        .occlude()
                        .absolute()
                        .top(px(34.0))
                        .left_0()
                        .right_0()
                        .max_h(if is_theme_field { px(180.0) } else { px(240.0) })
                        .overflow_scroll()
                        .overflow_x_hidden()
                        .rounded_md()
                        .bg(dropdown_bg)
                        .border_1()
                        .border_color(border_color)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|_view, _event: &MouseDownEvent, _window, cx| {
                                cx.stop_propagation();
                            }),
                        )
                        .on_scroll_wheel(cx.listener(
                            |_view, _event: &ScrollWheelEvent, _window, cx| {
                                cx.stop_propagation();
                            },
                        ))
                        .child(list),
                )
                .with_priority(10)
                .into_any_element(),
            );
        }

        let value_element = if is_numeric {
            div()
                .h_full()
                .flex()
                .items_center()
                .justify_between()
                .gap_1()
                .child(
                    div()
                        .id(SharedString::from(format!("dec-{field:?}")))
                        .w(px(NUMERIC_STEP_BUTTON_SIZE))
                        .h(px(NUMERIC_STEP_BUTTON_SIZE))
                        .rounded_sm()
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(bg_card)
                        .text_color(text_primary)
                        .text_sm()
                        .child("-")
                        .on_click(cx.listener(move |view, _, _, cx| {
                            cx.stop_propagation();
                            view.step_numeric_field(field, -1, cx);
                        })),
                )
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .text_color(text_secondary)
                        .text_align(TextAlign::Center)
                        .child(display_value),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("inc-{field:?}")))
                        .w(px(NUMERIC_STEP_BUTTON_SIZE))
                        .h(px(NUMERIC_STEP_BUTTON_SIZE))
                        .rounded_sm()
                        .cursor_pointer()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(bg_card)
                        .text_color(text_primary)
                        .text_sm()
                        .child("+")
                        .on_click(cx.listener(move |view, _, _, cx| {
                            cx.stop_propagation();
                            view.step_numeric_field(field, 1, cx);
                        })),
                )
                .into_any_element()
        } else if is_active {
            let font = Font {
                family: self.config.font_family.clone().into(),
                ..Font::default()
            };
            let selection_color = self.accent_with_alpha(0.3);
            TextInputElement::new(
                cx.entity(),
                self.focus_handle.clone(),
                font,
                px(13.0),
                text_secondary.into(),
                selection_color.into(),
                TextInputAlignment::Left,
            )
            .into_any_element()
        } else {
            div()
                .text_sm()
                .text_color(text_secondary)
                .child(display_value)
                .into_any_element()
        };

        let row = div()
            .id(SharedString::from(format!("editable-row-{field:?}")))
            .flex()
            .items_start()
            .gap_4()
            .py_3()
            .px_4()
            .rounded_lg()
            .bg(bg_card)
            .border_1()
            .border_color(if dropdown_open {
                Rgba {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                }
            } else {
                border_color
            })
            .cursor_pointer()
            .when(!is_numeric, |s| {
                s.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |view, event: &MouseDownEvent, window, cx| {
                        cx.stop_propagation();
                        if !view
                            .active_input
                            .as_ref()
                            .is_some_and(|input| input.field == field)
                        {
                            view.begin_editing_field(field, window, cx);
                        }

                        if let Some(input) = view.active_input.as_mut() {
                            let index = input.state.character_index_for_point(event.position);
                            if event.modifiers.shift {
                                input.state.select_to_utf16(index);
                            } else {
                                input.state.set_cursor_utf16(index);
                            }
                            input.selecting = true;
                        }

                        view.focus_handle.focus(window, cx);
                        cx.notify();
                    }),
                )
                .on_mouse_move(
                    cx.listener(move |view, event: &MouseMoveEvent, _window, cx| {
                        let Some(input) = view.active_input.as_mut() else {
                            return;
                        };
                        if input.field != field || !input.selecting || !event.dragging() {
                            return;
                        }
                        let index = input.state.character_index_for_point(event.position);
                        input.state.select_to_utf16(index);
                        cx.notify();
                    }),
                )
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(move |view, _event: &MouseUpEvent, _window, cx| {
                        if let Some(input) = view.active_input.as_mut()
                            && input.field == field
                        {
                            input.selecting = false;
                            cx.notify();
                        }
                    }),
                )
                .on_mouse_up_out(
                    MouseButton::Left,
                    cx.listener(move |view, _event: &MouseUpEvent, _window, cx| {
                        if let Some(input) = view.active_input.as_mut()
                            && input.field == field
                        {
                            input.selecting = false;
                            cx.notify();
                        }
                    }),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_1()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(text_primary)
                            .child(title),
                    )
                    .child(div().text_xs().text_color(text_muted).child(description)),
            )
            .child(
                div()
                    .when(is_numeric, |s| s.w(px(NUMERIC_INPUT_WIDTH)).flex_none())
                    .when(!is_numeric, |s| {
                        s.flex_1().min_w(px(220.0)).max_w(px(560.0))
                    })
                    .relative()
                    .h(if is_numeric {
                        px(NUMERIC_INPUT_HEIGHT)
                    } else {
                        px(28.0)
                    })
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        div()
                            .h_full()
                            .px_2()
                            .rounded_md()
                            .bg(input_bg)
                            .border_1()
                            .border_color(if is_active && accent_inner_border {
                                accent.into()
                            } else {
                                border_color
                            })
                            .overflow_hidden()
                            .child(value_element),
                    )
                    .when_some(dropdown, |s, dropdown| s.child(dropdown)),
            );

        self.wrap_setting_with_scroll_anchor(search_key, row.into_any_element())
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.keystroke.modifiers.secondary()
            && !event.keystroke.modifiers.alt
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.function
            && event.keystroke.key.eq_ignore_ascii_case("w")
        {
            window.remove_window();
            return;
        }

        let cmd_only = event.keystroke.modifiers.secondary()
            && !event.keystroke.modifiers.alt
            && !event.keystroke.modifiers.control
            && !event.keystroke.modifiers.function;

        if self.active_input.is_none() && !self.sidebar_search_active {
            return;
        }

        if self.sidebar_search_active && self.active_input.is_none() {
            if cmd_only && event.keystroke.key.eq_ignore_ascii_case("a") {
                self.sidebar_search_state.select_all();
                cx.notify();
                return;
            }

            match event.keystroke.key.as_str() {
                "enter" => self.jump_to_first_search_result(window, cx),
                "escape" => {
                    self.blur_sidebar_search();
                    cx.notify();
                }
                "backspace" => {
                    self.sidebar_search_state.delete_backward();
                    self.refresh_search_navigation(window, cx);
                }
                "delete" => {
                    self.sidebar_search_state.delete_forward();
                    self.refresh_search_navigation(window, cx);
                }
                "left" => {
                    self.sidebar_search_state.move_left();
                    cx.notify();
                }
                "right" => {
                    self.sidebar_search_state.move_right();
                    cx.notify();
                }
                "home" => {
                    self.sidebar_search_state.move_to_start();
                    cx.notify();
                }
                "end" => {
                    self.sidebar_search_state.move_to_end();
                    cx.notify();
                }
                _ => {}
            }
            return;
        }

        let active_field = self.active_input.as_ref().map(|input| input.field);
        let allow_text_editing = active_field.is_some_and(Self::uses_text_input_for_field);

        if cmd_only
            && event.keystroke.key.eq_ignore_ascii_case("a")
            && let Some(input) = self.active_input.as_mut()
        {
            input.state.select_all();
            cx.notify();
            return;
        }

        match event.keystroke.key.as_str() {
            "enter" => {
                if active_field == Some(EditableField::FontFamily) {
                    if let Some(first) = self
                        .active_input
                        .as_ref()
                        .map(|input| self.filtered_font_suggestions(input.state.text()))
                        .and_then(|items| items.into_iter().next())
                    {
                        self.apply_font_selection(&first, cx);
                    } else {
                        self.cancel_active_input(cx);
                    }
                } else {
                    self.commit_active_input(cx);
                }
            }
            "escape" => self.cancel_active_input(cx),
            "tab" => {
                if self
                    .active_input
                    .as_ref()
                    .is_some_and(|input| input.field == EditableField::Theme)
                    && let Some(first) = self
                        .active_input
                        .as_ref()
                        .map(|input| self.filtered_theme_suggestions(input.state.text()))
                        .and_then(|items| items.into_iter().next())
                {
                    self.apply_theme_selection(&first, cx);
                }
                if self
                    .active_input
                    .as_ref()
                    .is_some_and(|input| input.field == EditableField::FontFamily)
                    && let Some(first) = self
                        .active_input
                        .as_ref()
                        .map(|input| self.filtered_font_suggestions(input.state.text()))
                        .and_then(|items| items.into_iter().next())
                {
                    self.apply_font_selection(&first, cx);
                }
            }
            "backspace" => {
                if allow_text_editing && let Some(input) = self.active_input.as_mut() {
                    input.state.delete_backward();
                }
                cx.notify();
            }
            "delete" => {
                if allow_text_editing && let Some(input) = self.active_input.as_mut() {
                    input.state.delete_forward();
                }
                cx.notify();
            }
            "left" => {
                if allow_text_editing && let Some(input) = self.active_input.as_mut() {
                    input.state.move_left();
                }
                cx.notify();
            }
            "right" => {
                if allow_text_editing && let Some(input) = self.active_input.as_mut() {
                    input.state.move_right();
                }
                cx.notify();
            }
            "home" => {
                if allow_text_editing && let Some(input) = self.active_input.as_mut() {
                    input.state.move_to_start();
                }
                cx.notify();
            }
            "end" => {
                if allow_text_editing && let Some(input) = self.active_input.as_mut() {
                    input.state.move_to_end();
                }
                cx.notify();
            }
            _ => {}
        }
    }

    fn render_cursor_style_row(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let current = self.config.cursor_style;
        let highlight_intensity = self.setting_highlight_intensity("cursor-style");
        let bg_card = self.apply_setting_highlight_bg(self.bg_card(), highlight_intensity);
        let border_color =
            self.apply_setting_highlight_border(self.border_color(), highlight_intensity);
        let text_primary = self.text_primary();
        let text_muted = self.text_muted();
        let text_secondary = self.text_secondary();
        let accent = self.accent();
        let hover_bg = self.bg_hover();
        let switch_off_bg = self.bg_input();
        let selected_text = self.contrasting_text_for_fill(accent, bg_card);

        let row = div()
            .flex()
            .items_center()
            .justify_between()
            .py_3()
            .px_4()
            .rounded_lg()
            .bg(bg_card)
            .border_1()
            .border_color(border_color)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(text_primary)
                            .child("Cursor Style"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child("Shape of the terminal cursor"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child({
                        let is_selected = current == CursorStyle::Block;
                        div()
                            .id("cursor-style-block")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .bg(if is_selected {
                                accent.into()
                            } else {
                                switch_off_bg
                            })
                            .text_color(if is_selected {
                                selected_text
                            } else {
                                text_secondary
                            })
                            .hover(|s| if !is_selected { s.bg(hover_bg) } else { s })
                            .child("Block")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.config.cursor_style = CursorStyle::Block;
                                let _ = set_config_value("cursor_style", "block");
                                cx.notify();
                            }))
                    })
                    .child({
                        let is_selected = current == CursorStyle::Line;
                        div()
                            .id("cursor-style-line")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .bg(if is_selected {
                                accent.into()
                            } else {
                                switch_off_bg
                            })
                            .text_color(if is_selected {
                                selected_text
                            } else {
                                text_secondary
                            })
                            .hover(|s| if !is_selected { s.bg(hover_bg) } else { s })
                            .child("Line")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.config.cursor_style = CursorStyle::Line;
                                let _ = set_config_value("cursor_style", "line");
                                cx.notify();
                            }))
                    }),
            );

        self.wrap_setting_with_scroll_anchor("cursor-style", row.into_any_element())
    }

    fn render_tab_title_mode_row(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let current = self.config.tab_title.mode;
        let highlight_intensity = self.setting_highlight_intensity("title-mode");
        let bg_card = self.apply_setting_highlight_bg(self.bg_card(), highlight_intensity);
        let border_color =
            self.apply_setting_highlight_border(self.border_color(), highlight_intensity);
        let text_primary = self.text_primary();
        let text_muted = self.text_muted();
        let text_secondary = self.text_secondary();
        let accent = self.accent();
        let hover_bg = self.bg_hover();
        let switch_off_bg = self.bg_input();
        let selected_text = self.contrasting_text_for_fill(accent, bg_card);

        let row = div()
            .flex()
            .items_center()
            .justify_between()
            .py_3()
            .px_4()
            .rounded_lg()
            .bg(bg_card)
            .border_1()
            .border_color(border_color)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(text_primary)
                            .child("Title Mode"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(text_muted)
                            .child("How tab titles are determined"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child({
                        let is_selected = current == TabTitleMode::Smart;
                        div()
                            .id("tab-mode-smart")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .bg(if is_selected {
                                accent.into()
                            } else {
                                switch_off_bg
                            })
                            .text_color(if is_selected {
                                selected_text
                            } else {
                                text_secondary
                            })
                            .hover(|s| if !is_selected { s.bg(hover_bg) } else { s })
                            .child("Smart")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.config.tab_title.mode = TabTitleMode::Smart;
                                let _ = set_config_value("tab_title_mode", "smart");
                                cx.notify();
                            }))
                    })
                    .child({
                        let is_selected = current == TabTitleMode::Shell;
                        div()
                            .id("tab-mode-shell")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .bg(if is_selected {
                                accent.into()
                            } else {
                                switch_off_bg
                            })
                            .text_color(if is_selected {
                                selected_text
                            } else {
                                text_secondary
                            })
                            .hover(|s| if !is_selected { s.bg(hover_bg) } else { s })
                            .child("Shell")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.config.tab_title.mode = TabTitleMode::Shell;
                                let _ = set_config_value("tab_title_mode", "shell");
                                cx.notify();
                            }))
                    })
                    .child({
                        let is_selected = current == TabTitleMode::Explicit;
                        div()
                            .id("tab-mode-explicit")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .bg(if is_selected {
                                accent.into()
                            } else {
                                switch_off_bg
                            })
                            .text_color(if is_selected {
                                selected_text
                            } else {
                                text_secondary
                            })
                            .hover(|s| if !is_selected { s.bg(hover_bg) } else { s })
                            .child("Explicit")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.config.tab_title.mode = TabTitleMode::Explicit;
                                let _ = set_config_value("tab_title_mode", "explicit");
                                cx.notify();
                            }))
                    })
                    .child({
                        let is_selected = current == TabTitleMode::Static;
                        div()
                            .id("tab-mode-static")
                            .px_3()
                            .py_1()
                            .rounded_md()
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .bg(if is_selected {
                                accent.into()
                            } else {
                                switch_off_bg
                            })
                            .text_color(if is_selected {
                                selected_text
                            } else {
                                text_secondary
                            })
                            .hover(|s| if !is_selected { s.bg(hover_bg) } else { s })
                            .child("Static")
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.config.tab_title.mode = TabTitleMode::Static;
                                let _ = set_config_value("tab_title_mode", "static");
                                cx.notify();
                            }))
                    }),
            );

        self.wrap_setting_with_scroll_anchor("title-mode", row.into_any_element())
    }

    fn render_appearance_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let background_blur = self.config.background_blur;
        let background_opacity = self.config.background_opacity;
        let theme = self.config.theme.clone();
        let font_family = self.config.font_family.clone();
        let font_size = self.config.font_size;
        let padding_x = self.config.padding_x;
        let padding_y = self.config.padding_y;

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.render_section_header("Appearance", "Customize the look and feel"))
            .child(self.render_group_header("THEME"))
            .child(self.render_editable_row(
                "theme",
                EditableField::Theme,
                "Theme",
                "Current color scheme name",
                theme,
                cx,
            ))
            .child(self.render_group_header("WINDOW"))
            .child(self.render_setting_row(
                "background-blur",
                "blur-toggle",
                "Background Blur",
                "Enable blur effect for transparent backgrounds",
                background_blur,
                cx,
                |view, _cx| {
                    view.config.background_blur = !view.config.background_blur;
                    let _ = set_config_value(
                        "background_blur",
                        &view.config.background_blur.to_string(),
                    );
                },
            ))
            .child(self.render_editable_row(
                "background-opacity",
                EditableField::BackgroundOpacity,
                "Background Opacity",
                "Window transparency (0-100%)",
                format!("{}%", (background_opacity * 100.0) as i32),
                cx,
            ))
            .child(self.render_group_header("FONT"))
            .child(self.render_editable_row(
                "font-family",
                EditableField::FontFamily,
                "Font Family",
                "Font family used in terminal UI",
                font_family,
                cx,
            ))
            .child(self.render_editable_row(
                "font-size",
                EditableField::FontSize,
                "Font Size",
                "Terminal font size in pixels",
                format!("{}px", font_size as i32),
                cx,
            ))
            .child(self.render_group_header("PADDING"))
            .child(self.render_editable_row(
                "padding-x",
                EditableField::PaddingX,
                "Horizontal Padding",
                "Left and right terminal padding",
                format!("{}px", padding_x as i32),
                cx,
            ))
            .child(self.render_editable_row(
                "padding-y",
                EditableField::PaddingY,
                "Vertical Padding",
                "Top and bottom terminal padding",
                format!("{}px", padding_y as i32),
                cx,
            ))
    }

    fn render_terminal_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let cursor_blink = self.config.cursor_blink;
        let term = self.config.term.clone();
        let shell = self
            .config
            .shell
            .clone()
            .unwrap_or_else(|| "System default".to_string());
        let colorterm = self
            .config
            .colorterm
            .clone()
            .unwrap_or_else(|| "Disabled".to_string());
        let scrollback = self.config.scrollback_history;
        let scroll_mult = self.config.mouse_scroll_multiplier;
        let command_palette_show_keybinds = self.config.command_palette_show_keybinds;

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.render_section_header("Terminal", "Configure terminal behavior"))
            .child(self.render_group_header("CURSOR"))
            .child(self.render_setting_row(
                "cursor-blink",
                "cursor-blink-toggle",
                "Cursor Blink",
                "Enable blinking cursor animation",
                cursor_blink,
                cx,
                |view, _cx| {
                    view.config.cursor_blink = !view.config.cursor_blink;
                    let _ = set_config_value("cursor_blink", &view.config.cursor_blink.to_string());
                },
            ))
            .child(self.render_cursor_style_row(cx))
            .child(self.render_group_header("SHELL"))
            .child(self.render_editable_row(
                "shell",
                EditableField::Shell,
                "Shell",
                "Executable for new sessions",
                shell,
                cx,
            ))
            .child(self.render_editable_row(
                "term",
                EditableField::Term,
                "TERM",
                "Terminal type for child apps",
                term,
                cx,
            ))
            .child(self.render_editable_row(
                "colorterm",
                EditableField::Colorterm,
                "COLORTERM",
                "Color support advertisement",
                colorterm,
                cx,
            ))
            .child(self.render_group_header("SCROLLING"))
            .child(self.render_editable_row(
                "scrollback-history",
                EditableField::ScrollbackHistory,
                "Scrollback History",
                "Lines to keep in buffer",
                format!("{} lines", scrollback),
                cx,
            ))
            .child(self.render_editable_row(
                "scroll-multiplier",
                EditableField::ScrollMultiplier,
                "Scroll Multiplier",
                "Mouse wheel scroll speed",
                format!("{}x", scroll_mult),
                cx,
            ))
            .child(self.render_group_header("UI"))
            .child(self.render_setting_row(
                "palette-keybinds",
                "palette-keybinds-toggle",
                "Show Keybindings in Palette",
                "Display keyboard shortcuts in command palette",
                command_palette_show_keybinds,
                cx,
                |view, _cx| {
                    view.config.command_palette_show_keybinds =
                        !view.config.command_palette_show_keybinds;
                    let _ = set_config_value(
                        "command_palette_show_keybinds",
                        &view.config.command_palette_show_keybinds.to_string(),
                    );
                },
            ))
    }

    fn render_tabs_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let use_tabs = self.config.use_tabs;
        let shell_integration = self.config.tab_title.shell_integration;
        let fallback = self.config.tab_title.fallback.clone();

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.render_section_header("Tabs", "Configure tab behavior and titles"))
            .child(self.render_group_header("TAB BAR"))
            .child(self.render_setting_row(
                "use-tabs",
                "use-tabs-toggle",
                "Enable Tabs",
                "Show compact tab strip",
                use_tabs,
                cx,
                |view, _cx| {
                    view.config.use_tabs = !view.config.use_tabs;
                    let _ = set_config_value("use_tabs", &view.config.use_tabs.to_string());
                },
            ))
            .child(self.render_group_header("TAB TITLES"))
            .child(self.render_tab_title_mode_row(cx))
            .child(self.render_setting_row(
                "shell-integration",
                "shell-integration-toggle",
                "Shell Integration",
                "Export TERMY_* env vars for shell hooks",
                shell_integration,
                cx,
                |view, _cx| {
                    view.config.tab_title.shell_integration =
                        !view.config.tab_title.shell_integration;
                    let _ = set_config_value(
                        "tab_title_shell_integration",
                        &view.config.tab_title.shell_integration.to_string(),
                    );
                },
            ))
            .child(self.render_editable_row(
                "fallback-title",
                EditableField::TabFallbackTitle,
                "Fallback Title",
                "Default when no other source available",
                fallback,
                cx,
            ))
    }

    fn render_advanced_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let working_dir = self
            .config
            .working_dir
            .clone()
            .unwrap_or_else(|| "Not set".to_string());
        let window_width = self.config.window_width;
        let window_height = self.config.window_height;
        let bg_card = self.bg_card();
        let border_color = self.border_color();
        let text_muted = self.text_muted();
        let text_secondary = self.text_secondary();
        let accent = self.accent();
        let accent_hover = self.accent_with_alpha(0.8);
        let button_text = self.contrasting_text_for_fill(accent, bg_card);
        let button_hover_text = self.contrasting_text_for_fill(accent_hover, bg_card);

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.render_section_header("Advanced", "Advanced configuration options"))
            .child(self.render_group_header("STARTUP"))
            .child(self.render_editable_row(
                "working-directory",
                EditableField::WorkingDirectory,
                "Working Directory",
                "Initial directory for new sessions",
                working_dir,
                cx,
            ))
            .child(self.render_group_header("WINDOW"))
            .child(self.render_editable_row(
                "window-width",
                EditableField::WindowWidth,
                "Default Width",
                "Window width on startup",
                format!("{}px", window_width as i32),
                cx,
            ))
            .child(self.render_editable_row(
                "window-height",
                EditableField::WindowHeight,
                "Default Height",
                "Window height on startup",
                format!("{}px", window_height as i32),
                cx,
            ))
            .child(self.render_group_header("CONFIG FILE"))
            .child(
                div()
                    .py_4()
                    .px_4()
                    .rounded_lg()
                    .bg(bg_card)
                    .border_1()
                    .border_color(border_color)
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(text_muted)
                            .child("To change these settings, edit the config file:"),
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_family("monospace")
                            .text_color(text_secondary)
                            .child("~/.config/termy/config.txt"),
                    )
                    .child(
                        div()
                            .id("open-config-btn")
                            .mt_2()
                            .px_4()
                            .py_2()
                            .rounded_md()
                            .bg(accent)
                            .text_sm()
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_color(button_text)
                            .cursor_pointer()
                            .hover(move |s| s.bg(accent_hover).text_color(button_hover_text))
                            .child("Open Config File")
                            .on_click(cx.listener(|_view, _, _, cx| {
                                crate::config::open_config_file();
                                cx.notify();
                            })),
                    ),
            )
    }
}

impl TextInputProvider for SettingsWindow {
    fn text_input_state(&self) -> Option<&TextInputState> {
        let settings_input = self
            .active_input
            .as_ref()
            .and_then(|input| Self::uses_text_input_for_field(input.field).then_some(&input.state));

        settings_input.or_else(|| {
            self.sidebar_search_active
                .then_some(&self.sidebar_search_state)
        })
    }

    fn text_input_state_mut(&mut self) -> Option<&mut TextInputState> {
        let settings_input = self.active_input.as_mut().and_then(|input| {
            Self::uses_text_input_for_field(input.field).then_some(&mut input.state)
        });

        if settings_input.is_some() {
            settings_input
        } else if self.sidebar_search_active {
            Some(&mut self.sidebar_search_state)
        } else {
            None
        }
    }
}

impl gpui::EntityInputHandler for SettingsWindow {
    fn text_for_range(
        &mut self,
        range: std::ops::Range<usize>,
        adjusted_range: &mut Option<std::ops::Range<usize>>,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<String> {
        let state = TextInputProvider::text_input_state(self)?;
        Some(state.text_for_range(range, adjusted_range))
    }

    fn selected_text_range(
        &mut self,
        _ignore_disabled_input: bool,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<gpui::UTF16Selection> {
        let state = TextInputProvider::text_input_state(self)?;
        Some(state.selected_text_range())
    }

    fn marked_text_range(
        &self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<std::ops::Range<usize>> {
        let state = TextInputProvider::text_input_state(self)?;
        state.marked_text_range_utf16()
    }

    fn unmark_text(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) {
        if let Some(state) = TextInputProvider::text_input_state_mut(self) {
            state.unmark_text();
        }
    }

    fn replace_text_in_range(
        &mut self,
        range: Option<std::ops::Range<usize>>,
        text: &str,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let mut changed = false;
        if let Some(state) = TextInputProvider::text_input_state_mut(self) {
            state.replace_text_in_range(range, text);
            changed = true;
        }

        if changed {
            self.refresh_search_navigation(window, cx);
        }
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range: Option<std::ops::Range<usize>>,
        new_text: &str,
        new_selected_range: Option<std::ops::Range<usize>>,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) {
        let mut changed = false;
        if let Some(state) = TextInputProvider::text_input_state_mut(self) {
            state.replace_and_mark_text_in_range(range, new_text, new_selected_range);
            changed = true;
        }

        if changed {
            self.refresh_search_navigation(window, cx);
        }
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: std::ops::Range<usize>,
        element_bounds: gpui::Bounds<gpui::Pixels>,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<gpui::Bounds<gpui::Pixels>> {
        let state = TextInputProvider::text_input_state(self)?;
        Some(state.bounds_for_range(range_utf16, element_bounds))
    }

    fn character_index_for_point(
        &mut self,
        point: gpui::Point<gpui::Pixels>,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> Option<usize> {
        let state = TextInputProvider::text_input_state(self)?;
        Some(state.character_index_for_point(point))
    }

    fn accepts_text_input(
        &self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> bool {
        TextInputProvider::text_input_state(self).is_some()
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bg = self.bg_primary();
        div()
            .id("settings-root")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_any_mouse_down(cx.listener(|view, _event: &MouseDownEvent, _window, cx| {
                if view.active_input.is_some() || view.sidebar_search_active {
                    view.active_input = None;
                    view.blur_sidebar_search();
                    cx.notify();
                }
            }))
            .flex()
            .size_full()
            .bg(bg)
            .font_family(self.config.font_family.clone())
            .child(self.render_sidebar(cx))
            .child(
                div()
                    .id("settings-content-scroll")
                    .flex_1()
                    .h_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.content_scroll_handle)
                    .overflow_x_hidden()
                    .p_6()
                    .child(self.render_content(cx)),
            )
    }
}
