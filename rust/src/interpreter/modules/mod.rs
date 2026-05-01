mod module_builtins;
mod module_import_execution;
mod module_registry;
mod module_samples;
mod module_word_types;

use crate::coreword_registry::CorewordMetadata;
use crate::error::Result;
use crate::interpreter::Interpreter;

pub fn execute_module_word(interp: &mut Interpreter, name: &str) -> Option<Result<()>> {
    module_registry::execute_module_word(interp, name)
}

pub fn is_mode_preserving_word(name: &str) -> bool {
    module_registry::is_mode_preserving_word(name)
}

pub fn op_import(interp: &mut Interpreter) -> Result<()> {
    module_import_execution::op_import(interp)
}

pub fn op_import_only(interp: &mut Interpreter) -> Result<()> {
    module_import_execution::op_import_only(interp)
}

pub fn restore_module(interp: &mut Interpreter, module_name: &str) -> bool {
    module_import_execution::restore_module(interp, module_name)
}

pub(crate) fn module_word_metadata_entries() -> Vec<CorewordMetadata> {
    module_builtins::module_word_metadata_entries()
}

/// Look up a module word's user-facing description by qualified name
/// (e.g. `"ALGO@SORT"`) or by `(module, short_name)`. Returns `None` if no
/// such canonical module word exists.
pub fn module_word_description(module_name: &str, short_name: &str) -> Option<&'static str> {
    module_builtins::module_word_description(module_name, short_name)
}
