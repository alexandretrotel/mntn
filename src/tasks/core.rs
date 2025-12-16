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
