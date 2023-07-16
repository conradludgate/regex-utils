#![allow(clippy::result_large_err)]

use regex_automata::{
    dfa::{
        dense::{BuildError, Config, DFA},
        Automaton,
    },
    util::primitives::StateID,
    Input,
};

/// `RegexIter` will produce every possible string value that will match with the given regex.
///
/// # Note
///
/// Regexes can be infinite (eg `a*`). Use with caution.
///
/// # Implementation Details
///
/// Given a `DFA` (Deterministic Finite Automaton), the iterator walks the graph
/// of states using [`IDDFS`](https://en.wikipedia.org/wiki/Iterative_deepening_depth-first_search)
/// to traverse through every possible state path. At each depth, if we find a match, it is returned.
///
/// The order of matches is not guaranteed, but it currently returns all strings in lexicographical byte ordering.
pub struct DfaIter<A> {
    // the graph to search
    pub(crate) regex: A,
    // the start node of the graph
    start: StateID,
    // the max depth we currently want to search
    depth: usize,
    // the max depth observed in the graph
    max_depth: usize,
    // (state, edge, depth)
    stack: Vec<(StateID, u8, usize)>,
    // the current path
    str: Vec<u8>,
}

impl<A: Automaton> From<A> for DfaIter<A> {
    fn from(dfa: A) -> Self {
        // anchored because if we didn't anchor our search we would have an infinite amount of prefixes that were valid
        // and that isn't very interesting
        let start = dfa
            .start_state_forward(&Input::new("").anchored(regex_automata::Anchored::Yes))
            .unwrap();

        Self {
            regex: dfa,
            start,
            depth: 0,
            max_depth: 0,
            stack: vec![(start, 0, 0)],
            str: vec![],
        }
    }
}

impl DfaIter<DFA<Vec<u32>>> {
    pub fn new(pattern: &str) -> Result<Self, BuildError> {
        DFA::builder()
            .configure(Config::new().accelerate(false))
            .build(pattern)
            .map(Self::from)
    }
    pub fn new_many<P: AsRef<str>>(patterns: &[P]) -> Result<Self, BuildError> {
        DFA::builder()
            .configure(Config::new().accelerate(false))
            .build_many(patterns)
            .map(Self::from)
    }
}

impl<A: Automaton> DfaIter<A> {
    fn borrow_next(&mut self) -> Option<&[u8]> {
        loop {
            let Some((current, b, depth)) = self.stack.pop() else {
                // we didn't get any deeper. no more search space
                if self.max_depth < self.depth {
                    break None;
                }

                self.depth += 1;
                self.stack.clear();
                self.stack.push((self.start, 0, 0));
                continue;
            };

            // update recorded max depth
            self.max_depth = usize::max(self.max_depth, depth);
            self.str.truncate(depth);
            self.str.push(b);

            // check we can explore deeper
            if depth < self.depth {
                for b in (0..=255).rev() {
                    let next_state = self.regex.next_state(current, b);
                    // check if the next state is valid
                    if !self.regex.is_dead_state(next_state) {
                        self.stack.push((next_state, b, depth + 1));
                    }
                }
            } else {
                // test that this state is final
                let eoi_state = self.regex.next_eoi_state(current);
                if self.regex.is_match_state(eoi_state) {
                    break Some(&self.str[1..]);
                }
            }
        }
    }
}

impl<A: Automaton> Iterator for DfaIter<A> {
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        self.borrow_next().map(ToOwned::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use regex_automata::dfa::dense::DFA;

    use super::*;

    #[test]
    fn finite() {
        let dfa = DFA::new(r"[0-1]{4}-[0-1]{2}-[0-1]{2}").unwrap();

        // finite regex has finite iteration depth
        // and no repeats
        let x: HashSet<Vec<u8>> = DfaIter::from(&dfa).collect();
        assert_eq!(x.len(), 256);
        for y in x {
            assert_eq!(y.len(), 10);
        }
    }

    #[test]
    fn repeated() {
        let dfa = DFA::new(r"a+(0|1)").unwrap();

        // infinite regex iterates over all cases
        let x: Vec<Vec<u8>> = DfaIter::from(&dfa).take(20).collect();
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
        let dfa = DFA::new(r"(a+|b+)*").unwrap();

        // infinite regex iterates over all cases
        let x: Vec<Vec<u8>> = DfaIter::from(&dfa).take(8).collect();
        let y = [
            b"".to_vec(),
            b"a".to_vec(),
            b"b".to_vec(),
            b"aa".to_vec(),
            b"ab".to_vec(),
            b"ba".to_vec(),
            b"bb".to_vec(),
            b"aaa".to_vec(),
        ];
        assert_eq!(x, y);
    }

    #[test]
    fn email() {
        let dfa = DFA::new(r"[a-zA-Z0-9.!#$%&’*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\.[a-zA-Z0-9-]+)*")
            .unwrap();
        let mut search = DfaIter::from(&dfa);

        // skip a ~few
        for _ in 0..100_000 {
            search.borrow_next();
        }

        let x = search.borrow_next().unwrap();
        assert_eq!(String::from_utf8_lossy(x), "0@hI");
    }

    #[test]
    fn many() {
        let search = DfaIter::new_many(&["[0-1]+", "^[a-b]+"]).unwrap();
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
