use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    process::exit,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct Symbol(usize);
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GrammarSymbol {
    Token(Token),
    Symbol(Symbol),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct Semantic(usize);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Rule {
    pub symbol: Symbol,
    pub tokens: Vec<GrammarSymbol>,
    pub semantics: Vec<Option<Semantic>>,
    pub reduce_sem: Option<Semantic>,
}

#[derive(Debug)]
pub struct Grammar {
    rules: Vec<Rule>,
    symbols: Vec<String>,
    tokens: Vec<String>,
    semantics: Vec<String>,
    firsts: HashMap<Vec<GrammarSymbol>, HashSet<Option<Token>>>,
    follows: HashMap<Symbol, HashSet<Option<Token>>>,
}

fn add_or_get<T, U: Copy, F: FnOnce(U) -> T, C: Fn(&T, U) -> bool>(
    arr: &mut Vec<T>,
    data: U,
    cond: C,
    map: F,
) -> usize {
    arr.iter()
        .enumerate()
        .find_map(|(i, s)| if cond(s, data) { Some(i) } else { None })
        .unwrap_or_else(|| {
            arr.push(map(data));
            arr.len() - 1
        })
}

impl Grammar {
    pub fn new<'a, I: Iterator<Item = Cow<'a, str>>>(lines: I) -> Self {
        let mut symbols = Vec::new();
        let mut tokens = Vec::new();
        let mut semantics = Vec::new();
        let mut rules = Vec::new();
        let lines = lines.filter(|s| !s.is_empty()).collect::<Vec<_>>();
        let mut rules_unparsed = Vec::with_capacity(lines.len());
        for line in lines {
            let Some((a, b)) = line.split_once("->") else {
                println!("Error in line {line}, ignoring");
                continue;
            };
            let symbol = a.trim();
            let symbol = Symbol(add_or_get(
                &mut symbols,
                symbol,
                |s, a| s == a,
                ToString::to_string,
            ));
            rules_unparsed.push((symbol, b.to_string()));
        }
        for (symbol, b) in rules_unparsed {
            let mut toks = Vec::new();
            let mut reduce_sem = None;
            let mut sems = Vec::new();
            let mut last_sem = false;
            for tok_or_sem in b.split_whitespace() {
                if tok_or_sem.starts_with('{') && tok_or_sem.ends_with('}') {
                    let sem = &tok_or_sem[1..tok_or_sem.len() - 1];
                    let sem = Some(Semantic(add_or_get(
                        &mut semantics,
                        sem,
                        |s, a| s == a,
                        ToString::to_string,
                    )));
                    if last_sem {
                        println!("Error, duplicate semantics");
                        exit(1);
                    }
                    sems.push(sem);
                    last_sem = true;
                } else if tok_or_sem.starts_with("R{") && tok_or_sem.ends_with('}') {
                    let sem = &tok_or_sem[2..tok_or_sem.len() - 1];
                    let sem = Some(Semantic(add_or_get(
                        &mut semantics,
                        sem,
                        |s, a| s == a,
                        ToString::to_string,
                    )));
                    reduce_sem = sem;
                } else {
                    let tok = symbols
                        .iter()
                        .enumerate()
                        .find(|(_, s)| *s == tok_or_sem)
                        .map_or_else(
                            || {
                                GrammarSymbol::Token(Token(add_or_get(
                                    &mut tokens,
                                    tok_or_sem,
                                    |t, a| t == a,
                                    ToString::to_string,
                                )))
                            },
                            |(i, _)| GrammarSymbol::Symbol(Symbol(i)),
                        );
                    toks.push(tok);
                    if !last_sem {
                        sems.push(None);
                    }
                    last_sem = false;
                }
            }

            let rule = Rule {
                symbol,
                tokens: toks,
                semantics: sems,
                reduce_sem,
            };
            rules.push(rule);
        }
        let mut hs = HashSet::with_capacity(1);
        hs.insert(None);
        let mut follows = HashMap::with_capacity(1);
        follows.insert(rules[0].symbol, hs);
        let mut s = Self {
            symbols,
            tokens,
            rules,
            semantics,
            firsts: HashMap::new(),
            follows,
        };
        s.follows();
        s
    }

    pub fn get_rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn get_symbol(&self, symbol: Symbol) -> &str {
        self.symbols.get(symbol.0).map(AsRef::as_ref).unwrap()
    }

    pub fn get_token(&self, token: Token) -> &str {
        self.tokens.get(token.0).map(AsRef::as_ref).unwrap()
    }
    pub fn get_semantic(&self, semantic: Semantic) -> &str {
        self.semantics.get(semantic.0).map(AsRef::as_ref).unwrap()
    }

    pub fn get_grammar_symbol(&self, s: GrammarSymbol) -> &str {
        match s {
            GrammarSymbol::Token(t) => self.get_token(t),
            GrammarSymbol::Symbol(s) => self.get_symbol(s),
        }
    }

    fn first(&mut self, v: &[GrammarSymbol]) -> Cow<'_, HashSet<Option<Token>>> {
        if self.firsts.contains_key(v) {
            return Cow::Borrowed(self.firsts.get(v).unwrap());
        }
        let mut f = HashSet::new();
        f.insert(None);
        let mut iter = v.iter();
        while f.contains(&None) {
            match iter.next() {
                None => break,
                Some(GrammarSymbol::Token(tok)) => {
                    f.remove(&None);
                    f.insert(Some(*tok));
                }
                Some(GrammarSymbol::Symbol(sym)) => {
                    f.remove(&None);
                    #[allow(clippy::needless_collect)]
                    for rule in self
                        .rules
                        .iter()
                        .filter(|r| r.symbol == *sym)
                        .map(|rule| rule.tokens.to_vec())
                        .collect::<Vec<_>>()
                    {
                        f.extend(self.first(&rule).as_ref());
                    }
                }
            }
        }
        self.firsts.insert(v.to_vec(), f.clone());
        Cow::Owned(f)
    }

    fn first_non_mut(&self, v: &[GrammarSymbol]) -> HashSet<Option<Token>> {
        let mut f = HashSet::new();
        f.insert(None);
        let mut iter = v.iter();
        while f.contains(&None) {
            match iter.next() {
                None => break,
                Some(GrammarSymbol::Token(tok)) => {
                    f.remove(&None);
                    f.insert(Some(*tok));
                }
                Some(GrammarSymbol::Symbol(sym)) => {
                    f.remove(&None);
                    #[allow(clippy::needless_collect)]
                    for rule in self
                        .rules
                        .iter()
                        .filter(|r| r.symbol == *sym)
                        .map(|rule| rule.tokens.to_vec())
                        .collect::<Vec<_>>()
                    {
                        f.extend(self.first_non_mut(&rule));
                    }
                }
            }
        }
        f
    }

    // fn print_set(&self, set: &HashSet<Option<Token>>) {
    //     print!("{{");
    //     for x in set {
    //         print!("{}, ", x.map_or("None", |x| self.get_token(x)))
    //     }
    //     print!("}}");
    // }

    pub fn follow(&mut self, v: Symbol) -> Cow<'_, HashSet<Option<Token>>> {
        if self.follows.contains_key(&v) {
            return Cow::Borrowed(self.follows.get(&v).unwrap());
        }
        unreachable!()
        // println!();
        // print!("Follow {} ", self.get_symbol(v));
        // self.follows.insert(v, HashSet::new());
        // let mut f = HashSet::new();
        // let mut oldset = None;

        // while Some(f.clone()) != oldset {
        //     oldset = Some(f.clone());
        //     #[allow(clippy::needless_collect)]
        //     for (sym, rule) in self
        //         .rules
        //         .iter()
        //         .map(|x| (x.symbol, x.tokens.to_vec()))
        //         .collect::<Vec<_>>()
        //     {
        //         // print!("{} -> ", self.get_symbol(sym));
        //         let mut set = HashSet::<Option<Token>>::new();
        //         for i in 0..rule.len() {
        //             // print!("{}: {}, ", self.get_grammar_symbol(rule[i]), rule[i] == GrammarSymbol::Symbol(v));
        //             if rule[i] == GrammarSymbol::Symbol(v) {
        //                 set.extend(self.first(&rule[i + 1..]).as_ref())
        //             }
        //             // self.print_set(&f);
        //         }
        //         // println!();

        //         if set.contains(&None) {
        //             f.extend(self.follow(sym).as_ref().iter().copied());
        //         }
        //         f.extend(set.into_iter().flatten().map(Some));
        //     }
        // }

        // // println!();

        // self.follows.insert(v, f.clone());
        // Cow::Owned(f)
    }

    pub fn follows(&mut self) {
        // Initialize all sets to an empty set
        for symbol in 0..self.symbols.len() {
            let symbol = Symbol(symbol);
            self.follows.entry(symbol).or_default();
        }

        let mut follows = self.follows.clone();

        let mut oldsets = None;
        while Some(&follows) != oldsets.as_ref() {
            oldsets = Some(follows.clone());
            for rule in &self.rules {
                for i in 0..rule.tokens.len() {
                    if let GrammarSymbol::Symbol(b) = rule.tokens[i] {
                        let mut f = self.first_non_mut(&rule.tokens[(i + 1)..]);
                        if f.contains(&None) {
                            f.remove(&None);
                            let follows_a = follows.get(&rule.symbol).unwrap().clone();
                            follows.get_mut(&b).unwrap().extend(follows_a);
                        }
                        follows.get_mut(&b).unwrap().extend(f);
                    }
                }
            }
        }
        self.follows = follows;
    }

    pub fn print(&mut self) {
        println!("Grammar:");
        for (i, rule) in self.rules.iter().enumerate() {
            print!("{i:>4} {} -> ", self.get_symbol(rule.symbol));
            let mut sems = rule.semantics.iter();
            let mut toks = rule.tokens.iter();
            while let (Some(sem), Some(tok)) = (sems.next(), toks.next()) {
                if let Some(sem) = sem {
                    print!("{{{}}} ", self.get_semantic(*sem))
                }
                print!("{} ", self.get_grammar_symbol(*tok))
            }
            if let Some(Some(last_sem)) = sems.next() {
                print!("{{{}}} ", self.get_semantic(*last_sem))
            }
            if let Some(red) = rule.reduce_sem {
                print!("R{{{}}}", self.get_semantic(red))
            }
            println!();
            // println!("SYMBOLS: {:?}", rule.tokens);
            // println!("SEMANTICS: {:?}", rule.semantics);
        }
        println!();
        println!("Tokens:");
        for (i, tok) in self.tokens.iter().enumerate() {
            println!("{i:>4} {tok}");
        }
        println!();
        println!("Symbols:");
        for (i, tok) in self.symbols.iter().enumerate() {
            println!("{i:>4} {tok}");
        }
        println!();
        println!("Semantics:");
        for (i, tok) in self.semantics.iter().enumerate() {
            println!("{i:>4} {tok}");
        }
        println!();
        println!("Firsts:");
        for i in 0..self.symbols.len() {
            print!("{:>4} = {{", self.get_symbol(Symbol(i)));
            let first = self.first(&[GrammarSymbol::Symbol(Symbol(i))]);
            for f in first.as_ref().clone() {
                print!("{}, ", f.map_or("lambda", |x| self.get_token(x)))
            }
            println!("}}");
        }
        println!();
        println!("Follows:");
        for i in 0..self.symbols.len() {
            print!("{:>4} = {{", self.get_symbol(Symbol(i)));

            let follow = self.follow(Symbol(i));
            for f in follow.as_ref().clone() {
                print!("{}, ", f.map_or("$", |x| self.get_token(x)))
            }
            println!("}}");
        }
    }
    // pub fn print(&mut self) {
    //     println!("Grammar:");
    //     for (i, rule) in self.rules.iter().enumerate() {
    //         print!("{i:>4} {} -> ", self.get_symbol(rule.symbol));
    //         if let Some(sem) = rule.initial {
    //             print!("{{{}}} ", self.get_semantic(sem));
    //         }
    //         for (tok, sem) in &rule.tokens {
    //             print!("{} ", self.get_grammar_symbol(*tok));
    //             if let Some(sem) = sem {
    //                 print!("{{{}}} ", self.get_semantic(*sem));
    //             }
    //         }
    //         println!();
    //     }
    //     println!();
    //     println!("Tokens:");
    //     for (i, tok) in self.tokens.iter().enumerate() {
    //         println!("{i:>4} {tok}");
    //     }
    //     println!();
    //     println!("Symbols:");
    //     for (i, tok) in self.symbols.iter().enumerate() {
    //         println!("{i:>4} {tok}");
    //     }
    //     println!();
    //     println!("Semantics:");
    //     for (i, tok) in self.semantics.iter().enumerate() {
    //         println!("{i:>4} {tok}");
    //     }
    //     println!();
    //     println!("Firsts:");
    //     for i in 0..self.symbols.len() {
    //         print!("{:>4} = {{", self.get_symbol(Symbol(i)));

    //         let first = self.first(&[GrammarSymbol::Symbol(Symbol(i))]);
    //         for f in first.as_ref().clone() {
    //             print!("{}, ", f.map_or("lambda", |x| self.get_token(x)))
    //         }
    //         println!("}}");
    //     }
    //     println!();
    //     println!("Follows:");
    //     for i in 0..self.symbols.len() {
    //         print!("{:>4} = {{", self.get_symbol(Symbol(i)));

    //         let follow = self.follow(Symbol(i));
    //         for f in follow.as_ref().clone() {
    //             print!("{}, ", f.map_or("$", |x| self.get_token(x)))
    //         }
    //         println!("}}");
    //     }
    // }
}
