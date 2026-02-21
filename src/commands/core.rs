use anyhow::Result;

pub trait Command {
    fn name(&self) -> &str;
    fn execute(&mut self) -> Result<()>;
}

pub struct CommandExecutor;

impl CommandExecutor {
    pub fn run<T: Command>(task: &mut T) {
        let name = task.name().to_string();

        println!("Starting {}", name);
        if let Err(e) = task.execute() {
            eprintln!("Error during {}: {}", name, e);
            return;
        }
        println!("{} complete", name);
    }
}
