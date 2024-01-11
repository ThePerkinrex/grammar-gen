use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum TemplateSource {
    String(String),
    File { file: PathBuf },
}

impl TemplateSource {
    pub fn load_string(self) -> String {
        match self {
            Self::String(s) => s,
            Self::File { file } => std::fs::read_to_string(file).expect("valid file"),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum SemanticTemplateSource {
    Switch {
        case: TemplateSource,
        body: TemplateSource,
    },
    Line {
        line: TemplateSource,
    },
}

#[derive(Debug, serde::Deserialize, Clone, Copy, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum PrintOption {
    Shift,
    Reduce,
    Goto,
    #[serde(rename = "semantics/state")]
    StateSemantics,
    #[serde(rename = "semantics/reduce")]
    ReduceSemantics,
    Dot,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub grammar: PathBuf,
    pub reduce_template: TemplateSource,
    pub shift_template: TemplateSource,
    pub goto_template: TemplateSource,
    pub token_replace: HashMap<String, String>,
    pub semantics: SemanticsConfig,
    pub results: HashMap<PathBuf, HashSet<PrintOption>>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum SingleOrMultiLineString {
    Single(String),
    Multiline(Vec<String>)
}

impl ToString for SingleOrMultiLineString {
    fn to_string(&self) -> String {
        match self {
            SingleOrMultiLineString::Single(s) => s.to_string(),
            SingleOrMultiLineString::Multiline(v) => v.join("\n"),
        }
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct SemanticsConfig {
    pub reduce_template: SemanticTemplateSource,
    pub state_template: SemanticTemplateSource,
    pub replacements: HashMap<String, SingleOrMultiLineString>,
}
