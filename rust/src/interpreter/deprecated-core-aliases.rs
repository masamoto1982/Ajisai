pub(crate) struct DeprecatedCoreAlias {
    pub old_name: &'static str,
    pub replacement_qualified: &'static str,
    pub import_hint: &'static str,
}

pub(crate) const DEPRECATED_CORE_ALIASES: &[DeprecatedCoreAlias] = &[
    DeprecatedCoreAlias {
        old_name: "NOW",
        replacement_qualified: "TIME@NOW",
        import_hint: "'time' IMPORT and TIME@NOW",
    },
    DeprecatedCoreAlias {
        old_name: "DATETIME",
        replacement_qualified: "TIME@DATETIME",
        import_hint: "'time' IMPORT and TIME@DATETIME",
    },
    DeprecatedCoreAlias {
        old_name: "TIMESTAMP",
        replacement_qualified: "TIME@TIMESTAMP",
        import_hint: "'time' IMPORT and TIME@TIMESTAMP",
    },
    DeprecatedCoreAlias {
        old_name: "CSPRNG",
        replacement_qualified: "CRYPTO@CSPRNG",
        import_hint: "'crypto' IMPORT and CRYPTO@CSPRNG",
    },
    DeprecatedCoreAlias {
        old_name: "HASH",
        replacement_qualified: "CRYPTO@HASH",
        import_hint: "'crypto' IMPORT and CRYPTO@HASH",
    },
    DeprecatedCoreAlias {
        old_name: "SORT",
        replacement_qualified: "ALGO@SORT",
        import_hint: "'algo' IMPORT and ALGO@SORT",
    },
    DeprecatedCoreAlias {
        old_name: "SQRT",
        replacement_qualified: "MATH@SQRT",
        import_hint: "'math' IMPORT and MATH@SQRT",
    },
    DeprecatedCoreAlias {
        old_name: "SQRT_EPS",
        replacement_qualified: "MATH@SQRT-EPS",
        import_hint: "'math' IMPORT and MATH@SQRT-EPS",
    },
    DeprecatedCoreAlias {
        old_name: "INTERVAL",
        replacement_qualified: "MATH@INTERVAL",
        import_hint: "'math' IMPORT and MATH@INTERVAL",
    },
    DeprecatedCoreAlias {
        old_name: "LOWER",
        replacement_qualified: "MATH@LOWER",
        import_hint: "'math' IMPORT and MATH@LOWER",
    },
    DeprecatedCoreAlias {
        old_name: "UPPER",
        replacement_qualified: "MATH@UPPER",
        import_hint: "'math' IMPORT and MATH@UPPER",
    },
    DeprecatedCoreAlias {
        old_name: "WIDTH",
        replacement_qualified: "MATH@WIDTH",
        import_hint: "'math' IMPORT and MATH@WIDTH",
    },
    DeprecatedCoreAlias {
        old_name: "IS_EXACT",
        replacement_qualified: "MATH@IS-EXACT",
        import_hint: "'math' IMPORT and MATH@IS-EXACT",
    },
];

pub(crate) fn lookup_deprecated_core_alias(name: &str) -> Option<&'static DeprecatedCoreAlias> {
    let upper = name.to_uppercase();
    DEPRECATED_CORE_ALIASES.iter().find(|alias| alias.old_name == upper)
}
