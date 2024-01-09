use std::{
    borrow::Cow,
    collections::HashMap,
    env::args,
    fmt::Display,
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf, char::DecodeUtf16,
};

use closures::Automata;
use config::{Config, SemanticTemplateSource};
use context::{
    GotoContext, ReduceContext, SemBodyContext, SemContext, SemReduceCaseContext,
    SemStateCaseContext, ShiftContext,
};
use grammar::{Semantic, Token};
use tinytemplate::TinyTemplate;
use typed_arena::Arena;

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

fn format_token_maybe(
    token: Option<Token>,
    grammar: &Grammar,
    replacements: &HashMap<String, String>,
) -> String {
    let g = token.map_or("$", |token| grammar.get_token(token));
    replacements.get(g).cloned().unwrap_or_else(|| {
        eprintln!("WARNING: {g} not replaced");
        g.to_string()
    })
}

enum SemanticTemplateGen<'a> {
    Switch { case: &'a str, body: &'a str },
    Line { line: &'a str },
}

impl<'a> SemanticTemplateGen<'a> {
    fn render<
        'b,
        T,
        C: serde::Serialize,
        F: Fn(&T) -> C,
        B: Fn(Semantic) -> SemBodyContext<'b>,
        R: FnMut(tinytemplate::error::Result<String>),
    >(
        &self,
        tt: &TinyTemplate,
        sem: Semantic,
        data: &[T],
        case: F,
        body: B,
        re: &mut R,
    ) {
        match self {
            SemanticTemplateGen::Switch { case: c, body: b } => {
                for s in data {
                    let s = case(s);
                    re(tt.render(c, &s));
                }
                let s = body(sem);
                re(tt.render(b, &s));
            }
            SemanticTemplateGen::Line { line } => {
                for s in data {
                    let s = SemContext {
                        case: case(s),
                        body: body(sem),
                    };

                    re(tt.render(line, &s));
                }
            }
        }
    }
}

fn add_templates<'b, S: Display>(
    tt: &mut TinyTemplate<'b>,
    heap: &'b Arena<String>,
    name: S,
    template: SemanticTemplateSource,
) -> SemanticTemplateGen<'b> {
    match template {
        SemanticTemplateSource::Switch {
            case: case_t,
            body: body_t,
        } => {
            let case = heap.alloc(format!("{}/case", name)).as_str();
            let text = heap.alloc(case_t.load_string()).as_str();
            tt.add_template(case, text).expect("Valid case template");
            let body = heap.alloc(format!("{}/body", name)).as_str();
            let text = heap.alloc(body_t.load_string()).as_str();
            tt.add_template(body, text).expect("Valid body template");
            SemanticTemplateGen::Switch { case, body }
        }
        SemanticTemplateSource::Line { line } => {
            let name = heap.alloc(format!("{}/line", name)).as_str();
            let text = heap.alloc(line.load_string()).as_str();
            tt.add_template(name, text).expect("Valid line template");
            SemanticTemplateGen::Line { line: name }
        }
    }
}

