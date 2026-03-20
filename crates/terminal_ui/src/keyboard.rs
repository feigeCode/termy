use alacritty_terminal::term::TermMode;
use gpui::{Keystroke, Modifiers};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TerminalKeyboardMode {
    disambiguate_escape_codes: bool,
    report_event_types: bool,
    report_alternate_keys: bool,
    report_all_keys_as_esc: bool,
    report_associated_text: bool,
}

impl TerminalKeyboardMode {
    pub(crate) fn from_term_mode(mode: TermMode) -> Self {
        Self {
            disambiguate_escape_codes: mode.contains(TermMode::DISAMBIGUATE_ESC_CODES),
            report_event_types: mode.contains(TermMode::REPORT_EVENT_TYPES),
            report_alternate_keys: mode.contains(TermMode::REPORT_ALTERNATE_KEYS),
            report_all_keys_as_esc: mode.contains(TermMode::REPORT_ALL_KEYS_AS_ESC),
            report_associated_text: mode.contains(TermMode::REPORT_ASSOCIATED_TEXT),
        }
    }

    pub fn disambiguate_escape_codes(self) -> bool {
        self.disambiguate_escape_codes
    }

    pub fn report_event_types(self) -> bool {
        self.report_event_types
    }

    pub fn report_all_keys_as_esc(self) -> bool {
        self.report_all_keys_as_esc
    }

    pub fn report_associated_text(self) -> bool {
        self.report_associated_text
    }

    pub fn report_alternate_keys(self) -> bool {
        self.report_alternate_keys
    }

    pub fn enhanced_reporting_active(self) -> bool {
        self.disambiguate_escape_codes
            || self.report_event_types
            || self.report_alternate_keys
            || self.report_all_keys_as_esc
            || self.report_associated_text
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalKeyEventKind {
    Press,
    Repeat,
    Release,
}

pub fn keystroke_to_input(
    keystroke: &Keystroke,
    event_kind: TerminalKeyEventKind,
    keyboard_mode: TerminalKeyboardMode,
    prompt_shortcuts_enabled: bool,
) -> Option<Vec<u8>> {
    if !keyboard_mode.enhanced_reporting_active() {
        return match event_kind {
            TerminalKeyEventKind::Press | TerminalKeyEventKind::Repeat => {
                basic_keystroke_to_input(keystroke, prompt_shortcuts_enabled, true)
            }
            TerminalKeyEventKind::Release => None,
        };
    }

    enhanced_keystroke_to_input(keystroke, event_kind, keyboard_mode).or_else(|| match event_kind {
        TerminalKeyEventKind::Press | TerminalKeyEventKind::Repeat => {
            basic_keystroke_to_input(keystroke, prompt_shortcuts_enabled, false)
        }
        TerminalKeyEventKind::Release => None,
    })
}

fn enhanced_keystroke_to_input(
    keystroke: &Keystroke,
    event_kind: TerminalKeyEventKind,
    keyboard_mode: TerminalKeyboardMode,
) -> Option<Vec<u8>> {
    if matches!(event_kind, TerminalKeyEventKind::Release) && !keyboard_mode.report_event_types() {
        return None;
    }

    let include_event_type = keyboard_mode.report_event_types()
        && matches!(
            event_kind,
            TerminalKeyEventKind::Repeat | TerminalKeyEventKind::Release
        );
    if include_event_type
        && !keyboard_mode.report_all_keys_as_esc()
        && matches!(keystroke.key.as_str(), "enter" | "tab" | "backspace")
    {
        return None;
    }

    if !should_build_enhanced_sequence(keystroke, event_kind, keyboard_mode) {
        return None;
    }

    let modifiers = SequenceModifiers::from_modifiers(keystroke.modifiers);
    let associated_text = associated_text(keystroke, event_kind, keyboard_mode);
    let sequence_base = named_special_sequence_base(
        keystroke,
        modifiers,
        associated_text.is_some(),
        include_event_type,
    )
    .or_else(|| modifier_or_control_sequence_base(keystroke, keyboard_mode))
        .or_else(|| textual_sequence_base(keystroke, keyboard_mode, associated_text.is_some()));

    let SequenceBase { payload, terminator } = sequence_base?;

    let mut sequence = format!("\x1b[{payload}");
    if include_event_type || !modifiers.is_empty() || associated_text.is_some() {
        sequence.push_str(&format!(";{}", modifiers.encode_esc_sequence()));
    }

    if include_event_type {
        sequence.push(':');
        let event_code = match event_kind {
            TerminalKeyEventKind::Press => '1',
            TerminalKeyEventKind::Repeat => '2',
            TerminalKeyEventKind::Release => '3',
        };
        sequence.push(event_code);
    }

    if let Some(text) = associated_text {
        let mut codepoints = text.chars().map(u32::from);
        if let Some(first) = codepoints.next() {
            sequence.push_str(&format!(";{first}"));
        }
        for codepoint in codepoints {
            sequence.push_str(&format!(":{codepoint}"));
        }
    }

    sequence.push(terminator);
    Some(sequence.into_bytes())
}

fn should_build_enhanced_sequence(
    keystroke: &Keystroke,
    event_kind: TerminalKeyEventKind,
    keyboard_mode: TerminalKeyboardMode,
) -> bool {
    if keyboard_mode.report_all_keys_as_esc() {
        return true;
    }

    if matches!(event_kind, TerminalKeyEventKind::Release) {
        return keyboard_mode.report_event_types();
    }

    if is_named_special_key(keystroke.key.as_str()) {
        return true;
    }

    if is_modifier_key(keystroke.key.as_str()) {
        return false;
    }

    if keyboard_mode.disambiguate_escape_codes()
        && should_disambiguate_escape_code(keystroke.key.as_str(), keystroke.modifiers)
    {
        return true;
    }

    if is_basic_named_control_key(keystroke.key.as_str()) {
        return false;
    }

    let has_plain_text = keystroke
        .key_char
        .as_deref()
        .is_some_and(|text| !text.is_empty())
        || (keystroke.key.chars().count() == 1
            && !keystroke.modifiers.control
            && !keystroke.modifiers.alt
            && !keystroke.modifiers.platform
            && !keystroke.modifiers.function);
    !has_plain_text
}

fn should_disambiguate_escape_code(key: &str, modifiers: Modifiers) -> bool {
    if key == "escape" {
        return true;
    }

    let only_shift = modifiers.shift
        && !modifiers.control
        && !modifiers.alt
        && !modifiers.platform
        && !modifiers.function;
    !modifiers_are_empty(modifiers)
        && (!only_shift || matches!(key, "tab" | "enter" | "backspace"))
}

fn modifiers_are_empty(modifiers: Modifiers) -> bool {
    !modifiers.control
        && !modifiers.alt
        && !modifiers.shift
        && !modifiers.platform
        && !modifiers.function
}

fn associated_text<'a>(
    keystroke: &'a Keystroke,
    event_kind: TerminalKeyEventKind,
    keyboard_mode: TerminalKeyboardMode,
) -> Option<&'a str> {
    if !keyboard_mode.report_associated_text()
        || matches!(event_kind, TerminalKeyEventKind::Release)
    {
        return None;
    }

