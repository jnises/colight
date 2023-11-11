use std::collections::VecDeque;

pub(crate) struct WindowSearcher {
    window_size: usize,
    haystack: VecDeque<u8>,
    needle: VecDeque<u8>,
    matches: Vec<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SearchState {
    Buffering,
    Flushed { buffer: VecDeque<u8> },
}

impl WindowSearcher {
    pub(crate) fn new(window_size: usize) -> Self {
        Self {
            window_size,
            haystack: VecDeque::new(),
            needle: VecDeque::new(),
            matches: Vec::new(),
        }
    }

    pub(crate) fn search(&mut self, next_byte: u8) -> SearchState {
        self.matches
            .retain_mut(|i| self.haystack.get(*i + self.needle.len()) == Some(&next_byte));
        let r = if self.matches.is_empty() {
            self.haystack.extend(self.needle.iter());
            while self.haystack.len() > self.window_size {
                self.haystack.pop_front();
            }
            let buffer = std::mem::take(&mut self.needle);
            self.matches = (0..self.haystack.len())
                .filter(|&i| self.haystack[i] == next_byte)
                .collect();
            SearchState::Flushed { buffer }
        } else {
            SearchState::Buffering
        };
        self.needle.push_back(next_byte);
        r
    }

    pub(crate) fn flush(mut self) -> VecDeque<u8> {
        std::mem::take(&mut self.needle)
    }
}