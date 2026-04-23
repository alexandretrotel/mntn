use crate::cli::SecretActions;
use crate::commands::core::{Command, CommandExecutor};
use crate::encryption::persist_encryption_password;
use anyhow::Result;

struct SecretSetTask;

impl Command for SecretSetTask {
    fn name(&self) -> &str {
        "Secret set"
    }

    fn execute(&mut self) -> Result<()> {
        persist_encryption_password()
    }
}

pub(crate) fn run(action: SecretActions) {
    match action {
        SecretActions::Set => CommandExecutor::run(&mut SecretSetTask),
    }
}
