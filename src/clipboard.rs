pub struct Clipboard {
    inner: Option<arboard::Clipboard>,
}

impl Clipboard {
    pub fn new() -> Self {
        Self {
            inner: arboard::Clipboard::new().ok(),
        }
    }

    pub fn set_text(&mut self, text: &str) -> bool {
        self.inner
            .as_mut()
            .is_some_and(|clipboard| clipboard.set_text(text).is_ok())
    }

    pub fn get_text(&mut self) -> Option<String> {
        self.inner
            .as_mut()
            .and_then(|clipboard| clipboard.get_text().ok())
    }
}

impl Default for Clipboard {
    fn default() -> Self {
        Self::new()
    }
}
