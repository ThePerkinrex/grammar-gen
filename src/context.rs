use crate::grammar::{Symbol, Semantic};

#[derive(Debug, serde::Serialize)]
pub struct ShiftContext<'a> {
	pub state: usize,
	pub token: &'a str,
	pub next: usize
}

#[derive(Debug, serde::Serialize)]
pub struct ReduceContext<'a> {
	pub state: usize,
	pub token: &'a str,
	pub elements: usize,
	pub ruleno: usize,
	pub symbolNo: Symbol,
	pub symbolNotReplaced: &'a str
}

#[derive(Debug, serde::Serialize)]
pub struct GotoContext<'a> {
	pub state: usize,
	pub symbolNo: Symbol,
	pub symbolNotReplaced: &'a str,
	pub next: usize
}

#[derive(Debug, serde::Serialize)]
pub struct SemStateContext<'a> {
	pub state: usize,
	pub semantic: Semantic,
	pub semanticName: &'a str
}

#[derive(Debug, serde::Serialize)]
pub struct SemReduceContext<'a> {
	pub ruleno: usize,
	pub semantic: Semantic,
	pub semanticName: &'a str
}
