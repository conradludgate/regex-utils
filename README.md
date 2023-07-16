# regex-utils

Rust utilitise for working with regexes

## Regex Iteration

Given a regex, generate all (potentially infinite) possible inputs that match the pattern.

```rust
use regex_utils::{DenseDfaIter, NfaIter, Utf8Iter};

// parse a regex as an NFA
let iter = NfaIter::new(r"a+(0|1)").unwrap();
// and expect it to be utf8
let iter = Utf8Iter::try_from(iter).unwrap();

// Because the regex has an infinite pattern (`+`)
// the iterator will be infinite. Let's take a subset
let x: Vec<String> = iter.take(10).collect();
assert_eq!(x, [
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
]);

// parse a regex as a dense DFA
let iter = DenseDfaIter::new(r"foo|(bar){1,2}|quux").unwrap();

// The regex has a finite number of states, so we can collect all of them
let x: Vec<Vec<u8>> = iter.collect();
assert_eq!(x, [
    b"bar".to_vec(),
    b"foo".to_vec(),
    b"quux".to_vec(),
    b"barbar".to_vec(),
]);
```

### NFA (Nondeterministic Finite Automaton)

Using `NfaIter` you can traverse the regex using an [`NFA`](https://docs.rs/regex-automata/0.3.3/regex_automata/nfa/index.html). NFAs are low memory
representations of regular expressions, at the cost of being slower.

These do not guarantee that output strings are unique (given that the graph is non-deterministic)
but the search space memory will be much smaller.

### DFA (Deterministic Finite Automaton)

Using `DfaIter` you can traverse the regex using a [`DFA`](https://docs.rs/regex-automata/0.3.3/regex_automata/dfa/index.html). DFAs are high memory
representations of regular expressions, at the cost of using much more memory.

These guarantee that output strings are unique, but the search space will likely use more memory.

### Utf8

Using `Utf8Iter` you can get the outputs of the NFA or DFA iterators as `String`
representations of regular expressions, at the cost of using much more memory.

These guarantee that output strings are unique, but the search space will likely use more memory.
