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
];

pub(crate) fn lookup_deprecated_core_alias(name: &str) -> Option<&'static DeprecatedCoreAlias> {
    let upper = name.to_uppercase();
    DEPRECATED_CORE_ALIASES.iter().find(|alias| alias.old_name == upper)
}
