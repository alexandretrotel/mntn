use anyhow::Result;

pub(crate) trait Command {
    fn name(&self) -> &str;
    fn execute(&mut self) -> Result<()>;
}

pub(crate) struct CommandExecutor;

impl CommandExecutor {
    pub(crate) fn run<T: Command>(task: &mut T) {
        let name = task.name().to_string();

        println!("Starting {}", name);
        if let Err(e) = task.execute() {
            eprintln!("Error during {}: {}", name, e);
            return;
        }
        println!("{} complete", name);
    }
}
