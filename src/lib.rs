use std::collections::{HashSet, VecDeque};

use indexmap::IndexSet;
use petgraph::prelude::DiGraphMap;
use regex_automata::{
    dfa::{dense::DFA, Automaton},
    util::{
        alphabet::{ByteClasses, Unit},
        primitives::StateID,
    },
    Input,
};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}
type G = DiGraphMap<StateID, IndexSet<u8>>;

// a hybrid depth-breadth first search impl.
// it's depth first search until it hits a cycle, at which point the branch splits.
pub struct RegexSearch<'a, A> {
    dfa: &'a A,
    classes: &'a ByteClasses,
    graph: G,
    cycle_starts: HashSet<StateID>,
    stacks: VecDeque<Stack>,
}

struct Stack {
    stack: Vec<(StateID, u8, usize)>,
    stash: Vec<StateID>,
    str: Vec<u8>,
}

impl<'a, T> From<&'a DFA<T>> for RegexSearch<'a, DFA<T>>
where
    DFA<T>: Automaton,
    T: AsRef<[u32]>,
{
    fn from(dfa: &'a DFA<T>) -> Self {
        let state = dfa.start_state_forward(&Input::new("")).unwrap();
        let mut graph = G::new();
        let start = graph.add_node(state);
        let classes = dfa.byte_classes();
        Self {
            dfa,
            classes,
            graph,
            cycle_starts: HashSet::new(),
            stacks: VecDeque::from([Stack {
                stack: vec![(start, 0, 0)],
                stash: vec![],
                str: vec![],
            }]),
        }
    }
}

impl<A> Iterator for RegexSearch<'_, A>
where
    A: Automaton,
{
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mut stack = self.stacks.pop_front()?;
            let Some((current, b, depth)) = stack.stack.pop() else { continue };
            stack.stash.truncate(depth);
            stack.str.truncate(depth);
            stack.stash.push(current);
            stack.str.push(b);

            if self.cycle_starts.contains(&current) {
                for (_, next, edge) in self.graph.edges(current) {
                    for b in edge {
                        self.stacks.push_back(Stack {
                            stack: vec![(next, *b, depth + 1)],
                            stash: stack.stash.clone(),
                            str: stack.str.clone(),
                        });
                    }
                }
            } else {
                for i in 0..self.classes.alphabet_len() {
                    for b in self.classes.elements(Unit::u8(i as u8)) {
                        if let Some(b) = b.as_u8() {
                            let next_state = self.dfa.next_state(current, b);
                            if self.dfa.is_dead_state(next_state) {
                                break;
                            }

                            let next = self.graph.add_node(next_state);
                            let mut edge =
                                self.graph.remove_edge(current, next).unwrap_or_default();
                            let first = !edge.insert(b);
                            self.graph.add_edge(current, next, edge);
                            let repeat = stack.stash.contains(&next);

                            if repeat && first {
                                self.cycle_starts.insert(next_state);
                            }
                            stack.stack.push((next, b, depth + 1));
                        }
                    }
                }
            }

            // test that this state is final
            let eoi_state = self.dfa.next_eoi_state(current);
            if self.dfa.is_match_state(eoi_state) {
                let res = stack.str[1..].to_owned();
                self.stacks.push_back(stack);
                return Some(res);
            }

            self.stacks.push_back(stack);
        }
    }
}

#[cfg(test)]
mod tests {
    use regex_automata::dfa::dense::DFA;

    use super::*;

    #[test]
    fn fixed() {
        let dfa = DFA::new(r"^[0-1]{4}-[0-1]{2}-[0-1]{2}$").unwrap();
        let x: HashSet<Vec<u8>> = RegexSearch::from(&dfa).collect();
        assert_eq!(x.len(), 256);
        for y in x {
            assert_eq!(y.len(), 10);
        }
    }

    #[test]
    fn repeated() {
        let dfa = DFA::new(r"^a+(0|1)$").unwrap();
        let x: HashSet<Vec<u8>> = RegexSearch::from(&dfa).take(20).collect();
        let y = HashSet::from_iter([
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
        ]);
        assert_eq!(x, y);
    }
}
