use crate::logger::log;

/// Represents a planned operation that a task would perform
#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub description: String,
    pub target: Option<String>,
}

impl PlannedOperation {
    pub fn new(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            target: None,
        }
    }

    pub fn with_target(description: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            target: Some(target.into()),
        }
    }
}

/// Core trait for tasks that support dry-run mode
pub trait Task {
    /// Human-readable name of the task
    fn name(&self) -> &str;

    /// Execute the task
    fn execute(&mut self);

    /// Preview what the task would do (for dry-run mode)
    fn dry_run(&self) -> Vec<PlannedOperation>;
}

/// Executor that handles running tasks with logging and dry-run support
pub struct TaskExecutor;

impl TaskExecutor {
    pub fn run<T: Task>(task: &mut T, dry_run: bool) {
        let name = task.name().to_string();

        if dry_run {
            println!("[DRY RUN] {}", name);
            log(&format!("[DRY RUN] Starting {}", name));

            let operations = task.dry_run();
            if operations.is_empty() {
                println!("  No operations to perform.");
            } else {
                println!("  Planned operations:");
                for op in &operations {
                    if let Some(target) = &op.target {
                        println!("    - {} -> {}", op.description, target);
                    } else {
                        println!("    - {}", op.description);
                    }
                }
                println!("  Total: {} operation(s)", operations.len());
            }

            log(&format!("[DRY RUN] {} complete", name));
        } else {
            log(&format!("Starting {}", name));
            task.execute();
            log(&format!("{} complete", name));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planned_operation_new() {
        let op = PlannedOperation::new("Test operation");
        assert_eq!(op.description, "Test operation");
        assert!(op.target.is_none());
    }

    #[test]
    fn test_planned_operation_new_with_string() {
        let op = PlannedOperation::new(String::from("Dynamic description"));
        assert_eq!(op.description, "Dynamic description");
        assert!(op.target.is_none());
    }

    #[test]
    fn test_planned_operation_with_target() {
        let op = PlannedOperation::with_target("Copy file", "/path/to/dest");
        assert_eq!(op.description, "Copy file");
        assert_eq!(op.target, Some("/path/to/dest".to_string()));
    }

    #[test]
    fn test_planned_operation_with_target_strings() {
        let op = PlannedOperation::with_target(
            String::from("Create symlink"),
            String::from("/home/user/.config"),
        );
        assert_eq!(op.description, "Create symlink");
        assert_eq!(op.target, Some("/home/user/.config".to_string()));
    }

    #[test]
    fn test_planned_operation_clone() {
        let op = PlannedOperation::with_target("Original", "/target");
        let cloned = op.clone();

        assert_eq!(cloned.description, op.description);
        assert_eq!(cloned.target, op.target);
    }

    #[test]
    fn test_planned_operation_debug() {
        let op = PlannedOperation::new("Debug test");
        let debug_str = format!("{:?}", op);

        assert!(debug_str.contains("PlannedOperation"));
        assert!(debug_str.contains("Debug test"));
    }

    // Mock task implementation for testing
    struct MockTask {
        name: String,
        operations: Vec<PlannedOperation>,
        executed: bool,
    }

    impl MockTask {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                operations: Vec::new(),
                executed: false,
            }
        }

        fn with_operations(name: &str, ops: Vec<PlannedOperation>) -> Self {
            Self {
                name: name.to_string(),
                operations: ops,
                executed: false,
            }
        }
    }

    impl Task for MockTask {
        fn name(&self) -> &str {
            &self.name
        }

        fn execute(&mut self) {
            self.executed = true;
        }

        fn dry_run(&self) -> Vec<PlannedOperation> {
            self.operations.clone()
        }
    }

    #[test]
    fn test_task_name() {
        let task = MockTask::new("Test Task");
        assert_eq!(task.name(), "Test Task");
    }

    #[test]
    fn test_task_execute() {
        let mut task = MockTask::new("Execute Test");
        assert!(!task.executed);

        task.execute();
        assert!(task.executed);
    }

    #[test]
    fn test_task_dry_run_empty() {
        let task = MockTask::new("Empty Task");
        let ops = task.dry_run();
        assert!(ops.is_empty());
    }

    #[test]
    fn test_task_dry_run_with_operations() {
        let ops = vec![
            PlannedOperation::new("Op 1"),
            PlannedOperation::with_target("Op 2", "/target"),
        ];
        let task = MockTask::with_operations("Task with Ops", ops);

        let result = task.dry_run();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].description, "Op 1");
        assert_eq!(result[1].description, "Op 2");
    }

    #[test]
    fn test_task_executor_run_executes_task() {
        let mut task = MockTask::new("Executor Test");
        assert!(!task.executed);

        TaskExecutor::run(&mut task, false);
        assert!(task.executed);
    }

    #[test]
    fn test_task_executor_dry_run_does_not_execute() {
        let mut task = MockTask::new("Dry Run Test");
        assert!(!task.executed);

        TaskExecutor::run(&mut task, true);
        assert!(!task.executed);
    }

    #[test]
    fn test_task_executor_dry_run_with_operations() {
        let ops = vec![
            PlannedOperation::new("Operation A"),
            PlannedOperation::with_target("Operation B", "/some/path"),
        ];
        let mut task = MockTask::with_operations("Multi-Op Task", ops);

        // This should not panic and should print operations
        TaskExecutor::run(&mut task, true);
        assert!(!task.executed);
    }

    #[test]
    fn test_task_executor_dry_run_empty_operations() {
        let mut task = MockTask::new("No-Op Task");

        // This should not panic and should print "No operations to perform"
        TaskExecutor::run(&mut task, true);
        assert!(!task.executed);
    }

    struct CountingTask {
        name: String,
        execute_count: usize,
    }

    impl CountingTask {
        fn new(name: &str) -> Self {
            Self {
                name: name.to_string(),
                execute_count: 0,
            }
        }
    }

    impl Task for CountingTask {
        fn name(&self) -> &str {
            &self.name
        }

        fn execute(&mut self) {
            self.execute_count += 1;
        }

        fn dry_run(&self) -> Vec<PlannedOperation> {
            vec![PlannedOperation::new("Counting operation")]
        }
    }

    #[test]
    fn test_task_executor_multiple_executions() {
        let mut task = CountingTask::new("Counting Task");

        TaskExecutor::run(&mut task, false);
        assert_eq!(task.execute_count, 1);

        TaskExecutor::run(&mut task, false);
        assert_eq!(task.execute_count, 2);

        TaskExecutor::run(&mut task, false);
        assert_eq!(task.execute_count, 3);
    }

    #[test]
    fn test_task_executor_mixed_dry_and_real_runs() {
        let mut task = CountingTask::new("Mixed Task");

        TaskExecutor::run(&mut task, true); // dry run
        assert_eq!(task.execute_count, 0);

        TaskExecutor::run(&mut task, false); // real run
        assert_eq!(task.execute_count, 1);

        TaskExecutor::run(&mut task, true); // dry run
        assert_eq!(task.execute_count, 1);

        TaskExecutor::run(&mut task, false); // real run
        assert_eq!(task.execute_count, 2);
    }
}
