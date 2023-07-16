//! Utilitise for working with regexes
//!
//! # Regex Iteration
//!
//! Given a regex, generate all (potentially infinite) possible inputs that match the pattern.
//!
//! ```
//! use regex_utils::{DenseDfaIter, NfaIter, Utf8Iter};
//!
//! // parse a regex as an NFA
//! let iter = NfaIter::new(r"a+(0|1)").unwrap();
//! // and expect it to be utf8
//! let iter = Utf8Iter::try_from(iter).unwrap();
//!
//! // Because the regex has an infinite pattern (`+`)
//! // the iterator will be infinite. Let's take a subset
//! let x: Vec<String> = iter.take(10).collect();
//! assert_eq!(x, [
//!     "a0".to_owned(),
//!     "a1".to_owned(),
//!     "aa0".to_owned(),
//!     "aa1".to_owned(),
//!     "aaa0".to_owned(),
//!     "aaa1".to_owned(),
//!     "aaaa0".to_owned(),
//!     "aaaa1".to_owned(),
//!     "aaaaa0".to_owned(),
//!     "aaaaa1".to_owned(),
//! ]);
//!
//! // parse a regex as a dense DFA
//! let iter = DenseDfaIter::new(r"foo|(bar){1,2}|quux").unwrap();
//!
//! // The regex has a finite number of states, so we can collect all of them
//! let x: Vec<Vec<u8>> = iter.collect();
//! assert_eq!(x, [
//!     b"bar".to_vec(),
//!     b"foo".to_vec(),
//!     b"quux".to_vec(),
//!     b"barbar".to_vec(),
//! ]);
//! ```
//!
//! ## NFA (Nondeterministic Finite Automaton)
//!
//! Using [`NfaIter`] you can traverse the regex using an [`NFA`](regex_automata::nfa). NFAs are low memory
//! representations of regular expressions, at the cost of being slower.
//!
//! These do not guarantee that output strings are unique (given that the graph is non-deterministic)
//! but the search space memory will be much smaller.
//!
//! ## DFA (Deterministic Finite Automaton)
//!
//! Using [`DfaIter`] you can traverse the regex using a [`DFA`](regex_automata::dfa). DFAs are high memory
//! representations of regular expressions, at the cost of using much more memory.
//!
//! These guarantee that output strings are unique, but the search space will likely use more memory.
//!
//! ## Utf8
//!
//! Using [`Utf8Iter`] you can get the outputs of the NFA or DFA iterators as [`String`]
//! representations of regular expressions, at the cost of using much more memory.
//!
//! These guarantee that output strings are unique, but the search space will likely use more memory.

use core::fmt;
use std::error;

pub use dfa::{DenseDfaIter, DfaIter, SparseDfaIter};
pub use nfa::NfaIter;
use regex_automata::dfa::Automaton;

mod dfa;
mod nfa;

/// [`NfaIter`] or [`DfaIter`] iterator with UTF8 [`String`]s as output
pub struct Utf8Iter<I>(I);

#[derive(Debug)]
/// Regex provided to [`Utf8Iter`] was not valid for generating UTF8 strings
pub struct RegexNotUtf8;

impl fmt::Display for RegexNotUtf8 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("regex is not utf8")
    }
}

impl error::Error for RegexNotUtf8 {}

impl TryFrom<NfaIter> for Utf8Iter<NfaIter> {
    type Error = RegexNotUtf8;
    fn try_from(value: NfaIter) -> Result<Self, Self::Error> {
        if value.regex.is_utf8() {
            Ok(Self(value))
        } else {
            Err(RegexNotUtf8)
        }
    }
}

impl<A: Automaton> TryFrom<DfaIter<A>> for Utf8Iter<DfaIter<A>> {
    type Error = RegexNotUtf8;
    fn try_from(value: DfaIter<A>) -> Result<Self, Self::Error> {
        if value.regex.is_utf8() {
            Ok(Self(value))
        } else {
            Err(RegexNotUtf8)
        }
    }
}

