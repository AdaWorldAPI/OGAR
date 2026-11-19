#![allow(dead_code)]

pub const TUTORIAL: &str = include_str!("../fixtures/tutorial_nothing_to_do.ki");
pub const HTTP: &str = include_str!("../fixtures/http_request.ki");
pub const SSH_LINE: &str = include_str!("../fixtures/ssh_execute_command_line.ki");

/// SYNTHETIC wrapper: the third public example is one `do` statement, not a
/// complete item, so it is placed under a header written here.
pub fn ssh_wrapped() -> String {
    format!("on\n  ogit/_id\nwhen\n  Synthetic\ndo\n{SSH_LINE}")
}

/// SYNTHETIC: a complete item around the given `when` lines and `do` lines.
pub fn item(when: &str, do_: &str) -> String {
    format!("on\n  ogit/_id\nwhen\n  {when}\ndo\n  {do_}\n")
}
