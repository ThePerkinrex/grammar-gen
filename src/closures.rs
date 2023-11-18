use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::grammar::{Grammar, GrammarSymbol, Rule, Semantic, Symbol, Token};

#[derive(Debug, Clone)]
pub struct Item {
    rule: Rule,
    position: usize,
    ruleno: usize,
}

impl std::hash::Hash for Item {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.position.hash(state);
        self.ruleno.hash(state);
    }
}

impl PartialEq for Item {
    fn eq(&self, other: &Self) -> bool {
        self.position == other.position && self.ruleno == other.ruleno
    }
}

impl Eq for Item {}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.ruleno.partial_cmp(&other.ruleno) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.position.partial_cmp(&other.position)
    }
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.ruleno.cmp(&other.ruleno) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.position.cmp(&other.position)
    }
}

impl Item {
    pub const fn new(rule: Rule, ruleno: usize) -> Self {
        Self {
            rule,
            position: 0,
            ruleno,
        }
    }

    pub fn current_sem(&self) -> Option<Semantic> {
        if self.position == 0 {
            self.rule.initial
        } else {
            self.rule.tokens[self.position - 1].1
        }
    }

    pub fn advance(&self) -> Self {
        Self {
            position: (self.position + 1).min(self.rule.tokens.len() + 1),
            rule: self.rule.clone(),
            ruleno: self.ruleno,
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

    pub fn eprint(&self, grammar: &Grammar) {
        eprint!("{} -> ", grammar.get_symbol(self.rule.symbol));
        if self.position == 0 {
            eprint!("· ");
        }
        for (i, (e, _)) in self.rule.tokens.iter().enumerate() {
            eprint!("{} ", grammar.get_grammar_symbol(*e));
            if (i + 1) == self.position {
                eprint!("· ");
            }
        }
        eprintln!();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Closure {
    set: HashSet<Rc<Item>>,
    data: Vec<Rc<Item>>,
}

impl Closure {
    fn new() -> Self {
        Self {
            set: HashSet::new(),
            data: Vec::new(),
        }
    }

    pub fn add(&mut self, item: Item) -> bool {
        let rc = Rc::new(item);
        if self.set.insert(rc.clone()) {
            self.data
                .insert(self.data.binary_search(&rc).unwrap_err(), rc);
            true
        } else {
            false
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn iter(&self) -> Vec<Rc<Item>> {
        self.data.clone()
    }
    pub fn ref_iter(&self) -> impl Iterator<Item = Rc<Item>> + '_ {
        self.data.iter().cloned()
    }
}

impl std::hash::Hash for Closure {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

pub fn closure(mut items: Closure, grammar: &Grammar) -> Closure {
    let mut old_items_len = 0;
    while old_items_len != items.len() {
        old_items_len = items.len();
        for item in items.iter() {
            if let Some(GrammarSymbol::Symbol(sym)) = item.next_gram_sym() {
                for (i, rule) in grammar
                    .get_rules()
                    .into_iter()
                    .enumerate()
                    .filter(|(_, x)| x.symbol == sym)
                {
                    if !items.add(Item::new(rule.clone(), i)) {
                        break;
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
    reduce_actions: HashMap<Option<Token>, usize>,
    goto_actions: HashMap<Symbol, usize>,
    semantic_action: Option<Semantic>,
}

impl AutomataState {
    pub fn new(state: usize) -> Self {
        Self {
            state,
            shift_actions: HashMap::new(),
            reduce_actions: HashMap::new(),
            goto_actions: HashMap::new(),
            semantic_action: None,
        }
    }
}

pub struct Automata {
    states: HashMap<Rc<Closure>, AutomataState>,
}

impl Automata {
    pub fn new(grammar: &mut Grammar) -> Self {
        grammar.get_rules().first().cloned().map_or_else(|| Self {
				states: HashMap::new()
			}, |axiom| {
			let mut states = HashMap::new();
			let mut todo = Vec::new();

			let i0 = Rc::new(closure({let mut hs = Closure::new(); hs.add(Item::new(axiom, 0)); hs}, grammar));
			states.insert(i0.clone(), AutomataState::new(0));
			todo.push(i0);
			while !todo.is_empty() {
				let next_state = todo.remove(0);
				let mut goto_items = HashMap::<_, Vec<_>>::new();
				let mut shift_items = HashMap::<_, Vec<_>>::new();
				let mut reduce_items = HashMap::<_, _>::new();
				let mut sem_action = None;
				for item in next_state.ref_iter() {
					match item.next_gram_sym() {
						None => {
							for x in grammar.follow(item.rule.symbol).as_ref() {
								if let Some(old ) = reduce_items.insert(*x, item.ruleno) {
									eprintln!("Reduce - reduce conflict between rule {} and rule {} @ state {}", old, item.ruleno, states.get(&next_state).unwrap().state);
								}
							}
						},
						Some(GrammarSymbol::Symbol(s)) => {
							goto_items.entry(s).or_default().push(item.advance())
						},
						Some(GrammarSymbol::Token(t)) => {
							shift_items.entry(t).or_default().push(item.advance())
						}
					}
					if let Some(x) = item.current_sem() {
						if let Some(other) = sem_action {
							eprintln!("Multiple semantic actions ({} and {}) @ state {} with closure:", grammar.get_semantic(x), grammar.get_semantic(other), states.get(&next_state).unwrap().state);
							for item in next_state.ref_iter() {
								item.eprint(grammar);
							}
						}else{
							sem_action = Some(x);
						}
					}
				}

				states.get_mut(&next_state).unwrap().semantic_action = sem_action;

				for (s, items) in goto_items {
					let mut c = Closure::new();
					for item in items {
						c.add(item);
					}
					c = closure(c, grammar);
					let len = states.len();
					let goto_state = states.entry(Rc::new(c)).or_insert_with_key(|k| {
						todo.push(k.clone());
						AutomataState::new(len)
					}).state;
					states.get_mut(&next_state).unwrap().goto_actions.insert(s, goto_state);
				}

				for (t, items) in shift_items {
					if let Some(rule) = reduce_items.get(&Some(t)) {
						eprintln!("REDUCE - SHIFT Conflict: Reduce by rule {rule} for {} @ state {}", grammar.get_token(t), states.get(&next_state).unwrap().state)
					}
					let mut c = Closure::new();
					for item in items {
						c.add(item);
					}
					c = closure(c, grammar);
					let len = states.len();
					let shift_state = states.entry(Rc::new(c)).or_insert_with_key(|k| {
						todo.push(k.clone());
						AutomataState::new(len)
					}).state;
					states.get_mut(&next_state).unwrap().shift_actions.insert(t, shift_state);
				}

				states.get_mut(&next_state).unwrap().reduce_actions = reduce_items;

			}

			Self {states}
		})
    }

    pub fn print(&self, grammar: &Grammar) {
        let mut states = self.states.iter().collect::<Vec<_>>();
        states.sort_by_key(|(_, state)| state.state);
        for (closure, state) in states {
            println!();
            println!();
            println!("############# i{}", state.state);
            for item in closure.ref_iter() {
                item.print(grammar);
            }
            println!();
            println!("SHIFT TRANSITIONS");
            for (t, state) in &state.shift_actions {
                println!("{} -> i{state}", grammar.get_token(*t));
            }
            println!("GOTO TRANSITIONS");
            for (t, state) in &state.goto_actions {
                println!("{} -> i{state}", grammar.get_symbol(*t));
            }
            println!("REDUCE TRANSITIONS");
            for (t, rule) in &state.reduce_actions {
                println!("{} -> Rule {rule}", t.map_or("$", |t| grammar.get_token(t)));
            }
            if let Some(sem) = &state.semantic_action {
                println!("SEMANTIC: {}", grammar.get_semantic(*sem));
            }
        }
    }
}
