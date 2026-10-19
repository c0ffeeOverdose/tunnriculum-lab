// main.rs — entry point: CLI dispatch, selftest, TUI
mod app;
mod clipboard;
mod guide;
mod lab;
mod ui;

use std::env;
use std::process::exit;

use clipboard::b64;
use lab::handle_action;
use ui::tui;

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut cmd_args: Vec<String> = args.iter().skip(1).cloned().collect();
    if cmd_args.get(0).map(|s| s.as_str()) == Some("-c") {
        if let Some(cmd_str) = cmd_args.get(1) {
            let parts: Vec<String> = cmd_str.split_whitespace().map(|s| s.to_string()).collect();
            cmd_args = parts;
        } else { eprintln!("unknown action: -c"); exit(1); }
    }
    if !cmd_args.is_empty() {
        let action = cmd_args[0].clone();
        let rest = cmd_args[1..].to_vec();
        let code = handle_action(&action, &rest);
        exit(code);
    }
    if env::var("LABCTL_SELFTEST").is_ok() {
        assert_eq!(b64(b""), "");
        assert_eq!(b64(b"hi"), "aGk=");
        assert_eq!(b64(b"hello"), "aGVsbG8=");
        assert_eq!(b64(b"a"), "YQ==");
        println!("selftest ok");
        exit(0);
    }
    let is_tty = unsafe { libc::isatty(1) == 1 };
    if !is_tty {
        println!("Tunnriculum lab console (non-interactive)");
        println!("Usage: labctl <action> [endpoint]");
        println!("Actions: start stop restart reset status logs smoke peer");
        exit(0);
    }
    if let Err(e) = tui() {
        eprintln!("TUI error: {}", e);
        exit(1);
    }
}

mod libc { extern "C" { pub fn isatty(fd: i32) -> i32; } }
