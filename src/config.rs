use std::{path::PathBuf, collections::{HashMap, HashSet}};

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum TemplateSource {
	String(String),
	File{
		file: PathBuf
	}
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
		body: TemplateSource
	},
	Line {
		line: TemplateSource
	}
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
	ReduceSemantics
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
	pub grammar: PathBuf,
	pub reduceTemplate: TemplateSource,
	pub shiftTemplate: TemplateSource,
	pub gotoTemplate: TemplateSource,
	pub tokenReplace: HashMap<String, String>,
	pub semantics: SemanticsConfig,
	pub results: HashMap<PathBuf, HashSet<PrintOption>>
}


#[derive(Debug, serde::Deserialize)]
pub struct SemanticsConfig {
	pub reduceTemplate: TemplateSource,
	pub stateTemplate: TemplateSource,
	pub replacements: HashMap<String,String>
}