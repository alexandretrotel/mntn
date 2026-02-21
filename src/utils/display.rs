use std::path::Path;

pub(crate) fn short_component(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

const COLOR_GREEN: &str = "\x1b[32m";
const COLOR_YELLOW: &str = "\x1b[33m";
const COLOR_RED: &str = "\x1b[31m";
const COLOR_RESET: &str = "\x1b[0m";

fn color(text: &str, code: &str) -> String {
    format!("{code}{text}{COLOR_RESET}")
}

pub(crate) fn green(text: &str) -> String {
    color(text, COLOR_GREEN)
}

pub(crate) fn yellow(text: &str) -> String {
    color(text, COLOR_YELLOW)
}

pub(crate) fn red(text: &str) -> String {
    color(text, COLOR_RED)
}
