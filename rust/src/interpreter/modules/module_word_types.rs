use crate::error::Result;
use crate::interpreter::Interpreter;
use crate::types::{Capabilities, Stability};

pub(super) type ModuleExecutor = fn(&mut Interpreter) -> Result<()>;

#[derive(Clone)]
pub(super) struct ModuleWord {
    pub short_name: &'static str,
    pub description: &'static str,
    pub executor: ModuleExecutor,
    pub preserves_modes: bool,
    pub stability: Stability,
    pub capabilities: Capabilities,
}

#[derive(Clone)]
pub(super) struct SampleWord {
    pub name: &'static str,
    pub definition: &'static str,
    pub description: &'static str,
}

#[derive(Clone)]
pub(super) struct ModuleSpec {
    pub name: &'static str,
    pub words: &'static [ModuleWord],
    pub sample_words: &'static [SampleWord],
}