impl<A: Automaton> Utf8Iter<DfaIter<A>> {
    /// Get the next matching string ref from this regex iterator
    pub fn borrow_next(&mut self) -> Option<&str> {
        let next = self.0.borrow_next()?;
        Some(std::str::from_utf8(next).expect("Regex should only match utf8"))
    }
}
impl Utf8Iter<NfaIter> {
    /// Get the next matching string ref from this regex iterator
    pub fn borrow_next(&mut self) -> Option<&str> {
        let next = self.0.borrow_next()?;
        Some(std::str::from_utf8(next).expect("Regex should only match utf8"))
    }
}

impl<I: Iterator<Item = Vec<u8>>> Iterator for Utf8Iter<I> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        let next = self.0.next()?;
        Some(String::from_utf8(next).expect("Regex should only match utf8"))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use regex_automata::nfa::thompson::NFA;

    use super::*;

    #[test]
    fn finite() {
        let nfa = NFA::new(r"[0-1]{4}-[0-1]{2}-[0-1]{2}").unwrap();
        let iter = Utf8Iter::try_from(NfaIter::from(nfa)).unwrap();

        // finite regex has finite iteration depth
        // and no repeats
        let x: HashSet<String> = iter.collect();
        assert_eq!(x.len(), 256);
        for y in x {
            assert_eq!(y.len(), 10);
        }
    }

    #[test]
    fn repeated() {
        let nfa = NFA::new(r"a+(0|1)").unwrap();
        let iter = Utf8Iter::try_from(NfaIter::from(nfa)).unwrap();

        // infinite regex iterates over all cases
        let x: Vec<String> = iter.take(20).collect();
        let y = [
            "a0".to_owned(),
            "a1".to_owned(),
            "aa0".to_owned(),
            "aa1".to_owned(),
            "aaa0".to_owned(),
            "aaa1".to_owned(),
            "aaaa0".to_owned(),
            "aaaa1".to_owned(),
            "aaaaa0".to_owned(),
            "aaaaa1".to_owned(),
            "aaaaaa0".to_owned(),
            "aaaaaa1".to_owned(),
            "aaaaaaa0".to_owned(),
            "aaaaaaa1".to_owned(),
            "aaaaaaaa0".to_owned(),
            "aaaaaaaa1".to_owned(),
            "aaaaaaaaa0".to_owned(),
            "aaaaaaaaa1".to_owned(),
            "aaaaaaaaaa0".to_owned(),
            "aaaaaaaaaa1".to_owned(),
        ];
        assert_eq!(x, y);
    }

    #[test]
    fn complex() {
        let nfa = NFA::new(r"(a+|b+)*").unwrap();
        let iter = Utf8Iter::try_from(NfaIter::from(nfa)).unwrap();

        // infinite regex iterates over all cases
        let x: Vec<String> = iter.take(13).collect();
        let y = [
            "".to_owned(),
            "a".to_owned(),
            "b".to_owned(),
            "aa".to_owned(),
            "bb".to_owned(),
            "aaa".to_owned(),
            "bbb".to_owned(),
            "aaaa".to_owned(),
            // technically a different path
            "aa".to_owned(),
            "ab".to_owned(),
            "bbbb".to_owned(),
            "ba".to_owned(),
            // technically a different path
            "bb".to_owned(),
        ];
        assert_eq!(x, y);
    }

    #[test]
    fn many() {
        let search = NfaIter::new_many(&["[0-1]+", "^[a-b]+"]).unwrap();
        let iter = Utf8Iter::try_from(search).unwrap();
        let x: Vec<String> = iter.take(12).collect();
        let y = [
            "0".to_owned(),
            "1".to_owned(),
            "a".to_owned(),
            "b".to_owned(),
            "00".to_owned(),
            "01".to_owned(),
            "10".to_owned(),
            "11".to_owned(),
            "aa".to_owned(),
            "ab".to_owned(),
            "ba".to_owned(),
            "bb".to_owned(),
        ];
        assert_eq!(x, y);
    }
}