    let text = keystroke.key_char.as_deref()?;
    if text.is_empty() || is_control_character(text) {
        return None;
    }
    Some(text)
}

fn basic_keystroke_to_input(
    keystroke: &Keystroke,
    prompt_shortcuts_enabled: bool,
    allow_prompt_shortcuts: bool,
) -> Option<Vec<u8>> {
    if allow_prompt_shortcuts
        && let Some(modified_input) =
            modified_special_keystroke_input(keystroke, prompt_shortcuts_enabled)
    {
        return Some(modified_input.to_vec());
    }

    let key = keystroke.key.as_str();
    let modifiers = keystroke.modifiers;

    let input = match key {
        "enter" => {
            if modifiers.shift {
                Some(vec![b'\n'])
            } else {
                Some(vec![b'\r'])
            }
        }
        "tab" => Some(vec![b'\t']),
        "escape" => Some(vec![0x1b]),
        "backspace" => Some(vec![0x7f]),
        "delete" => Some(b"\x1b[3~".to_vec()),
        "up" => Some(b"\x1b[A".to_vec()),
        "down" => Some(b"\x1b[B".to_vec()),
        "right" => Some(b"\x1b[C".to_vec()),
        "left" => Some(b"\x1b[D".to_vec()),
        "home" => Some(b"\x1b[H".to_vec()),
        "end" => Some(b"\x1b[F".to_vec()),
        "pageup" => Some(b"\x1b[5~".to_vec()),
        "pagedown" => Some(b"\x1b[6~".to_vec()),
        "space" => Some(vec![b' ']),
        _ => None,
    };

    if let Some(input) = input {
        return Some(input);
    }

    if modifiers.control && !modifiers.platform && !modifiers.function && key.len() == 1 {
        let c = key.chars().next().unwrap();
        if c.is_ascii_alphabetic() {
            let ctrl_char = (c.to_ascii_lowercase() as u8) - b'a' + 1;
            return Some(vec![ctrl_char]);
        }
    }

    if !modifiers.control
        && !modifiers.platform
        && !modifiers.function
        && let Some(key_char) = keystroke.key_char.as_deref()
        && !key_char.is_empty()
    {
        return Some(key_char.as_bytes().to_vec());
    }

    if !modifiers.control && !modifiers.platform && !modifiers.function && key.len() == 1 {
        let c = key.chars().next().unwrap();
        if c.is_ascii() {
            return Some(vec![c as u8]);
        }

        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        return Some(s.as_bytes().to_vec());
    }

    None
}

