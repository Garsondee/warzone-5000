//! `w5k world`: the command line of lane WORLD (only that lane edits this file).

/// Entry point for `w5k world <args>`.
pub fn run(args: &[String]) -> Result<(), String> {
    let _ = args;
    Err("lane WORLD has no commands yet".to_string())
}
