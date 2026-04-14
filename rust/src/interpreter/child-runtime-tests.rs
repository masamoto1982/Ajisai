#[cfg(test)]
mod tests {
    use crate::interpreter::Interpreter;
    use crate::types::ValueData;

    #[tokio::test]
    async fn spawn_and_await_ok() {
        let mut interp = Interpreter::new();
        let result = interp.execute("{ [ 1 ] [ 2 ] + } SPAWN AWAIT").await;
        assert!(result.is_ok());
        let top = interp.stack.last().expect("expected await result");
        let ValueData::Vector(values) = &top.data else {
            panic!("await result should be vector");
        };
        assert_eq!(values[0].to_string(), "'completed'");
    }

    #[tokio::test]
    async fn child_failure_does_not_crash_parent() {
        let mut interp = Interpreter::new();
        let result = interp.execute("{ [ 1 ] [ 0 ] / } SPAWN AWAIT [ 5 ]").await;
        assert!(result.is_ok());
        assert!(!interp.stack.is_empty());
    }

    #[tokio::test]
    async fn status_and_kill_work() {
        let mut interp = Interpreter::new();
        let result = interp.execute("{ [ 1 ] } SPAWN STATUS").await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.last().unwrap().to_string(), "'running'");

        let result = interp.execute("{ [ 1 ] } SPAWN KILL").await;
        assert!(result.is_ok());
        assert_eq!(interp.stack.last().unwrap().to_string(), "'killed'");
    }

    #[tokio::test]
    async fn monitor_registration() {
        let mut interp = Interpreter::new();
        let result = interp.execute("{ [ 1 ] [ 0 ] / } SPAWN MONITOR AWAIT").await;
        assert!(result.is_ok());
        assert_eq!(interp.monitor_notifications.len(), 1);
    }

    #[tokio::test]
    async fn supervise_restarts_and_fails() {
        let mut interp = Interpreter::new();
        let result = interp.execute("{ [ 1 ] [ 0 ] / } [ 1 ] SUPERVISE").await;
        assert!(result.is_ok());
        let top = interp.stack.last().unwrap();
        let ValueData::Vector(values) = &top.data else {
            panic!("supervise result should be vector");
        };
        assert!(!values.is_empty());
    }
}
