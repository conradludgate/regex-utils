use std::collections::{hash_map, HashMap, VecDeque};

use petgraph::{
    graph::{Graph, NodeIndex},
    Directed,
};
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
type G = Graph<StateID, u8, Directed>;
pub struct RegexBfs<'a, A> {
    dfa: &'a A,
    classes: &'a ByteClasses,
    states: HashMap<StateID, NodeIndex>,
    graph: G,
    pub stack: VecDeque<NodeIndex>,
}

impl<'a, T> From<&'a DFA<T>> for RegexBfs<'a, DFA<T>>
where
    DFA<T>: Automaton,
    T: AsRef<[u32]>,
{
    fn from(dfa: &'a DFA<T>) -> Self {
        let state = dfa.start_state_forward(&Input::new("")).unwrap();
        let mut graph = G::new();
        let start = graph.add_node(state);
        let classes = dfa.byte_classes();
        let states = HashMap::from([(state, start)]);
        Self {
            dfa,
            classes,
            graph,
            stack: VecDeque::from([start]),
            states,
        }
    }
}

impl<A> Iterator for RegexBfs<'_, A>
where
    A: Automaton,
{
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let current = self.stack.pop_front()?;
            let state = self.graph[current];

            for i in 0..self.classes.alphabet_len() {
                for b in self.classes.elements(Unit::u8(i as u8)) {
                    if let Some(b) = b.as_u8() {
                        let next_state = self.dfa.next_state(state, b);
                        if self.dfa.is_dead_state(next_state) {
                            break;
                        }

                        // let next = match self.states.entry(next_state) {
                        //     hash_map::Entry::Occupied(o) => *o.get(),
                        //     hash_map::Entry::Vacant(v) => *v.insert(self.graph.add_node(next_state)),
                        // };
                        let next = self.graph.add_node(next_state);
                        self.graph.add_edge(current, next, b);
                        self.stack.push_back(next);
                    }
                }
            }

            // test that this state is final
            let eoi_state = self.dfa.next_eoi_state(state);
            if self.dfa.is_match_state(eoi_state) {
                return Some(string(&self.graph, current));
            }

            // let (state, depth, mut elements) = self.stack.pop_front()?;
            // self.string.truncate(depth);

            // // get the next element from this set
            // if let Some(b) = elements.next() {
            //     if let Some(b) = b.as_u8() {
            //         // transition the state
            //         let new_state = self.dfa.next_state(state, b);
            //         // if the state is still valid
            //         if !self.dfa.is_dead_state(new_state) {
            //             // re-insert the previous stack position
            //             self.stack.push_front((state, depth, elements));
            //             // insert this byte into the search string
            //             self.string.push(b);

            //             // for all possible set of element classes,
            //             // insert to the stack search space.
            //             for i in (0..self.classes.alphabet_len()) {
            //                 self.stack.push_back((
            //                     new_state,
            //                     depth + 1,
            //                     self.classes.elements(Unit::u8(i as u8)),
            //                 ));
            //             }

            //             // test that this state is final
            //             let eoi_state = self.dfa.next_eoi_state(new_state);
            //             if self.dfa.is_match_state(eoi_state) {
            //                 return Some(self.string.clone());
            //             }
            //         }
            //     } else {
            //         let new_state = self.dfa.next_eoi_state(state);
            //         if self.dfa.is_match_state(new_state) {
            //             return Some(self.string.clone());
            //         }
            //     }
            // }
        }
    }
}

fn string(graph: &G, mut current: NodeIndex) -> Vec<u8> {
    let mut s = vec![];
    while let Some(edge) = graph.first_edge(current, petgraph::Direction::Incoming) {
        let b = graph[edge];
        let (prev, _) = graph.edge_endpoints(edge).unwrap();
        current = prev;
        s.insert(0, b);
    }
    s
}

#[cfg(test)]
mod tests {
    use regex_automata::dfa::dense::DFA;

    use super::*;

    #[test]
    fn bfs_fixed() {
        let dfa = DFA::new(r"^[0-1]{4}-[0-1]{2}-[0-1]{2}$").unwrap();
        let x = RegexBfs::from(&dfa);

        for b in x.take(20) {
            dbg!(String::from_utf8_lossy(&b));
        }
    }
    #[test]
    fn bfs_repeated() {
        let dfa = DFA::new(r"^a+(0|1)$").unwrap();
        let x = RegexBfs::from(&dfa);

        for b in x.take(20) {
            dbg!(String::from_utf8_lossy(&b));
        }
    }
}
