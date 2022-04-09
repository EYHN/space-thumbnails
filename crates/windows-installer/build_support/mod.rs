mod download;
mod unzip;
use std::{process::Command, io::ErrorKind};

pub use download::*;
pub use unzip::*;

pub fn run_command(cmd: &mut Command, program: &str) {
    println!(
        "current_dir: {:?}\nrunning: {:?}",
        cmd.get_current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or("".to_string()),
        cmd
    );
    let status = match cmd.status() {
        Ok(status) => status,
        Err(ref e) if e.kind() == ErrorKind::NotFound => {
            panic!(
                "{}",
                &format!(
                    "failed to execute command: {}\nis `{}` not installed?",
                    e, program
                )
            );
        }
        Err(e) => panic!("{}", &format!("failed to execute command: {:?}", e)),
    };
    if !status.success() {
        panic!(
            "{}",
            &format!("command did not execute successfully, got: {}", status)
        );
    }
}