fn named_special_sequence_base(
    keystroke: &Keystroke,
    modifiers: SequenceModifiers,
    has_associated_text: bool,
    include_event_type: bool,
) -> Option<SequenceBase> {
    let key = keystroke.key.as_str();
    let one_based = if modifiers.is_empty() && !has_associated_text && !include_event_type {
        ""
    } else {
        "1"
    };

    let (payload, terminator) = match key {
        "pageup" => ("5".to_string(), '~'),
        "pagedown" => ("6".to_string(), '~'),
        "delete" => ("3".to_string(), '~'),
        "insert" => ("2".to_string(), '~'),
        "home" => (one_based.to_string(), 'H'),
        "end" => (one_based.to_string(), 'F'),
        "left" => (one_based.to_string(), 'D'),
        "right" => (one_based.to_string(), 'C'),
        "up" => (one_based.to_string(), 'A'),
        "down" => (one_based.to_string(), 'B'),
        "f1" => (one_based.to_string(), 'P'),
        "f2" => (one_based.to_string(), 'Q'),
        "f3" => (one_based.to_string(), 'R'),
        "f4" => (one_based.to_string(), 'S'),
        "f5" => ("15".to_string(), '~'),
        "f6" => ("17".to_string(), '~'),
        "f7" => ("18".to_string(), '~'),
        "f8" => ("19".to_string(), '~'),
        "f9" => ("20".to_string(), '~'),
        "f10" => ("21".to_string(), '~'),
        "f11" => ("23".to_string(), '~'),
        "f12" => ("24".to_string(), '~'),
        "f13" => ("25".to_string(), '~'),
        "f14" => ("26".to_string(), '~'),
        "f15" => ("28".to_string(), '~'),
        "f16" => ("29".to_string(), '~'),
        "f17" => ("31".to_string(), '~'),
        "f18" => ("32".to_string(), '~'),
        "f19" => ("33".to_string(), '~'),
        "f20" => ("34".to_string(), '~'),
        _ => return None,
    };

    Some(SequenceBase::new(payload, terminator))
}

fn modifier_or_control_sequence_base(
    keystroke: &Keystroke,
    keyboard_mode: TerminalKeyboardMode,
) -> Option<SequenceBase> {
    let payload = match keystroke.key.as_str() {
        "tab" => "9",
        "enter" => "13",
        "escape" => "27",
        "space" => "32",
        "backspace" => "127",
        "shift" => {
            if !keyboard_mode.report_all_keys_as_esc() {
                return None;
            }
            "57447"
        }
        "control" => {
            if !keyboard_mode.report_all_keys_as_esc() {
                return None;
            }
            "57448"
        }
        "alt" => {
            if !keyboard_mode.report_all_keys_as_esc() {
                return None;
            }
            "57449"
        }
        "super" | "cmd" => {
            if !keyboard_mode.report_all_keys_as_esc() {
                return None;
            }
            "57450"
        }
        _ => return None,
    };

    Some(SequenceBase::new(payload.to_string(), 'u'))
}

