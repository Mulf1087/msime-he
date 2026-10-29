use super::input::InputSession;
use crate::types::{KeyResult, SchemeKey};

impl InputSession {
    /// 参考 Rime：四码唯一时提交；重码等选择或第五键顶屏；四码空码由下一字母清掉。
    pub(super) fn handle_xiaohe_character(&mut self, value: u8) -> KeyResult {
        if value == b'\t' {
            self.reset_composition();
            return KeyResult::handled();
        }
        let raw = &self.engine.request().raw_input;
        if value == b';' && !raw.is_empty() {
            return if self.candidates().len() >= 2 {
                self.select_candidate(1)
            } else {
                KeyResult::handled()
            };
        }
        let full = raw.len() >= if raw.starts_with(';') { 2 } else { 4 };
        let mut previous = None;
        if full && value.is_ascii_lowercase() {
            if self.candidates().is_empty() {
                self.reset_composition();
            } else {
                previous = self.select_candidate(0).commit;
            }
        }
        let key = match value {
            b';' => SchemeKey::Semicolon,
            b'\'' => SchemeKey::Apostrophe,
            _ => SchemeKey::Letter(value),
        };
        if !self.engine.handle_key(key) {
            return KeyResult {
                handled: false,
                commit: previous,
                diagnostic: None,
            };
        }
        self.update_mixed_candidates();
        let raw = &self.engine.request().raw_input;
        let complete = raw.len() == 4 || (raw.starts_with(';') && raw.len() == 2);
        let mut result = if complete && self.candidates().len() == 1 {
            self.select_candidate(0)
        } else {
            KeyResult::handled()
        };
        if let Some(mut text) = previous {
            if let Some(current) = result.commit.take() {
                text.push_str(&current);
            }
            result.commit = Some(text);
        }
        result
    }
}
