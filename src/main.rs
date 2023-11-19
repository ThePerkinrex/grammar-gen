use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env::args,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
};

use closures::{closure, Automata, Item};
use config::Config;
use context::{ReduceContext, ShiftContext, GotoContext, SemStateContext, SemReduceContext};
use grammar::Token;
use tinytemplate::TinyTemplate;

use crate::grammar::Grammar;

mod closures;
mod config;
mod context;
mod grammar;

fn format_token(token: Token, grammar: &Grammar, replacements: &HashMap<String, String>) -> String {
    let g = grammar.get_token(token);
    replacements.get(g).cloned().unwrap_or_else(|| {
        eprintln!("WARNING: {g} not replaced");
        g.to_string()
    })
}

fn format_token_maybe(token: Option<Token>, grammar: &Grammar, replacements: &HashMap<String, String>) -> String {
    let g = token.map_or("$", |token| grammar.get_token(token));
    replacements.get(g).cloned().unwrap_or_else(|| {
        eprintln!("WARNING: {g} not replaced");
        g.to_string()
    })
}

fn main() {
    let config_path = PathBuf::from(args().nth(1).expect("A config file"));
    let config: Config = serde_json::from_reader(File::open(&config_path).expect("Existing file"))
        .expect("Valid json config file");
    let grammar_path = config_path.parent().unwrap().join(config.grammar);

    let mut grammar = Grammar::new(
        BufReader::new(File::open(grammar_path).expect("A grammar file"))
            .lines()
            .flatten()
            .map(Cow::Owned),
    );
    let automata = Automata::new(&mut grammar);
    automata.print(&grammar);

    let mut tt = TinyTemplate::new();
    let reduce_template = config.reduceTemplate.load_string();
    tt.add_template("reduce", &reduce_template)
        .expect("Valid reduce template");
    let shift_template = config.shiftTemplate.load_string();
    tt.add_template("shift", &shift_template)
        .expect("Valid shift template");
    let goto_template = config.gotoTemplate.load_string();
    tt.add_template("goto", &goto_template)
        .expect("Valid goto template");
    let sem_state_template = config.semantics.stateTemplate.load_string();
    tt.add_template("semantic/state", &sem_state_template)
        .expect("Valid semantic state template");
    let sem_reduce_template = config.semantics.reduceTemplate.load_string();
    tt.add_template("semantic/reduce", &sem_reduce_template)
        .expect("Valid semantic reduce template");

    let mut sinks = Vec::new();
    let mut shift_sinks = Vec::new();
    let mut reduce_sinks = Vec::new();
    let mut goto_sinks = Vec::new();
    let mut sem_state_sinks = Vec::new();
    let mut sem_reduce_sinks = Vec::new();

    for (path, rules) in config.results {
        let sinkno = sinks.len();
        sinks.push(File::create(path).expect("valid path"));
        if rules.contains(&config::PrintOption::Shift) {
            shift_sinks.push(sinkno);
        }
        if rules.contains(&config::PrintOption::Reduce) {
            reduce_sinks.push(sinkno);
        }
        if rules.contains(&config::PrintOption::Goto) {
            goto_sinks.push(sinkno);
        }
        if rules.contains(&config::PrintOption::StateSemantics) {
            sem_state_sinks.push(sinkno);
        }
        if rules.contains(&config::PrintOption::ReduceSemantics) {
            sem_reduce_sinks.push(sinkno);
        }
    }

    for state in automata.iter() {
        for (&token, &next) in state.shift_actions.iter() {
            let formatted = tt
                .render(
                    "shift",
                    &ShiftContext {
                        state: state.state,
                        token: &format_token(token, &grammar, &config.tokenReplace),
                        next,
                    },
                )
                .expect("Ability to format shift");
            for sink in &shift_sinks {
                writeln!(sinks[*sink], "{formatted}").unwrap();
            }
        }
        for (&token, &ruleno) in state.reduce_actions.iter() {
            let rule = &grammar.get_rules()[ruleno];
            let symbol_not_replaced = grammar.get_symbol(rule.symbol);
            let formatted = tt
                .render(
                    "reduce",
                    &ReduceContext {
                        state: state.state,
                        token: &format_token_maybe(token, &grammar, &config.tokenReplace),
                        ruleno,
                        elements: rule.tokens.len(),
                        symbolNotReplaced: symbol_not_replaced,
                        symbolNo: rule.symbol
                    },
                )
                .expect("Ability to format reduce");
            for sink in &reduce_sinks {
                writeln!(sinks[*sink], "{formatted}").unwrap();
            }
        }
        for (&symbol, &next) in state.goto_actions.iter() {
            let symbol_not_replaced = grammar.get_symbol(symbol);
            let formatted = tt
                .render(
                    "goto",
                    &GotoContext {
                        state: state.state,
                        symbolNotReplaced: symbol_not_replaced,
                        symbolNo: symbol,
                        next
                    },
                )
                .expect("Ability to format goto");
            for sink in &goto_sinks {
                writeln!(sinks[*sink], "{formatted}").unwrap();
            }
        }
    }

    for (state, sem) in automata.iter_state_sem() {
        let formatted = tt
                .render(
                    "semantic/state",
                    &SemStateContext {
                        state,
                        semantic: sem,
                        semanticName: grammar.get_semantic(sem)
                    },
                )
                .expect("Ability to format semantic state");
        for sink in &sem_state_sinks {
            writeln!(sinks[*sink], "{formatted}").unwrap();
        }
    }

    for (ruleno, sem) in automata.iter_reduce_sem() {
        let formatted = tt
                .render(
                    "semantic/reduce",
                    &SemReduceContext {
                        ruleno,
                        semantic: sem,
                        semanticName: grammar.get_semantic(sem)
                    },
                )
                .expect("Ability to format semantic reduce");
        for sink in &sem_reduce_sinks {
            writeln!(sinks[*sink], "{formatted}").unwrap();
        }
    }
}
