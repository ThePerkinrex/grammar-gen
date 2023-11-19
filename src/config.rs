use std::{path::PathBuf, collections::HashMap};

#[derive(Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum TemplateSource {
	String(String),
	File{
		file: PathBuf
	}
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrintOption {
	Shift,
	Reduce,
	Goto
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
	grammar: PathBuf,
	reduceTemplate: TemplateSource,
	shiftTemplate: TemplateSource,
	gotoTemplate: TemplateSource,
	tokenReplace: HashMap<String, String>,
	symbolReplace: HashMap<String, String>,
	results: HashMap<PathBuf, Vec<PrintOption>>
}