fn textual_sequence_base(
    keystroke: &Keystroke,
    keyboard_mode: TerminalKeyboardMode,
    has_associated_text: bool,
) -> Option<SequenceBase> {
    if keystroke.key.chars().count() == 1 {
        let ch = keystroke.key.chars().next().unwrap();
        let unshifted = if keystroke.modifiers.shift {
            ch.to_lowercase().next().unwrap_or(ch)
        } else {
            ch
        };

        let unicode_key_code = u32::from(unshifted);
        let alternate_key_code = u32::from(ch);
        let payload = if keyboard_mode.report_alternate_keys()
            && alternate_key_code != unicode_key_code
        {
            format!("{unicode_key_code}:{alternate_key_code}")
        } else {
            unicode_key_code.to_string()
        };

        return Some(SequenceBase::new(payload, 'u'));
    }

    if keyboard_mode.report_all_keys_as_esc() && has_associated_text {
        return Some(SequenceBase::new("0".to_string(), 'u'));
    }

    None
}

fn is_control_character(text: &str) -> bool {
    let codepoint = text.bytes().next().unwrap();
    text.len() == 1 && (codepoint < 0x20 || (0x7f..=0x9f).contains(&codepoint))
}

fn is_basic_named_control_key(key: &str) -> bool {
    matches!(key, "tab" | "enter" | "escape" | "backspace" | "space")
}

fn is_named_special_key(key: &str) -> bool {
    matches!(
        key,
        "delete"
            | "insert"
            | "home"
            | "end"
            | "left"
            | "right"
            | "up"
            | "down"
            | "pageup"
            | "pagedown"
            | "f1"
            | "f2"
            | "f3"
            | "f4"
            | "f5"
            | "f6"
            | "f7"
            | "f8"
            | "f9"
            | "f10"
            | "f11"
            | "f12"
            | "f13"
            | "f14"
            | "f15"
            | "f16"
            | "f17"
            | "f18"
            | "f19"
            | "f20"
    )
}

fn is_modifier_key(key: &str) -> bool {
    matches!(key, "shift" | "control" | "alt" | "super" | "cmd")
}

fn modified_special_keystroke_input(
    keystroke: &Keystroke,
    prompt_shortcuts_enabled: bool,
) -> Option<&'static [u8]> {
    let key = keystroke.key.as_str();
    let modifiers = keystroke.modifiers;
    #[cfg(target_os = "macos")]
    let _ = prompt_shortcuts_enabled;

    #[cfg(target_os = "macos")]
    {
        if is_plain_alt(modifiers) {
            return match key {
                "left" => Some(b"\x1bb"),
                "right" => Some(b"\x1bf"),
                "backspace" => Some(b"\x1b\x7f"),
                "delete" => Some(b"\x1bd"),
                _ => None,
            };
        }

        if is_plain_platform(modifiers) {
            return match key {
                "left" | "home" => Some(b"\x01"),
                "right" | "end" => Some(b"\x05"),
                "backspace" => Some(b"\x15"),
                "delete" => Some(b"\x0b"),
                _ => None,
            };
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        if prompt_shortcuts_enabled && is_plain_control(modifiers) {
            return match key {
                "left" => Some(b"\x1bb"),
                "right" => Some(b"\x1bf"),
                "backspace" => Some(b"\x17"),
                "delete" => Some(b"\x1bd"),
                _ => None,
            };
        }
    }

    None
}

#[cfg(target_os = "macos")]
#[inline]
fn is_plain_alt(modifiers: Modifiers) -> bool {
    modifiers.alt
        && !modifiers.control
        && !modifiers.platform
        && !modifiers.shift
        && !modifiers.function
}

#[cfg(target_os = "macos")]
#[inline]
fn is_plain_platform(modifiers: Modifiers) -> bool {
    modifiers.platform
        && !modifiers.control
        && !modifiers.alt
        && !modifiers.shift
        && !modifiers.function
}

#[cfg(not(target_os = "macos"))]
#[inline]
fn is_plain_control(modifiers: Modifiers) -> bool {
    modifiers.control
        && !modifiers.platform
        && !modifiers.alt
        && !modifiers.shift
        && !modifiers.function
}

#[derive(Debug, Clone)]
struct SequenceBase {
    payload: String,
    terminator: char,
}

