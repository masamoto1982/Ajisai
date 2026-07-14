mod module_builtins;
mod module_import_execution;
mod module_registry;
mod module_word_docs;
mod module_word_types;
mod semantic_sync;

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

pub fn op_unimport(interp: &mut Interpreter) -> Result<()> {
    module_import_execution::op_unimport(interp)
}

pub fn op_unimport_only(interp: &mut Interpreter) -> Result<()> {
    module_import_execution::op_unimport_only(interp)
}

pub fn restore_module(interp: &mut Interpreter, module_name: &str) -> bool {
    module_import_execution::restore_module(interp, module_name)
}

pub(crate) use module_builtins::CatalogWord;

/// All importable module names, in specification order.
pub fn available_module_names() -> Vec<&'static str> {
    module_builtins::available_module_names()
}

/// Full word + sample catalog for a module, regardless of import state.
pub(crate) fn module_catalog_words(module_name: &str) -> Option<Vec<CatalogWord>> {
    module_builtins::module_catalog_words(module_name)
}

/// Restore a precise (possibly partial) import state for one module.
pub fn restore_import_entry(
    interp: &mut Interpreter,
    module_name: &str,
    import_all_public: bool,
    words: Vec<String>,
    samples: Vec<String>,
) -> bool {
    module_import_execution::restore_import_entry(
        interp,
        module_name,
        import_all_public,
        words,
        samples,
    )
}

pub(crate) fn is_known_module(module_name: &str) -> bool {
    let upper = module_name.to_uppercase();
    module_builtins::MODULE_SPECS
        .iter()
        .any(|module| module.name == upper)
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

/// Render the four-section LOOKUP body for a module word, accepting either
/// a qualified `MODULE@WORD` name or a bare module word name. Returns
/// `None` if no such word exists.
pub fn lookup_module_word_detail(name: &str) -> Option<String> {
    module_builtins::lookup_module_word_detail(name)
}
