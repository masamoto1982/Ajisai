use super::builtin_word_definitions::{lookup_builtin_spec, BuiltinDetailGroup};
use super::detail_lookup_arithmetic_logic::lookup_detail_arithmetic_logic;
use super::detail_lookup_cond::lookup_detail_cond;
use super::detail_lookup_control_higher_order::lookup_detail_control_higher_order;
use super::detail_lookup_io_module::lookup_detail_io_module;
use super::detail_lookup_modifier::lookup_detail_modifier;
use super::detail_lookup_string_cast::lookup_detail_string_cast;
use super::detail_lookup_vector_ops::lookup_detail_vector_ops;

pub fn lookup_builtin_detail(name: &str) -> String {
    let canonical = crate::core_word_aliases::canonicalize_core_word_name(name);
    let Some(spec) = lookup_builtin_spec(&canonical) else {
        return format!("# {} - 詳細情報なし\n\n## 機能\n'{}' は組み込みワードではないか、詳細情報が登録されていません。", name, name);
    };

    let detail = match spec.detail_group {
        BuiltinDetailGroup::Modifier => lookup_detail_modifier(spec.name),
        BuiltinDetailGroup::ArithmeticLogic => lookup_detail_arithmetic_logic(spec.name),
        BuiltinDetailGroup::VectorOps => lookup_detail_vector_ops(spec.name),
        BuiltinDetailGroup::StringCast => lookup_detail_string_cast(spec.name),
        BuiltinDetailGroup::ControlHigherOrder => lookup_detail_control_higher_order(spec.name),
        BuiltinDetailGroup::Cond => lookup_detail_cond(spec.name),
        BuiltinDetailGroup::IoModule => lookup_detail_io_module(spec.name),
    };

    let alias_lead = crate::core_word_aliases::lookup_core_word_alias(name)
        .and_then(|alias| {
            alias.canonical.map(|canonical_name| {
                let lead = match alias.kind {
                    crate::core_word_aliases::CoreWordAliasKind::SymbolAlias => {
                        format!("{} is an alias of {}.\n\n", alias.alias, canonical_name)
                    }
                    crate::core_word_aliases::CoreWordAliasKind::SyntaxSugar => {
                        format!(
                            "{} is syntax sugar for {}.\n\n",
                            alias.alias, canonical_name
                        )
                    }
                    crate::core_word_aliases::CoreWordAliasKind::InputHelper => {
                        format!("{} is an input helper.\n\n", alias.alias)
                    }
                    crate::core_word_aliases::CoreWordAliasKind::Deprecated => {
                        format!("{} is deprecated.\n\n", alias.alias)
                    }
                };
                lead
            })
        })
        .unwrap_or_default();

    if let Some(content) = detail {
        return format!("{}{}", alias_lead, content);
    }

    let body = format!(
        "# {name} - {description}\n\n## 機能\n{description}（カテゴリ: {category}, シグネチャ: {signature}）\n\n## 使用例\n{syntax}\n",
        name = spec.name,
        description = spec.short_description,
        category = spec.category,
        signature = spec.signature_type,
        syntax = spec.syntax,
    );

    format!("{}{}", alias_lead, body)
}