fn main() {
    let config_path = PathBuf::from(args().nth(1).expect("A config file"));
    let config: Config = serde_json::from_reader(File::open(&config_path).expect("Existing file"))
        .expect("Valid json config file");
    let config_parent = config_path.parent().unwrap();
    let grammar_path = config_parent.join(config.grammar);

    let mut grammar = Grammar::new(
        BufReader::new(File::open(grammar_path).expect("A grammar file"))
            .lines()
            .flatten()
            .map(Cow::Owned),
    );
    grammar.print();
    let automata = Automata::new(&mut grammar);
    automata.print(&grammar);
    let arena = Arena::new();
    let mut tt = TinyTemplate::new();
    let reduce_template = config.reduce_template.load_string();
    tt.add_template("reduce", &reduce_template)
        .expect("Valid reduce template");
    let shift_template = config.shift_template.load_string();
    tt.add_template("shift", &shift_template)
        .expect("Valid shift template");
    let goto_template = config.goto_template.load_string();
    tt.add_template("goto", &goto_template)
        .expect("Valid goto template");
    let sem_state = add_templates(
        &mut tt,
        &arena,
        "semantic/state",
        config.semantics.state_template,
    );

    let sem_reduce = add_templates(
        &mut tt,
        &arena,
        "semantic/reduce",
        config.semantics.reduce_template,
    );

    let mut sinks = Vec::new();
    let mut shift_sinks = Vec::new();
    let mut reduce_sinks = Vec::new();
    let mut goto_sinks = Vec::new();
    let mut sem_state_sinks = Vec::new();
    let mut sem_reduce_sinks = Vec::new();
    let mut dot_sinks = Vec::new();

    for (path, rules) in config.results {
        let sinkno = sinks.len();
        sinks.push(File::create(config_parent.join(path)).expect("valid path"));
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
        if rules.contains(&config::PrintOption::Dot) {
            dot_sinks.push(sinkno);
        }
    }

    for sink in &dot_sinks {
        writeln!(sinks[*sink], "digraph automata {{").unwrap();
    }
    let mut states = automata.iter_all().collect::<Vec<_>>();
    states.sort_by_key(|(_, s)| s.state);
    for (closure, state) in states {
        let string = closure.ref_iter().map(|item|item.to_string(&grammar)).collect::<Vec<_>>().join("\\n\t");
        for sink in &dot_sinks {
            writeln!(sinks[*sink], "\ti{} [label=\"i{0}\\n\t{}\"];", state.state, string).unwrap();
        }
    }

    for state in automata.iter() {
        for (&token, &next) in state.shift_actions.iter() {
            let formatted = tt
                .render(
                    "shift",
                    &ShiftContext {
                        state: state.state,
                        token: &format_token(token, &grammar, &config.token_replace),
                        next,
                    },
                )
                .expect("Ability to format shift");
            for sink in &shift_sinks {
                writeln!(sinks[*sink], "{formatted}").unwrap();
            }
            for sink in &dot_sinks {
                writeln!(sinks[*sink], "\ti{} -> i{} [label=\"{}\"];", state.state, next, grammar.get_token(token)).unwrap();
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
                        token: &format_token_maybe(token, &grammar, &config.token_replace),
                        ruleno,
                        elements: rule.tokens.len(),
                        symbol_not_replaced,
                        symbol_no: rule.symbol,
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
                        symbol_not_replaced,
                        symbol_no: symbol,
                        next,
                    },
                )
                .expect("Ability to format goto");
            for sink in &goto_sinks {
                writeln!(sinks[*sink], "{formatted}").unwrap();
            }

            for sink in &dot_sinks {
                writeln!(sinks[*sink], "\ti{} -> i{} [label=\"{}\"];", state.state, next, grammar.get_symbol(symbol)).unwrap();
            }
        }
    }

    let mut render = |s: tinytemplate::error::Result<String>| {
        let formatted = s.expect("Valid sem state render");
        for sink in &sem_state_sinks {
            writeln!(sinks[*sink], "{formatted}").unwrap();
        }
    };
    for (sem, states) in
        automata
            .iter_state_sem()
            .fold(HashMap::<_, Vec<_>>::new(), |mut hm, (ruleno, sem)| {
                hm.entry(sem).or_default().push(ruleno);
                hm
            })
    {
        sem_state.render(
            &tt,
            sem,
            &states,
            |&state| SemStateCaseContext { state },
            |sem| {
                let semantic_name = grammar.get_semantic(sem);
                SemBodyContext {
                    semantic: sem,
                    semantic_name,
                    semantic_body: config
                        .semantics
                        .replacements
                        .get(semantic_name)
                        .map(AsRef::as_ref)
                        .unwrap_or_default(),
                }
            },
            &mut render,
        );
        // let semanticName = grammar.get_semantic(sem);
        // let formatted = tt
        //     .render(
        //         "semantic/state",
        //         &SemStateContext {
        //             state,
        //             semantic: sem,
        //             semanticName,
        //             semanticBody: config
        //                 .semantics
        //                 .replacements
        //                 .get(semanticName)
        //                 .map(AsRef::as_ref)
        //                 .unwrap_or_default(),
        //         },
        //     )
        //     .expect("Ability to format semantic state");
        // for sink in &sem_state_sinks {
        //     writeln!(sinks[*sink], "{formatted}").unwrap();
        // }
    }

    let mut render = |s: tinytemplate::error::Result<String>| {
        let formatted = s.expect("Valid sem reduce render");
        for sink in &sem_reduce_sinks {
            writeln!(sinks[*sink], "{formatted}").unwrap();
        }
    };
    for (sem, rules) in
        automata
            .iter_reduce_sem()
            .fold(HashMap::<_, Vec<_>>::new(), |mut hm, (ruleno, sem)| {
                hm.entry(sem).or_default().push(ruleno);
                hm
            })
    {
        sem_reduce.render(
            &tt,
            sem,
            &rules,
            |&ruleno| SemReduceCaseContext { ruleno },
            |sem| {
                let semantic_name = grammar.get_semantic(sem);
                SemBodyContext {
                    semantic: sem,
                    semantic_name,
                    semantic_body: config
                        .semantics
                        .replacements
                        .get(semantic_name)
                        .map(AsRef::as_ref)
                        .unwrap_or_default(),
                }
            },
            &mut render,
        );

        // let formatted = tt
        //     .render(
        //         "semantic/reduce",
        //         &SemReduceContext {
        //             ruleno,
        //             semantic: sem,
        //             semanticName,
        //             semanticBody: config
        //                 .semantics
        //                 .replacements
        //                 .get(semanticName)
        //                 .map(AsRef::as_ref)
        //                 .unwrap_or_default(),
        //         },
        //     )
        //     .expect("Ability to format semantic reduce");
        // for sink in &sem_reduce_sinks {
        //     writeln!(sinks[*sink], "{formatted}").unwrap();
        // }

    }
    
    for sink in &dot_sinks {
        writeln!(sinks[*sink], "}}").unwrap();
    }
}
