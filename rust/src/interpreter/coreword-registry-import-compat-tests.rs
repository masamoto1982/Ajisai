#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;

    #[tokio::test]
    async fn import_and_import_only_remain_compatible_for_standard_modules() {
        let mut interp = Interpreter::new();

        for module_name in ["MATH", "JSON", "IO", "TIME", "CRYPTO", "ALGO", "MUSIC"] {
            let code = format!("'{}' IMPORT", module_name);
            interp
                .execute(&code)
                .await
                .unwrap_or_else(|e| panic!("IMPORT failed for {}: {}", module_name, e));
            assert!(
                interp.import_table.modules.contains_key(module_name),
                "module should be present in import table: {}",
                module_name
            );
        }

        interp
            .execute("'MATH' [ 'SQRT' ] IMPORT-ONLY")
            .await
            .expect("IMPORT-ONLY for MATH@SQRT should succeed");
    }
}
