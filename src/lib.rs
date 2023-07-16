use dfa::DfaIter;
use nfa::NfaIter;
use regex_automata::dfa::Automaton;

pub mod dfa;
pub mod nfa;

pub struct Utf8Iter<I>(I);

#[derive(Debug)]
pub struct RegexNotUtf8;

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

impl<I: Iterator<Item = Vec<u8>>> Iterator for Utf8Iter<I> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.0.next()?;
            if let Ok(s) = String::from_utf8(next) {
                break Some(s);
            }
        }
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
