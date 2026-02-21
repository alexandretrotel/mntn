use crate::cli::{ProfileActions, ProfileArgs};

mod create;
mod delete;
mod list;

pub(crate) fn run(args: ProfileArgs) {
    match args.action {
        Some(ProfileActions::List) => list::list_profiles(),
        Some(ProfileActions::Create { name, description }) => {
            create::create_profile(&name, description)
        }
        Some(ProfileActions::Delete { name }) => delete::delete_profile(&name),
        None => {
            show_current_profile();
        }
    }
}

fn show_current_profile() {
    let current = crate::profiles::get_active_profile_name();
    match current {
        Some(name) => println!("Active profile: {}", name),
        None => println!("No active profile (using common only)"),
    }
    println!();
    list::list_profiles();
    println!();
    println!("Use 'mntn use <profile>' to switch profiles");
}
