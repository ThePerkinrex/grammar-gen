use std::collections::{HashSet, HashMap};

use crate::grammar::{Rule, Semantic, GrammarSymbol, Grammar, Token};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Item {
	rule: Rule,
	position: usize
}


impl Item {
	pub const fn new(rule: Rule) -> Self {
		Self {
			rule, 
			position: 0
		}
	}

	pub fn current_sem(&self) -> Option<Semantic> {
		if self.position == 0 {
			self.rule.initial
		}else{
			self.rule.tokens[self.position-1].1
		}
	}

	pub fn advance(&self) -> Self {
		Self {
			position: (self.position + 1).min(self.rule.tokens.len()+1),
			rule: self.rule.clone(),
		}
	}

	pub fn next_gram_sym(&self) -> Option<GrammarSymbol> {
		self.rule.tokens.get(self.position).map(|(x, _)| *x)
	}

	pub fn print(&self, grammar: &Grammar) {
        print!("{} -> ", grammar.get_symbol(self.rule.symbol));
		if self.position == 0 {
			print!("· ");
		}
		for (i, (e, _)) in self.rule.tokens.iter().enumerate() {
			print!("{} ", grammar.get_grammar_symbol(*e));
			if (i + 1) == self.position {
				print!("· ");
			}
		}
		println!();
    }
}

pub fn closure(mut items: HashSet<Item>, grammar: &Grammar) -> HashSet<Item> {
	let mut old_items_len = 0;
	while old_items_len != items.len() {
		old_items_len = items.len();
		for item in items.iter().cloned().collect::<Vec<_>>() {
			if let Some(GrammarSymbol::Symbol(sym)) = item.next_gram_sym() {
				for rule in grammar.get_rules().into_iter().filter(|x| x.symbol == sym) {
					if !items.insert(Item::new(rule.clone())) {
						break
					}
				}
			}
		}
	}

	items
}

pub struct AutomataState {
	state: usize,
	shift_actions: HashMap<Token, usize>,
	reduce_actions: HashMap<Option<Token>, Rule>,
	semantic_action: Option<Semantic>
}

pub struct Automata {

}