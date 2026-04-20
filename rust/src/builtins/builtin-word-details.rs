

use super::builtin_word_definitions::{lookup_builtin_spec, BuiltinDetailGroup};
use super::detail_lookup_arithmetic_logic::lookup_detail_arithmetic_logic;
use super::detail_lookup_cond::lookup_detail_cond;
use super::detail_lookup_control_higher_order::lookup_detail_control_higher_order;
use super::detail_lookup_io_module::lookup_detail_io_module;
use super::detail_lookup_modifier::lookup_detail_modifier;
use super::detail_lookup_string_cast::lookup_detail_string_cast;
use super::detail_lookup_vector_ops::lookup_detail_vector_ops;


pub fn lookup_builtin_detail(name: &str) -> String {
    let Some(spec) = lookup_builtin_spec(name) else {
        return format!("# {} - 詳細情報なし\n\n## 機能\n'{}' は組み込みワードではないか、詳細情報が登録されていません。", name, name);
    };

    let detail = match spec.detail_group {
        BuiltinDetailGroup::Modifier => lookup_detail_modifier(name),
        BuiltinDetailGroup::ArithmeticLogic => lookup_detail_arithmetic_logic(name),
        BuiltinDetailGroup::VectorOps => lookup_detail_vector_ops(name),
        BuiltinDetailGroup::StringCast => lookup_detail_string_cast(name),
        BuiltinDetailGroup::ControlHigherOrder => lookup_detail_control_higher_order(name),
        BuiltinDetailGroup::Cond => lookup_detail_cond(name),
        BuiltinDetailGroup::IoModule => lookup_detail_io_module(name),
    };

    if let Some(content) = detail {
        return content;
    }

    format!(
        "# {name} - {description}\n\n## 機能\n{description}（カテゴリ: {category}, シグネチャ: {signature}）\n\n## 使用例\n{syntax}\n",
        name = spec.name,
        description = spec.short_description,
        category = spec.category,
        signature = spec.signature_type,
        syntax = spec.syntax,
    )
}
