use crate::grammar::{Semantic, Symbol};

#[derive(Debug, serde::Serialize)]
pub struct ShiftContext<'a> {
    pub state: usize,
    pub token: &'a str,
    pub next: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct ReduceContext<'a> {
    pub state: usize,
    pub token: &'a str,
    pub elements: usize,
    pub ruleno: usize,
    pub symbol_no: Symbol,
    pub symbol_not_replaced: &'a str,
}

#[derive(Debug, serde::Serialize)]
pub struct GotoContext<'a> {
    pub state: usize,
    pub symbol_no: Symbol,
    pub symbol_not_replaced: &'a str,
    pub next: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct SemStateCaseContext {
    pub state: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct SemReduceCaseContext {
    pub ruleno: usize,
}

#[derive(Debug, serde::Serialize)]
pub struct SemBodyContext<'a> {
    pub semantic: Semantic,
    pub semantic_name: &'a str,
    pub semantic_body: &'a str,
}

#[derive(Debug, serde::Serialize)]
pub struct SemContext<'a, C: 'a> {
    #[serde(flatten)]
    pub case: C,
    #[serde(flatten)]
    pub body: SemBodyContext<'a>,
}
