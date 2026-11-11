use super::{Editor, Focus, PromptField, record};
use crate::buffer::CharIdx;
use crate::search;

impl Editor {
    pub(super) fn open_prompt(&mut self, focus: Focus) {
        self.focus = focus;
        self.prompt.field = PromptField::Find;
        match focus {
            Focus::Find | Focus::Replace => {
                self.prompt.query.clear();
                self.prompt.replacement.clear();
                self.prompt.matches.clear();
                self.prompt.current = None;
                self.prompt.origin = self.view.cursor();
            }
            Focus::GoToLine => self.prompt.goto.clear(),
            Focus::Editor => {}
        }
    }

    pub(super) fn refresh_matches(&mut self) {
        let index = self.view.buffer_id().index();
        let text = self.buffers[index].text();
        let starts = search::find_all(&text, &self.prompt.query);
        let len = self.prompt.query.chars().count();
        self.prompt.matches = starts
            .iter()
            .map(|&start| (CharIdx::new(start), CharIdx::new(start + len)))
            .collect();
        self.prompt.current = if self.prompt.matches.is_empty() {
            None
        } else {
            Some(
                self.prompt
                    .matches
                    .iter()
                    .position(|(from, _)| *from >= self.prompt.origin)
                    .unwrap_or(0),
            )
        };
    }

    pub(super) fn reveal_match(&mut self) {
        if let Some(current) = self.prompt.current {
            let at = self.prompt.matches[current].0;
            self.view.set_cursor(at);
        }
    }

    pub(super) fn find_step(&mut self, delta: isize) {
        if self.prompt.query.is_empty() {
            return;
        }
        if self.prompt.matches.is_empty() {
            self.prompt.origin = self.view.cursor();
            self.refresh_matches();
        }
        let len = self.prompt.matches.len();
        if len == 0 {
            self.message = Some(format!("no matches for \"{}\"", self.prompt.query));
            return;
        }
        let current = match self.prompt.current {
            Some(current) => {
                let step = delta.unsigned_abs() % len;
                if delta < 0 {
                    (current + len - step) % len
                } else {
                    (current + step) % len
                }
            }
            None if delta < 0 => len - 1,
            None => 0,
        };
        self.prompt.current = Some(current);
        let at = self.prompt.matches[current].0;
        self.view.set_cursor(at);
        self.message = None;
    }

    pub(super) fn prompt_insert(&mut self, character: char) {
        match self.focus {
            Focus::Find => {
                self.prompt.query.push(character);
                self.refresh_matches();
                self.reveal_match();
            }
            Focus::Replace => match self.prompt.field {
                PromptField::Find => {
                    self.prompt.query.push(character);
                    self.refresh_matches();
                    self.reveal_match();
                }
                PromptField::Replace => self.prompt.replacement.push(character),
            },
            Focus::GoToLine => self.prompt.goto.push(character),
            Focus::Editor => {}
        }
    }

    pub(super) fn prompt_backspace(&mut self) {
        match self.focus {
            Focus::Find => {
                self.prompt.query.pop();
                self.refresh_matches();
                self.reveal_match();
            }
            Focus::Replace => match self.prompt.field {
                PromptField::Find => {
                    self.prompt.query.pop();
                    self.refresh_matches();
                    self.reveal_match();
                }
                PromptField::Replace => {
                    self.prompt.replacement.pop();
                }
            },
            Focus::GoToLine => {
                self.prompt.goto.pop();
            }
            Focus::Editor => {}
        }
    }

    pub(super) fn prompt_enter(&mut self) {
        match self.focus {
            Focus::Find | Focus::Replace => self.find_step(1),
            Focus::GoToLine => self.goto_line(),
            Focus::Editor => {}
        }
    }

    pub(super) fn prompt_previous(&mut self) {
        if matches!(self.focus, Focus::Find | Focus::Replace) {
            self.find_step(-1);
        }
    }

    pub(super) fn prompt_toggle_field(&mut self) {
        if self.focus == Focus::Replace {
            self.prompt.field = match self.prompt.field {
                PromptField::Find => PromptField::Replace,
                PromptField::Replace => PromptField::Find,
            };
        }
    }

    pub(super) fn replace_all(&mut self) {
        if !matches!(self.focus, Focus::Find | Focus::Replace) || self.prompt.query.is_empty() {
            return;
        }
        if self.prompt.matches.is_empty() {
            self.prompt.origin = self.view.cursor();
            self.refresh_matches();
        }
        let matches: Vec<(CharIdx, CharIdx)> = self.prompt.matches.iter().copied().fold(
            Vec::new(),
            |mut kept: Vec<(CharIdx, CharIdx)>, candidate| {
                if kept.last().is_none_or(|previous| candidate.0 >= previous.1) {
                    kept.push(candidate);
                }
                kept
            },
        );
        let count = matches.len();
        if count == 0 {
            self.message = Some(format!("no matches for \"{}\"", self.prompt.query));
            return;
        }
        let replacement = self.prompt.replacement.clone();
        let index = self.view.buffer_id().index();
        let before = self.view.cursor();
        let transaction = self.buffers[index].replace_matches(&matches, &replacement);
        let after = matches[0].0;
        self.view.set_cursor(after);
        record(&mut self.undo, &mut self.redo, transaction, before, after);
        self.focus = Focus::Editor;
        self.prompt.matches.clear();
        self.prompt.current = None;
        self.message = Some(format!("replaced {count} matches"));
    }

    pub(super) fn goto_line(&mut self) {
        let Ok(number) = self.prompt.goto.trim().parse::<usize>() else {
            self.message = Some("invalid line number".to_owned());
            return;
        };
        if number == 0 {
            self.message = Some("invalid line number".to_owned());
            return;
        }
        let index = self.view.buffer_id().index();
        let line = (number - 1).min(self.buffers[index].len_lines().saturating_sub(1));
        let at = self.buffers[index].line_start(line);
        self.view.set_cursor(at);
        self.focus = Focus::Editor;
        self.message = None;
    }
}
