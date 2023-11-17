use std::collections::HashSet;

use closures::{closure, Automata, Item};

use crate::grammar::Grammar;

mod closures;
mod grammar;

fn main() {
    let mut g = Grammar::new(include_str!("test.grammar.txt").lines());
    g.print();
    let automata = Automata::new(&mut g);
    automata.print(&g);
}
