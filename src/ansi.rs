// yoinked from https://github.com/console-rs/console /src/ansi.rs

// i think i have to legally include this
/*
Copyright (c) 2017 Armin Ronacher <armin.ronacher@active-4.com>

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.
*/

use std::{
    borrow::Cow,
    iter::{FusedIterator, Peekable},
    str::CharIndices,
};

#[derive(Debug, Clone, Copy)]
enum State {
    Start,
    S1,
    S2,
    S3,
    S4,
    S5,
    S6,
    S7,
    S8,
    S9,
    S10,
    S11,
    Trap,
}

impl Default for State {
    fn default() -> Self {
        Self::Start
    }
}

impl State {
    fn is_final(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::S3 | Self::S5 | Self::S6 | Self::S7 | Self::S8 | Self::S9 | Self::S11 => true,
            _ => false,
        }
    }

    fn is_trapped(&self) -> bool {
        #[allow(clippy::match_like_matches_macro)]
        match self {
            Self::Trap => true,
            _ => false,
        }
    }

    fn transition(&mut self, c: char) {
        *self = match c {
            '\u{1b}' | '\u{9b}' => match self {
                Self::Start => Self::S1,
                _ => Self::Trap,
            },
            '(' | ')' => match self {
                Self::S1 => Self::S2,
                Self::S2 | Self::S4 => Self::S4,
                _ => Self::Trap,
            },
            ';' => match self {
                Self::S1 | Self::S2 | Self::S4 => Self::S4,
                Self::S5 | Self::S6 | Self::S7 | Self::S8 | Self::S10 => Self::S10,
                _ => Self::Trap,
            },

            '[' | '#' | '?' => match self {
                Self::S1 | Self::S2 | Self::S4 => Self::S4,
                _ => Self::Trap,
            },
            '0'..='2' => match self {
                Self::S1 | Self::S4 => Self::S5,
                Self::S2 => Self::S3,
                Self::S5 => Self::S6,
                Self::S6 => Self::S7,
                Self::S7 => Self::S8,
                Self::S8 => Self::S9,
                Self::S10 => Self::S5,
                _ => Self::Trap,
            },
            '3'..='9' => match self {
                Self::S1 | Self::S4 => Self::S5,
                Self::S2 => Self::S5,
                Self::S5 => Self::S6,
                Self::S6 => Self::S7,
                Self::S7 => Self::S8,
                Self::S8 => Self::S9,
                Self::S10 => Self::S5,
                _ => Self::Trap,
            },
            'A'..='P' | 'R' | 'Z' | 'c' | 'f'..='n' | 'q' | 'r' | 'y' | '=' | '>' | '<' => {
                match self {
                    Self::S1
                    | Self::S2
                    | Self::S4
                    | Self::S5
                    | Self::S6
                    | Self::S7
                    | Self::S8
                    | Self::S10 => Self::S11,
                    _ => Self::Trap,
                }
            }
            _ => Self::Trap,
        };
    }
}

#[derive(Debug)]
struct Matches<'a> {
    s: &'a str,
    it: Peekable<CharIndices<'a>>,
}

impl<'a> Matches<'a> {
    fn new(s: &'a str) -> Self {
        let it = s.char_indices().peekable();
        Self { s, it }
    }
}

#[derive(Debug)]
struct Match<'a> {
    text: &'a str,
    start: usize,
    end: usize,
}

impl<'a> Match<'a> {
    #[inline]
    pub fn as_str(&self) -> &'a str {
        &self.text[self.start..self.end]
    }
}

impl<'a> Iterator for Matches<'a> {
    type Item = Match<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        find_ansi_code_exclusive(&mut self.it).map(|(start, end)| Match {
            text: self.s,
            start,
            end,
        })
    }
}

impl<'a> FusedIterator for Matches<'a> {}

fn find_ansi_code_exclusive(it: &mut Peekable<CharIndices>) -> Option<(usize, usize)> {
    'outer: loop {
        if let (start, '\u{1b}') | (start, '\u{9b}') = it.peek()? {
            let start = *start;
            let mut state = State::default();
            let mut maybe_end = None;

            loop {
                let item = it.peek();

                if let Some((idx, c)) = item {
                    state.transition(*c);

                    if state.is_final() {
                        maybe_end = Some(*idx);
                    }
                }

                // The match is greedy so run till we hit the trap state no matter what. A valid
                // match is just one that was final at some point
                if state.is_trapped() || item.is_none() {
                    match maybe_end {
                        Some(end) => {
                            // All possible final characters are a single byte so it's safe to make
                            // the end exclusive by just adding one
                            return Some((start, end + 1));
                        }
                        // The character we are peeking right now might be the start of a match so
                        // we want to continue the loop without popping off that char
                        None => continue 'outer,
                    }
                }

                it.next();
            }
        }

        it.next();
    }
}

/// Helper function to strip ansi codes.
pub fn strip_ansi_codes(s: &str) -> Cow<str> {
    let mut char_it = s.char_indices().peekable();
    match find_ansi_code_exclusive(&mut char_it) {
        Some(_) => {
            let stripped: String = AnsiCodeIterator::new(s)
                .filter_map(|(text, is_ansi)| if is_ansi { None } else { Some(text) })
                .collect();
            Cow::Owned(stripped)
        }
        None => Cow::Borrowed(s),
    }
}

/// An iterator over ansi codes in a string.
///
/// This type can be used to scan over ansi codes in a string.
/// It yields tuples in the form `(s, is_ansi)` where `s` is a slice of
/// the original string and `is_ansi` indicates if the slice contains
/// ansi codes or string values.
pub struct AnsiCodeIterator<'a> {
    s: &'a str,
    pending_item: Option<(&'a str, bool)>,
    last_idx: usize,
    cur_idx: usize,
    iter: Matches<'a>,
}

impl<'a> AnsiCodeIterator<'a> {
    /// Creates a new ansi code iterator.
    pub fn new(s: &'a str) -> AnsiCodeIterator<'a> {
        AnsiCodeIterator {
            s,
            pending_item: None,
            last_idx: 0,
            cur_idx: 0,
            iter: Matches::new(s),
        }
    }

    /* unused, scared to delete forever
    /// Returns the string slice up to the current match.
    pub fn current_slice(&self) -> &str {
        &self.s[..self.cur_idx]
    }

    /// Returns the string slice from the current match to the end.
    pub fn rest_slice(&self) -> &str {
        &self.s[self.cur_idx..]
    }
    */
}

impl<'a> Iterator for AnsiCodeIterator<'a> {
    type Item = (&'a str, bool);

    fn next(&mut self) -> Option<(&'a str, bool)> {
        if let Some(pending_item) = self.pending_item.take() {
            self.cur_idx += pending_item.0.len();
            Some(pending_item)
        } else if let Some(m) = self.iter.next() {
            let s = &self.s[self.last_idx..m.start];
            self.last_idx = m.end;
            if s.is_empty() {
                self.cur_idx = m.end;
                Some((m.as_str(), true))
            } else {
                self.cur_idx = m.start;
                self.pending_item = Some((m.as_str(), true));
                Some((s, false))
            }
        } else if self.last_idx < self.s.len() {
            let rv = &self.s[self.last_idx..];
            self.cur_idx = self.s.len();
            self.last_idx = self.s.len();
            Some((rv, false))
        } else {
            None
        }
    }
}

impl<'a> FusedIterator for AnsiCodeIterator<'a> {}
