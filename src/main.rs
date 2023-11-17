use std::collections::HashSet;

use closures::{Item, closure};

use crate::grammar::Grammar;

mod grammar;
mod closures;

fn main() {
    let mut g = Grammar::new(include_str!("test.grammar.txt").lines());
    g.print();
    let axiom = g.get_rules()[0].clone();
    let mut axiom_closure = HashSet::new();
    axiom_closure.insert(Item::new(axiom));
    let i0 = closure(axiom_closure, &g);
    for i in i0 {
        i.print(&g);
    }
}
