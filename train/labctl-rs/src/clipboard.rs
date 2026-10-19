// clipboard.rs — OSC 52 copy (reaches the player's local clipboard through SSH)
use std::fs;
use std::io::{self, Write};

// Minimal base64 encoder for OSC 52 payloads
pub fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn osc52_copy(text: &str, file: &str) {
    let _ = fs::write(file, text);
    let mut stdout = io::stdout();
    let _ = write!(stdout, "\x1b]52;c;{}\x07", b64(text.as_bytes()));
    let _ = stdout.flush();
}
