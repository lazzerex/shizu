use std::time::{Duration, Instant};

use crate::anim::{FRAME_INTERVAL, MIN_HEIGHT, MIN_WIDTH, Mode, PetalView};
use crate::view::layout_line;

use super::Editor;

impl Editor {
    pub fn set_animation_mode(&mut self, mode: Mode) {
        self.mode = mode;
        self.bloom.clear();
    }

    pub fn welcome_visible(&self) -> bool {
        self.buffer().path().is_none() && self.buffer().is_empty()
    }

    pub fn note_input(&mut self, now: Instant) {
        self.idle.note_input(now);
        self.bloom.clear();
    }

    pub fn animation_visible(&self, now: Instant) -> bool {
        if self.animation_area_too_small() {
            return false;
        }
        match self.mode {
            Mode::Off => false,
            Mode::Welcome => self.welcome_visible(),
            Mode::Idle => {
                self.welcome_visible() || (self.idle.expired(now) && self.has_text_gaps())
            }
        }
    }

    pub fn next_wait(&self, now: Instant) -> Option<Duration> {
        if self.animation_area_too_small() {
            return None;
        }
        match self.mode {
            Mode::Off => None,
            Mode::Welcome => self.welcome_visible().then_some(FRAME_INTERVAL),
            Mode::Idle => {
                if self.welcome_visible() {
                    Some(FRAME_INTERVAL)
                } else if self.idle.expired(now) {
                    self.has_text_gaps().then_some(FRAME_INTERVAL)
                } else {
                    Some(self.idle.remaining(now))
                }
            }
        }
    }

    pub fn tick(&mut self, now: Instant) -> bool {
        if !self.animation_visible(now) {
            return self.bloom.clear();
        }
        let width = u16::try_from(self.text_width()).unwrap_or(u16::MAX);
        let height = u16::try_from(self.text_height()).unwrap_or(u16::MAX);
        self.bloom.tick(width, height)
    }

    pub fn petals(&self) -> impl Iterator<Item = PetalView> + '_ {
        self.bloom.petals()
    }

    #[cfg(test)]
    pub(crate) fn set_animation_seed(&mut self, seed: u64) {
        self.bloom = crate::anim::Bloom::with_glyphs(seed, crate::anim::Glyphs::Ascii);
    }

    fn animation_area_too_small(&self) -> bool {
        self.text_width() < usize::from(MIN_WIDTH) || self.text_height() < usize::from(MIN_HEIGHT)
    }

    fn has_text_gaps(&self) -> bool {
        let width = self.text_width();
        let height = self.text_height();
        if width == 0 || height == 0 {
            return false;
        }
        let buffer = self.buffer();
        let first = self.tabs[self.active].view.scroll_line();
        let scroll_col = self.tabs[self.active].view.scroll_col();
        let tab_width = self.tabs[self.active].view.tab_width();
        for row in 0..height {
            let line = first + row;
            if line >= buffer.len_lines() {
                return true;
            }
            let used: usize = layout_line(&buffer.line_text(line), tab_width)
                .iter()
                .map(|cell| cell.width)
                .sum();
            if used < scroll_col.saturating_add(width) {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, Instant};

    use super::Editor;
    use crate::anim::{FRAME_INTERVAL, IDLE_DELAY, Mode};
    use crate::buffer::Buffer;
    use crate::command::Command;
    use crate::test_support::TempDir;

    fn editor(text: &str) -> Editor {
        let mut editor = Editor::new(Buffer::from_text(text));
        editor.resize(40, 10);
        editor
    }

    #[test]
    fn off_mode_never_requests_ticks() {
        let now = Instant::now();
        let mut editor = editor("abc");
        editor.set_animation_mode(Mode::Off);
        editor.note_input(now);
        let later = now + IDLE_DELAY;

        assert_eq!(editor.next_wait(now), None);
        assert_eq!(editor.next_wait(later), None);
        assert!(!editor.animation_visible(later));
        assert!(!editor.tick(later));
        assert_eq!(editor.petals().count(), 0);
    }

    #[test]
    fn idle_animation_waits_for_delay() {
        let now = Instant::now();
        let mut editor = editor("abc");
        editor.note_input(now);

        assert_eq!(editor.next_wait(now), Some(IDLE_DELAY));
        assert!(!editor.animation_visible(now));
        assert!(editor.animation_visible(now + IDLE_DELAY));
        assert_eq!(editor.next_wait(now + IDLE_DELAY), Some(FRAME_INTERVAL));
        assert!(editor.tick(now + IDLE_DELAY));
        assert!(editor.petals().count() >= 2);
    }

    #[test]
    fn input_stops_idle_animation_and_restarts_timer() {
        let start = Instant::now();
        let mut editor = editor("abc");
        editor.note_input(start);
        let active = start + IDLE_DELAY;
        assert!(editor.tick(active));

        let stop = active + Duration::from_millis(10);
        assert!(editor.animation_visible(active));
        editor.note_input(stop);

        assert!(!editor.animation_visible(stop));
        assert_eq!(editor.petals().count(), 0);
        assert_eq!(editor.next_wait(stop), Some(IDLE_DELAY));
    }

    #[test]
    fn full_screen_of_text_never_animates() {
        let text = (0..9)
            .map(|_| "x".repeat(60))
            .collect::<Vec<_>>()
            .join("\n");
        let now = Instant::now();
        let mut editor = editor(&text);
        editor.resize(40, 10);
        editor.note_input(now);
        let later = now + IDLE_DELAY;

        assert!(!editor.animation_visible(later));
        assert_eq!(editor.next_wait(later), None);
        assert!(!editor.tick(later));
        assert_eq!(editor.petals().count(), 0);
    }

    #[test]
    fn welcome_mode_animates_only_the_welcome_screen() {
        let now = Instant::now();
        let mut editor = Editor::new(Buffer::new());
        editor.resize(40, 10);
        editor.set_animation_mode(Mode::Welcome);
        editor.note_input(now);

        assert!(editor.welcome_visible());
        assert_eq!(editor.next_wait(now), Some(FRAME_INTERVAL));

        editor.apply(Command::Insert('x'));
        assert!(!editor.welcome_visible());
        assert_eq!(editor.next_wait(Instant::now()), None);
    }

    #[test]
    fn opened_empty_file_is_not_a_welcome_screen() {
        let dir = TempDir::new();
        let path = dir.join("empty.txt");
        fs::write(&path, "").unwrap();
        let editor = Editor::new(Buffer::open(&path).unwrap());

        assert!(!editor.welcome_visible());
    }

    #[test]
    fn tiny_terminal_pauses_animation() {
        let now = Instant::now();
        let mut editor = editor("abc");
        editor.resize(15, 5);
        editor.note_input(now);
        let later = now + IDLE_DELAY;

        assert_eq!(editor.next_wait(later), None);
        assert!(!editor.tick(later));
        assert_eq!(editor.petals().count(), 0);
    }
}
