//! a line of text the player types into: what the keys do to it is plain logic (so it can be tried without a window), and `Typing` is what turns the
//! keyboard into those edits, with the key repeat that holding a key down should have

use crate::prelude::*;

//which characters a field takes: a name is letters and numbers and a few marks, a port is only digits
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Allowed {
    Name,
    Password,
    Port,
}

impl Allowed {
    pub(crate) fn takes(self, letter: char) -> bool {
        match self {
            //(the same as `clean_name` leaves, so what is typed is what is kept)
            Allowed::Name => letter.is_alphanumeric() || matches!(letter, ' ' | '-' | '_' | '\'' | '.'),
            Allowed::Password => !letter.is_control(),
            Allowed::Port => letter.is_ascii_digit(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub(crate) enum Edit {
    Type(char),
    Paste(String),
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
}

#[derive(Clone, Debug)]
pub(crate) struct TextField {
    text: String,
    cursor: usize, //in letters, not bytes
    limit: usize,  //the most letters it holds
    allowed: Allowed,
}

impl TextField {
    pub(crate) fn new(text: &str, limit: usize, allowed: Allowed) -> TextField {
        let text: String = text.chars().filter(|&letter| allowed.takes(letter)).take(limit).collect();
        TextField { cursor: text.chars().count(), text, limit, allowed }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn len(&self) -> usize {
        self.text.chars().count()
    }

    //what is shown of it: the text itself, or a mark for each letter when it is a secret
    pub(crate) fn shown(&self, secret: bool) -> String {
        if secret { "*".repeat(self.len()) } else { self.text.clone() }
    }

    fn byte_of(&self, letters: usize) -> usize {
        self.text.char_indices().nth(letters).map_or(self.text.len(), |(at, _)| at)
    }

    fn put(&mut self, letter: char) {
        if self.len() < self.limit && self.allowed.takes(letter) {
            self.text.insert(self.byte_of(self.cursor), letter);
            self.cursor += 1;
        }
    }

    pub(crate) fn apply(&mut self, edit: Edit) {
        match edit {
            Edit::Type(letter) => self.put(letter),
            //(only what is on the first line: a password that was copied with the end of its line on it is the password without it)
            Edit::Paste(text) => text.chars().take_while(|letter| !matches!(letter, '\n' | '\r')).for_each(|letter| self.put(letter)),
            Edit::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.text.remove(self.byte_of(self.cursor));
                }
            }
            Edit::Delete => {
                if self.cursor < self.len() {
                    self.text.remove(self.byte_of(self.cursor));
                }
            }
            Edit::Left => self.cursor = self.cursor.saturating_sub(1),
            Edit::Right => self.cursor = (self.cursor + 1).min(self.len()),
            Edit::Home => self.cursor = 0,
            Edit::End => self.cursor = self.len(),
        }
    }
}

const REPEAT_AFTER: f32 = 0.4; //seconds a key is held before it starts repeating
const REPEAT_EVERY: f32 = 0.035;
const EDIT_KEYS: [(KeyCode, Edit); 4] = [(KeyCode::Backspace, Edit::Backspace), (KeyCode::Delete, Edit::Delete), (KeyCode::Left, Edit::Left), (KeyCode::Right, Edit::Right)];

//the keyboard as edits: what is typed, pasted (Ctrl+V), and the keys that move about in the text, which repeat while they are held
#[derive(Default)]
pub(crate) struct Typing {
    held: Option<(usize, f32)>, //which of EDIT_KEYS, and how long until it repeats
}

impl Typing {
    pub(crate) fn new() -> Typing {
        //(whatever was typed before the screen came up, and the key that brought it, is not for it)
        clear_input_queue();
        Typing::default()
    }

    pub(crate) fn edits(&mut self, dt: f32) -> Vec<Edit> {
        let mut edits = Vec::new();
        while let Some(letter) = get_char_pressed() {
            if !letter.is_control() {
                edits.push(Edit::Type(letter));
            }
        }
        let control = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
        if control && is_key_pressed(KeyCode::V) {
            if let Some(text) = miniquad::window::clipboard_get() {
                edits.push(Edit::Paste(text));
            }
        }
        for (key, edit) in [(KeyCode::Home, Edit::Home), (KeyCode::End, Edit::End)] {
            if is_key_pressed(key) {
                edits.push(edit);
            }
        }
        for (which, (key, edit)) in EDIT_KEYS.iter().enumerate() {
            if is_key_pressed(*key) {
                edits.push(edit.clone());
                self.held = Some((which, REPEAT_AFTER));
            }
        }
        if let Some((which, wait)) = self.held {
            let (key, edit) = &EDIT_KEYS[which];
            if !is_key_down(*key) {
                self.held = None;
            } else if !is_key_pressed(*key) {
                let mut wait = wait - dt;
                while wait <= 0.0 {
                    edits.push(edit.clone());
                    wait += REPEAT_EVERY;
                }
                self.held = Some((which, wait));
            }
        }
        edits
    }
}
