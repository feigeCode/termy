use super::*;

impl SettingsWindow {
    pub(super) fn render_keybindings_section(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let keybind_meta = Self::setting_metadata("keybind").expect("missing metadata for keybind");
        let bg_card = self.bg_card();
        let border_color = self.border_color();
        let text_muted = self.text_muted();
        let text_secondary = self.text_secondary();
        let structured_rows = if self.config.keybind_lines.is_empty() {
            vec![
                div()
                    .text_sm()
                    .text_color(text_muted)
                    .child("Using built-in default keybindings")
                    .into_any_element(),
            ]
        } else {
            self.config
                .keybind_lines
                .iter()
                .map(|line| {
                    div()
                        .text_sm()
                        .text_color(text_secondary)
                        .font_family("monospace")
                        .child(format!("keybind = {}", line.value))
                        .into_any_element()
                })
                .collect::<Vec<_>>()
        };

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(self.render_section_header(
                "Keybindings",
                "Structured list plus raw directive editor",
            ))
            .child(self.render_group_header("STRUCTURED"))
            .child(
                div()
                    .py_4()
                    .px_4()
                    .rounded(px(0.0))
                    .bg(bg_card)
                    .border_1()
                    .border_color(border_color)
                    .child(div().flex().flex_col().gap_1().children(structured_rows)),
            )
            .child(self.render_group_header("RAW"))
            .child(self.render_editable_row(
                "keybind",
                EditableField::KeybindDirectives,
                keybind_meta.title,
                "Semicolon or newline-separated keybind directives",
                self.editable_field_value(EditableField::KeybindDirectives),
                cx,
            ))
    }
}
