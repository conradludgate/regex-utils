use regex_automata::{
    dfa::{dense::DFA, Automaton},
    util::{
        alphabet::{ByteClassElements, ByteClasses, Unit},
        primitives::StateID,
    },
    Input,
};

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

pub struct Dfs<'a, A> {
    dfa: &'a A,
    classes: &'a ByteClasses,
    stack: Vec<(StateID, usize, ByteClassElements<'a>)>,
    string: Vec<u8>,
}

impl<'a, T> From<&'a DFA<T>> for Dfs<'a, DFA<T>>
where
    DFA<T>: Automaton,
    T: AsRef<[u32]>,
{
    fn from(dfa: &'a DFA<T>) -> Self {
        let state = dfa.start_state_forward(&Input::new("")).unwrap();
        let classes = dfa.byte_classes();
        let mut stack = vec![];
        for i in 0..classes.alphabet_len() {
            stack.push((state, 0, classes.elements(Unit::u8(i as u8))));
        }
        Self {
            dfa,
            classes,
            stack,
            string: vec![],
        }
    }
}

impl<A> Iterator for Dfs<'_, A>
where
    A: Automaton,
{
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (state, depth, mut elements) = self.stack.pop()?;
            self.string.truncate(depth);

            // get the next element from this set
            if let Some(b) = elements.next() {
                if let Some(b) = b.as_u8() {
                    // transition the state
                    let new_state = self.dfa.next_state(state, b);
                    // if the state is still valid
                    if !self.dfa.is_dead_state(new_state) {
                        // re-insert the previous stack position
                        self.stack.push((state, depth, elements));
                        // insert this byte into the search string
                        self.string.push(b);

                        // for all possible set of element classes,
                        // insert to the stack search space.
                        for i in 0..self.classes.alphabet_len() {
                            self.stack.push((
                                new_state,
                                depth + 1,
                                self.classes.elements(Unit::u8(i as u8)),
                            ));
                        }

                        // test that this state is final
                        let eoi_state = self.dfa.next_eoi_state(new_state);
                        if self.dfa.is_match_state(eoi_state) {
                            return Some(self.string.clone());
                        }
                    }
                } else {
                    let new_state = self.dfa.next_eoi_state(state);
                    if self.dfa.is_match_state(new_state) {
                        return Some(self.string.clone());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use regex_automata::dfa::dense::DFA;

    use super::*;

    #[test]
    fn it_works() {
        let dfa = DFA::new(r"^[0-9]{4}-[0-9]{2}-[0-9]{2}$").unwrap();
        let mut x = Dfs::from(&dfa);

        assert_eq!(x.nth(12345).unwrap(), b"0001-23-45");
        assert_eq!(x.nth(12345).unwrap(), b"0002-46-91");
        assert_eq!(x.next().unwrap(), b"0002-46-92");
    }
}
