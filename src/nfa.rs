#![allow(clippy::result_large_err)]

use regex_automata::{
    nfa::thompson::{BuildError, State, NFA},
    util::{look::Look, primitives::StateID},
};
use tinyvec::TinyVec;

/// For Look/Union/BinaryUnion/Capture/Fail/Match: meaningless (should be empty)
/// For ByteRange: indicates the current byte
/// For Sparse: indicates the current byte for each ByteRange
/// For Dense: indicates the current byte (0..=255)
type SearchRange = TinyVec<[u16; 12]>;

/// `NfaIter` will produce every possible string value that will match with the given nfa regex.
///
/// # Note
///
/// Regexes can be infinite (eg `a*`). Either use this iterator lazily, or limit the number
/// of iterations.
pub struct NfaIter {
    // the graph to search
    pub(crate) regex: NFA,
    // the start node of the graph
    start: StateID,
    start_range: SearchRange,
    // the max depth we currently want to search
    depth: usize,
    // the max depth observed in the graph
    max_depth: usize,
    // (state, search_range, byte depth, search depth)
    // the search_range is used differently depending on what state we are exploring
    stack: Vec<(StateID, SearchRange, usize, usize)>,
    // the current path
    str: Vec<u8>,
}

impl From<NFA> for NfaIter {
    fn from(nfa: NFA) -> Self {
        // anchored because if we didn't anchor our search we would have an infinite amount of prefixes that were valid
        // and that isn't very interesting
        let start = nfa.start_anchored();
        let start_range = range_for(nfa.state(start));

        Self {
            regex: nfa,
            stack: vec![(start, start_range.clone(), 0, 0)],
            start,
            start_range,
            depth: 0,
            max_depth: 0,
            str: vec![],
        }
    }
}

fn range_for(s: &State) -> SearchRange {
    match s {
        State::ByteRange { trans } => tinyvec::tiny_vec![trans.start as u16],
        State::Sparse(s) => s
            .transitions
            .iter()
            .map(|trans| trans.start as u16)
            .collect(),
        State::Dense(_) => tinyvec::tiny_vec![0],
        State::Look { .. } => tinyvec::tiny_vec![],
        State::Union { .. } => tinyvec::tiny_vec![],
        State::BinaryUnion { .. } => tinyvec::tiny_vec![],
        State::Capture { .. } => tinyvec::tiny_vec![],
        State::Fail => tinyvec::tiny_vec![],
        State::Match { .. } => tinyvec::tiny_vec![],
    }
}

impl NfaIter {
    /// Parse the given regular expression using a default configuration and
    /// return the corresponding `NfaIter`.
    ///
    /// If you want a non-default configuration, then use the
    /// [`thompson::Compiler`](regex_automata::nfa::thompson::Compiler) to set your own configuration.
    ///
    /// See [`NFA`] for details
    pub fn new(pattern: &str) -> Result<Self, BuildError> {
        NFA::compiler().build(pattern).map(Self::from)
    }

    /// Parse the given regular expressions using a default configuration and
    /// return the corresponding multi-`NfaIter`.
    ///
    /// If you want a non-default configuration, then use the
    /// [`thompson::Compiler`](regex_automata::nfa::thompson::Compiler) to set your own configuration.
    ///
    /// See [`NFA`] for details
    pub fn new_many<P: AsRef<str>>(patterns: &[P]) -> Result<Self, BuildError> {
        NFA::compiler().build_many(patterns).map(Self::from)
    }

    fn range_for(&self, s: StateID) -> SearchRange {
        range_for(self.regex.state(s))
    }

