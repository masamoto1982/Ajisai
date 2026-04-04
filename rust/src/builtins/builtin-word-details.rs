// rust/src/builtins/builtin-word-details.rs
//
// Detailed documentation for built-in words (used by the ? word).
// Delegates to category-specific lookup functions.

use super::builtin_word_definitions::collect_builtin_definitions;
use super::detail_lookup_arithmetic_logic::lookup_detail_arithmetic_logic;
use super::detail_lookup_control_higher_order::lookup_detail_control_higher_order;
use super::detail_lookup_cond::lookup_detail_cond;
use super::detail_lookup_io_module::lookup_detail_io_module;
use super::detail_lookup_modifier::lookup_detail_modifier;
use super::detail_lookup_string_cast::lookup_detail_string_cast;
use super::detail_lookup_vector_ops::lookup_detail_vector_ops;

/// Returns detailed documentation for a built-in word.
/// Used by the `?` word to display help information.
pub fn lookup_builtin_detail(name: &str) -> String {
    if let Some(detail) = lookup_detail_modifier(name) {
        return detail;
    }
    if let Some(detail) = lookup_detail_arithmetic_logic(name) {
        return detail;
    }
    if let Some(detail) = lookup_detail_vector_ops(name) {
        return detail;
    }
    if let Some(detail) = lookup_detail_string_cast(name) {
        return detail;
    }
    if let Some(detail) = lookup_detail_control_higher_order(name) {
        return detail;
    }
    if let Some(detail) = lookup_detail_cond(name) {
        return detail;
    }
    if let Some(detail) = lookup_detail_io_module(name) {
        return detail;
    }

    // Fallback: generate basic info from word definitions
    for (word_name, description, syntax, sig_type) in collect_builtin_definitions() {
        if word_name == name {
            return format!(
                "# {} - {}\n\n## シグネチャタイプ\n{}\n\n## 構文\n{}\n",
                name, description, sig_type, syntax
            );
        }
    }

    format!("'{}' の詳細情報はありません。", name)
}
