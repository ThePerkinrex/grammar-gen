use std::{collections::HashSet, env::args, fs::File};

use closures::{closure, Automata, Item};
use config::Config;

use crate::grammar::Grammar;

mod closures;
mod grammar;
mod config;

fn main() {
    let config = args().nth(1).expect("A config file");
    let config: Config = serde_json::from_reader(File::open(config).expect("Existing file")).expect("Valid json config file");
    dbg!(config);
}
