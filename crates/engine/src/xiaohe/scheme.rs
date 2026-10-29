use crate::types::{QueryRequest, SchemeKey, SchemeType};

#[derive(Default)]
pub struct XiaoheScheme {
    raw: String,
}

impl XiaoheScheme {
    pub fn reset(&mut self) {
        self.raw.clear();
    }

    pub fn handle_key(&mut self, key: SchemeKey) -> bool {
        match key {
            SchemeKey::Backspace => self.raw.pop().is_some(),
            SchemeKey::Letter(letter) if letter.is_ascii_lowercase() => {
                let limit = if self.raw.starts_with(';') { 2 } else { 4 };
                if self.raw.len() >= limit {
                    return false;
                }
                self.raw.push(char::from(letter));
                true
            }
            SchemeKey::Apostrophe if !self.raw.is_empty() && !self.raw.ends_with('\'') => {
                self.raw.push('\'');
                true
            }
            SchemeKey::Semicolon if self.raw.is_empty() => {
                self.raw.push(';');
                true
            }
            _ => false,
        }
    }

    pub fn build_request(&self) -> QueryRequest {
        let mut request = QueryRequest::default();
        self.build_request_into(&mut request);
        request
    }

    pub fn build_request_into(&self, request: &mut QueryRequest) {
        request.scheme = SchemeType::XiaoheYinxing;
        request.raw_input.clone_from(&self.raw);
        request.raw_input_with_cases.clone_from(&self.raw);
        request.normalized_input.clone_from(&self.raw);
        request.raw_segmentation.clone_from(&self.raw);
        request.normalized_segmentation.clone_from(&self.raw);
        request.segmentation.clone_from(&self.raw);
        request.valid = super::valid_code(&self.raw);
    }

    pub fn preedit(&self) -> String {
        self.raw.clone()
    }

    pub fn set_raw_input(&mut self, raw: &str) {
        self.raw.clear();
        if super::valid_code(raw) {
            self.raw.push_str(raw);
        }
    }
}
