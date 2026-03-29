#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn test_hash_rejects_stack_mode() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'hello' .. HASH").await;
        assert!(result.is_err(), "HASH should reject Stack mode");
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("HASH") && err_msg.contains("Stack mode"),
            "Expected Stack mode error for HASH, got: {}",
            err_msg
        );
    }

    #[tokio::test]
    async fn test_hash_string() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'hello' HASH").await;
        assert!(result.is_ok(), "HASH should succeed: {:?}", result);
        assert_eq!(interp.stack.len(), 1);

        let val = &interp.stack[0];
        assert!(
            matches!(&val.data, ValueData::Vector(_)),
            "Hash result should be a vector"
        );
    }

    #[tokio::test]
    async fn test_hash_deterministic() {
        let mut interp = Interpreter::new();

        interp.execute("'hello' HASH").await.unwrap();
        let hash1 = interp.stack.pop().unwrap();

        interp.execute("'hello' HASH").await.unwrap();
        let hash2 = interp.stack.pop().unwrap();

        assert_eq!(
            hash1.data, hash2.data,
            "Same input should produce same hash"
        );
    }

    #[tokio::test]
    async fn test_hash_different_inputs() {
        let mut interp = Interpreter::new();

        interp.execute("'hello' HASH").await.unwrap();
        let hash1 = interp.stack.pop().unwrap();

        interp.execute("'world' HASH").await.unwrap();
        let hash2 = interp.stack.pop().unwrap();

        assert_ne!(
            hash1.data, hash2.data,
            "Different inputs should produce different hashes"
        );
    }

    #[tokio::test]
    async fn test_hash_vector() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ 1 2 3 ] HASH").await;
        assert!(
            result.is_ok(),
            "HASH on vector should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_fraction_normalization() {
        let mut interp = Interpreter::new();

        interp.execute("[ 1/2 ] HASH").await.unwrap();
        let hash1 = interp.stack.pop().unwrap();

        interp.execute("[ 2/4 ] HASH").await.unwrap();
        let hash2 = interp.stack.pop().unwrap();

        assert_eq!(
            hash1.data, hash2.data,
            "Equivalent fractions should produce same hash (1/2 = 2/4)"
        );
    }

    #[tokio::test]
    async fn test_hash_with_bit_specification() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ 128 ] 'hello' HASH").await;
        assert!(
            result.is_ok(),
            "HASH with bit spec should succeed: {:?}",
            result
        );

        let val = &interp.stack[0];
        assert!(
            matches!(&val.data, ValueData::Vector(_)),
            "Hash result should be a vector"
        );
    }

    #[tokio::test]
    async fn test_hash_boolean() {
        let mut interp = Interpreter::new();

        interp.execute("[ TRUE ] HASH").await.unwrap();
        let hash_true = interp.stack.pop().unwrap();

        interp.execute("[ FALSE ] HASH").await.unwrap();
        let hash_false = interp.stack.pop().unwrap();

        assert_ne!(
            hash_true.data, hash_false.data,
            "TRUE and FALSE should have different hashes"
        );
    }

    #[tokio::test]
    async fn test_hash_nested_vector() {
        let mut interp = Interpreter::new();
        let result = interp.execute("[ [ 1 2 ] [ 3 4 ] ] HASH").await;
        assert!(
            result.is_ok(),
            "HASH on nested vector should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_empty_string() {
        let mut interp = Interpreter::new();
        let result = interp.execute("'' HASH").await;
        assert!(
            result.is_ok(),
            "HASH on empty string should succeed: {:?}",
            result
        );
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_nil() {
        let mut interp = Interpreter::new();
        let result = interp.execute("NIL HASH").await;
        assert!(result.is_ok(), "NIL should be hashable: {:?}", result);
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_preserves_stack() {
        let mut interp = Interpreter::new();
        interp.execute("[ 1/2 ] 'hello' HASH").await.unwrap();

        assert_eq!(interp.stack.len(), 2);
    }

    #[tokio::test]
    async fn test_hash_bits_consumed() {
        let mut interp = Interpreter::new();
        interp.execute("[ 128 ] 'hello' HASH").await.unwrap();

        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_keep_mode_preserves_operand() {
        let mut interp = Interpreter::new();
        interp.execute("'hello' ,, HASH").await.unwrap();
        assert_eq!(interp.stack.len(), 2);
    }

    #[tokio::test]
    async fn test_hash_scalar_bits_supported() {
        let mut interp = Interpreter::new();
        interp.execute("128 'hello' HASH").await.unwrap();
        assert_eq!(interp.stack.len(), 1);
    }

    #[tokio::test]
    async fn test_hash_invalid_bits() {
        let mut interp = Interpreter::new();

        let result = interp.execute("[ 16 ] 'hello' HASH").await;
        assert!(result.is_err(), "Bits < 32 should error");

        let result = interp.execute("[ 2048 ] 'hello' HASH").await;
        assert!(result.is_err(), "Bits > 1024 should error");
    }

    #[tokio::test]
    async fn test_hash_distribution() {
        let mut interp = Interpreter::new();

        let inputs = ["a", "b", "c", "aa", "ab", "abc"];
        let mut hashes = Vec::new();

        for input in inputs {
            interp.execute(&format!("'{}' HASH", input)).await.unwrap();
            hashes.push(interp.stack.pop().unwrap());
        }

        for i in 0..hashes.len() {
            for j in (i + 1)..hashes.len() {
                assert_ne!(
                    hashes[i].data, hashes[j].data,
                    "Different inputs should have different hashes"
                );
            }
        }
    }
}
