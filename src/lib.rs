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
    inner: Vec<(StateID, Vec<u8>, ByteClassElements<'a>)>,
}

impl<'a, T> From<&'a DFA<T>> for Dfs<'a, DFA<T>>
where
    DFA<T>: Automaton,
    T: AsRef<[u32]>,
{
    fn from(dfa: &'a DFA<T>) -> Self {
        let state = dfa.start_state_forward(&Input::new("")).unwrap();
        let mut inner = vec![];
        let classes = dfa.byte_classes();
        for i in 0..classes.alphabet_len() {
            inner.push((state, vec![], classes.elements(Unit::u8(i as u8))));
        }
        Self {
            dfa,
            classes,
            inner,
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
            let (state, current, mut elements) = self.inner.pop()?;
            let next = elements.next();
            if let Some(b) = next {
                if let Some(b) = b.as_u8() {
                    let new_state = self.dfa.next_state(state, b);
                    if !self.dfa.is_dead_state(new_state) {
                        let mut new = current.clone();
                        new.push(b);
                        self.inner.push((state, current, elements));

                        for i in 0..self.classes.alphabet_len() {
                            self.inner.push((
                                new_state,
                                new.clone(),
                                self.classes.elements(Unit::u8(i as u8)),
                            ));
                        }

                        let eoi_state = self.dfa.next_eoi_state(new_state);
                        if self.dfa.is_match_state(eoi_state) {
                            return Some(new);
                        }
                    }
                } else {
                    let new_state = self.dfa.next_eoi_state(state);
                    if self.dfa.is_match_state(new_state) {
                        return Some(current);
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
        let x = Dfs::from(&dfa);
        for b in x.skip(12000).take(20) {
            dbg!(String::from_utf8_lossy(&b));
        }

        // let classes = dfa.byte_classes();
        // dbg!(classes);

        // let mut state = dfa.start_state_forward(&Input::new("")).unwrap();
        // // dbg!(state);

        // let mut output = vec![];

        // 'outer: loop {
        //     let s = dfa.next_eoi_state(state);
        //     if dfa.is_match_state(s) {
        //         dbg!(String::from_utf8_lossy(&output));
        //     }

        //     // try all byte classes
        //     for b in classes.representatives(..) {
        //         if let Some(b) = b.as_u8() {
        //             let s = dfa.next_state(state, b);
        //             if !dfa.is_dead_state(s) {
        //                 output.push(b);
        //                 state = s;
        //                 continue 'outer;
        //             }
        //         }
        //     }
        //     break;
        // }

        // let state = dbg!(dfa.next_state(state, b'0'));
        // let state = dbg!(dfa.next_state(state, b'0'));
        // let state = dbg!(dfa.next_state(state, b'0'));
        // let state = dbg!(dfa.next_state(state, b'0'));
        // let state = dbg!(dfa.next_state(state, b'-'));
        // let state = dbg!(dfa.next_state(state, b'0'));
        // let state = dbg!(dfa.next_state(state, b'0'));
        // let state = dbg!(dfa.next_state(state, b'-'));
        // let state = dbg!(dfa.next_state(state, b'0'));
        // let state = dbg!(dfa.next_state(state, b'0'));
        // // let state = dbg!(dfa.next_eoi_state(state));

        // dbg!(dfa.is_dead_state(state), dfa.is_match_state(state));

        // for i in 0..classes.alphabet_len() {
        //     let reps: Vec<Unit> = classes.elements(Unit::u8(i as u8)).collect();
        //     dbg!(reps);
        // }
    }
}