    /// Get the next matching string ref from this regex iterator
    pub fn borrow_next(&mut self) -> Option<&[u8]> {
        loop {
            let Some((current, range, byte_depth, depth)) = self.stack.pop() else {
                // we didn't get any deeper. no more search space
                if self.max_depth < self.depth {
                    break None;
                }

                self.depth += 1;
                self.stack.clear();
                self.stack.push((self.start, self.start_range.clone(), 0, 0));
                continue;
            };

            // update recorded max depth
            self.max_depth = usize::max(self.max_depth, depth);
            self.str.truncate(byte_depth);

            let state = self.regex.state(current);

            // check we can explore deeper
            if depth < self.depth {
                match state {
                    State::ByteRange { trans } => {
                        // make sure we revisit this state
                        if (range[0] as u8) < trans.end {
                            self.stack.push((
                                current,
                                tinyvec::tiny_vec![range[0] + 1],
                                byte_depth,
                                depth,
                            ));
                        }
                        self.str.push(range[0] as u8);
                        self.stack.push((
                            trans.next,
                            self.range_for(trans.next),
                            byte_depth + 1,
                            depth + 1,
                        ));
                    }
                    State::Sparse(s) => {
                        for (i, &r) in range.iter().enumerate() {
                            let t = s.transitions[i];
                            if r <= t.end as u16 {
                                // make sure we revisit this state
                                let mut new_range = range.clone();
                                new_range[i] += 1;
                                self.stack.push((current, new_range, byte_depth, depth));

                                self.str.push(r as u8);
                                // add the new state
                                self.stack.push((
                                    t.next,
                                    self.range_for(t.next),
                                    byte_depth + 1,
                                    depth + 1,
                                ));
                                break;
                            }
                        }
                    }
                    State::Dense(d) => {
                        // make sure we revisit this state
                        if range[0] < 255 {
                            self.stack.push((
                                current,
                                tinyvec::tiny_vec![range[0] + 1],
                                byte_depth,
                                depth,
                            ));
                        }
                        self.str.push(range[0] as u8);
                        self.stack.push((
                            d.transitions[range[0] as usize],
                            self.range_for(d.transitions[range[0] as usize]),
                            byte_depth + 1,
                            depth + 1,
                        ));
                    }
                    State::Look { look, next } => {
                        let should = match look {
                            Look::Start if byte_depth == 0 => true,
                            Look::StartLF
                                if byte_depth == 0 || self.str[byte_depth - 1] == b'\n' =>
                            {
                                true
                            }
                            Look::StartCRLF
                                if byte_depth == 0
                                    || self.str[byte_depth - 1] == b'\n'
                                    || self.str[byte_depth - 1] == b'\r' =>
                            {
                                true
                            }
                            Look::End => true,
                            Look::EndLF => true,
                            Look::EndCRLF => true,
                            Look::WordAscii => todo!(),
                            Look::WordAsciiNegate => todo!(),
                            Look::WordUnicode => todo!(),
                            Look::WordUnicodeNegate => todo!(),
                            _ => false,
                        };
                        if should {
                            self.stack
                                .push((*next, self.range_for(*next), byte_depth, depth + 1));
                        }
                    }
                    State::Union { alternates } => {
                        // same byte_depth because we matched no bytes
                        for &alt in alternates.iter().rev() {
                            self.stack
                                .push((alt, self.range_for(alt), byte_depth, depth + 1));
                        }
                    }
                    State::BinaryUnion { alt1, alt2 } => {
                        // same byte_depth because we matched no bytes
                        for &alt in [alt1, alt2].into_iter().rev() {
                            self.stack
                                .push((alt, self.range_for(alt), byte_depth, depth + 1));
                        }
                    }
                    State::Capture { next, .. } => {
                        // same byte_depth because we matched no bytes
                        self.stack
                            .push((*next, self.range_for(*next), byte_depth, depth + 1));
                    }
                    State::Fail => {}
                    State::Match { .. } => {}
                }
            } else {
                // test that this state is final
                if matches!(state, State::Match { .. }) {
                    break Some(&self.str);
                }
            }
        }
    }
}

impl Iterator for NfaIter {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        self.borrow_next().map(ToOwned::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn set() {
        let iter = NfaIter::new(r"b|(a)?|cc").unwrap();

        let x: Vec<Vec<u8>> = iter.collect();
        assert_eq!(
            x,
            [b"b".to_vec(), b"".to_vec(), b"cc".to_vec(), b"a".to_vec(),]
        );
    }

    #[test]
    fn finite() {
        let nfa = NFA::new(r"[0-1]{4}-[0-1]{2}-[0-1]{2}").unwrap();

        // finite regex has finite iteration depth
        // and no repeats
        let x: HashSet<Vec<u8>> = NfaIter::from(nfa).collect();
        assert_eq!(x.len(), 256);
        for y in x {
            assert_eq!(y.len(), 10);
        }
    }

    #[test]
    fn repeated() {
        let nfa = NFA::new(r"a+(0|1)").unwrap();

        // infinite regex iterates over all cases
        let x: Vec<Vec<u8>> = NfaIter::from(nfa).take(20).collect();
        let y = [
            b"a0".to_vec(),
            b"a1".to_vec(),
            b"aa0".to_vec(),
            b"aa1".to_vec(),
            b"aaa0".to_vec(),
            b"aaa1".to_vec(),
            b"aaaa0".to_vec(),
            b"aaaa1".to_vec(),
            b"aaaaa0".to_vec(),
            b"aaaaa1".to_vec(),
            b"aaaaaa0".to_vec(),
            b"aaaaaa1".to_vec(),
            b"aaaaaaa0".to_vec(),
            b"aaaaaaa1".to_vec(),
            b"aaaaaaaa0".to_vec(),
            b"aaaaaaaa1".to_vec(),
            b"aaaaaaaaa0".to_vec(),
            b"aaaaaaaaa1".to_vec(),
            b"aaaaaaaaaa0".to_vec(),
            b"aaaaaaaaaa1".to_vec(),
        ];
        assert_eq!(x, y);
    }

    #[test]
    fn complex() {
        let nfa = NFA::new(r"(a+|b+)*").unwrap();

        // infinite regex iterates over all cases
        let x: Vec<Vec<u8>> = NfaIter::from(nfa).take(13).collect();
        let y = [
            b"".to_vec(),
            b"a".to_vec(),
            b"b".to_vec(),
            b"aa".to_vec(),
            b"bb".to_vec(),
            b"aaa".to_vec(),
            b"bbb".to_vec(),
            b"aaaa".to_vec(),
            // technically a different path
            b"aa".to_vec(),
            b"ab".to_vec(),
            b"bbbb".to_vec(),
            b"ba".to_vec(),
            // technically a different path
            b"bb".to_vec(),
        ];
        assert_eq!(x, y);
    }

    #[test]
    fn many() {
        let search = NfaIter::new_many(&["[0-1]+", "^[a-b]+"]).unwrap();
        let x: Vec<Vec<u8>> = search.take(12).collect();
        let y = [
            b"0".to_vec(),
            b"1".to_vec(),
            b"a".to_vec(),
            b"b".to_vec(),
            b"00".to_vec(),
            b"01".to_vec(),
            b"10".to_vec(),
            b"11".to_vec(),
            b"aa".to_vec(),
            b"ab".to_vec(),
            b"ba".to_vec(),
            b"bb".to_vec(),
        ];
        assert_eq!(x, y);
    }
}