impl SequenceBase {
    fn new(payload: String, terminator: char) -> Self {
        Self { payload, terminator }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SequenceModifiers(u8);

impl SequenceModifiers {
    const SHIFT: u8 = 1 << 0;
    const ALT: u8 = 1 << 1;
    const CONTROL: u8 = 1 << 2;
    const SUPER: u8 = 1 << 3;

    fn from_modifiers(modifiers: Modifiers) -> Self {
        let mut encoded = 0;
        if modifiers.shift {
            encoded |= Self::SHIFT;
        }
        if modifiers.alt {
            encoded |= Self::ALT;
        }
        if modifiers.control {
            encoded |= Self::CONTROL;
        }
        if modifiers.platform {
            encoded |= Self::SUPER;
        }
        Self(encoded)
    }

    fn encode_esc_sequence(self) -> u8 {
        self.0 + 1
    }

    fn is_empty(self) -> bool {
        self.0 == 0
    }
}

#[cfg(test)]
mod tests {
    use super::{TerminalKeyEventKind, TerminalKeyboardMode, keystroke_to_input};
    use gpui::{Keystroke, Modifiers};

    fn keystroke(key: &str, key_char: Option<&str>, modifiers: Modifiers) -> Keystroke {
        Keystroke {
            modifiers,
            key: key.to_string(),
            key_char: key_char.map(str::to_string),
        }
    }

    fn report_all_mode() -> TerminalKeyboardMode {
        TerminalKeyboardMode {
            report_all_keys_as_esc: true,
            ..TerminalKeyboardMode::default()
        }
    }

    fn report_all_with_event_types_mode() -> TerminalKeyboardMode {
        TerminalKeyboardMode {
            report_all_keys_as_esc: true,
            report_event_types: true,
            ..TerminalKeyboardMode::default()
        }
    }

    #[test]
    fn enhanced_mode_reports_modifier_only_press() {
        let modifiers = Modifiers {
            platform: true,
            ..Modifiers::default()
        };

        assert_eq!(
            keystroke_to_input(
                &keystroke("super", None, modifiers),
                TerminalKeyEventKind::Press,
                report_all_mode(),
                true,
            ),
            Some(b"\x1b[57450;9u".to_vec())
        );
    }

    #[test]
    fn enhanced_mode_reports_modifier_only_release_with_event_types() {
        let modifiers = Modifiers::default();

        assert_eq!(
            keystroke_to_input(
                &keystroke("super", None, modifiers),
                TerminalKeyEventKind::Release,
                report_all_with_event_types_mode(),
                true,
            ),
            Some(b"\x1b[57450;1:3u".to_vec())
        );
    }

    #[test]
    fn enhanced_mode_suppresses_prompt_shortcuts() {
        let modifiers = Modifiers {
            platform: true,
            ..Modifiers::default()
        };

        assert_eq!(
            keystroke_to_input(
                &keystroke("left", None, modifiers),
                TerminalKeyEventKind::Press,
                TerminalKeyboardMode {
                    disambiguate_escape_codes: true,
                    ..TerminalKeyboardMode::default()
                },
                true,
            ),
            Some(b"\x1b[1;9D".to_vec())
        );
    }

    #[test]
    fn enhanced_mode_reports_text_key_releases() {
        let modifiers = Modifiers {
            platform: true,
            shift: true,
            ..Modifiers::default()
        };

        assert_eq!(
            keystroke_to_input(
                &keystroke("t", None, modifiers),
                TerminalKeyEventKind::Release,
                TerminalKeyboardMode {
                    disambiguate_escape_codes: true,
                    report_event_types: true,
                    ..TerminalKeyboardMode::default()
                },
                true,
            ),
            Some(b"\x1b[116;10:3u".to_vec())
        );
    }

    #[test]
    fn enhanced_mode_reports_arrow_releases_with_one_based_payload() {
        assert_eq!(
            keystroke_to_input(
                &keystroke("left", None, Modifiers::default()),
                TerminalKeyEventKind::Release,
                TerminalKeyboardMode {
                    report_event_types: true,
                    ..TerminalKeyboardMode::default()
                },
                true,
            ),
            Some(b"\x1b[1;1:3D".to_vec())
        );
    }

    #[test]
    fn enhanced_mode_skips_enter_release_without_report_all_keys() {
        assert_eq!(
            keystroke_to_input(
                &keystroke("enter", None, Modifiers::default()),
                TerminalKeyEventKind::Release,
                TerminalKeyboardMode {
                    report_event_types: true,
                    ..TerminalKeyboardMode::default()
                },
                true,
            ),
            None
        );
    }

    #[test]
    fn legacy_mode_keeps_plain_control_bytes() {
        let modifiers = Modifiers {
            control: true,
            ..Modifiers::default()
        };

        assert_eq!(
            keystroke_to_input(
                &keystroke("t", None, modifiers),
                TerminalKeyEventKind::Press,
                TerminalKeyboardMode::default(),
                true,
            ),
            Some(vec![0x14])
        );
    }
}
