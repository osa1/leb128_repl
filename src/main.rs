use leb128;

use std::io::{self, Write};
use std::str;

fn display(bytes: &[u8]) -> String {
    let mut s = vec![];

    // Base 10.
    write!(&mut s, "{:?}\n", bytes).unwrap();

    // Hex.
    write!(&mut s, "[").unwrap();
    for (i, b) in bytes.iter().enumerate() {
        if i != 0 {
            write!(&mut s, ", ").unwrap();
        }
        write!(&mut s, "{:0x}", b).unwrap();
    }
    writeln!(&mut s, "]").unwrap();

    // Binary.
    write!(&mut s, "[").unwrap();
    for (i, b) in bytes.iter().enumerate() {
        if i != 0 {
            write!(&mut s, ", ").unwrap();
        }
        write!(&mut s, "{:08b}", b).unwrap();
    }
    writeln!(&mut s, "]").unwrap();

    String::from_utf8(s).unwrap()
}

fn main() {
    println!(
        "
LEB128 Read-Eval-Print-Loop!

Converts numbers to signed and unsigned LEB128 and displays the results in
base-10, hex, and binary.
"
    );

    let mut stdout = io::stdout();

    let mut rl = rustyline::Editor::<()>::with_config(
        rustyline::config::Builder::new()
            .max_history_size(usize::MAX)
            .auto_add_history(true)
            .build(),
    );

    loop {
        let readline = rl.readline("> ");
        let line = match readline {
            Err(_) => break,
            Ok(line) => line,
        };

        let buf = line.as_bytes();

        let uleb = str::from_utf8(&buf)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .and_then(|n: u64| {
                let mut s = vec![];
                leb128::write::unsigned(&mut s, n).ok()?;
                Some(display(&s))
            })
            .unwrap_or_else(|| "error\n".into());
        stdout
            .write_all(b"# unsigned LEB128\n")
            .and_then(|_| stdout.write_all(uleb.as_bytes()))
            .and_then(|_| stdout.write_all(b"\n"))
            .expect("failed to write to stdout");

        let leb = str::from_utf8(&buf)
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .and_then(|n: i64| {
                let mut s = vec![];
                leb128::write::signed(&mut s, n).ok()?;
                Some(display(&s))
            })
            .unwrap_or_else(|| "error\n".into());
        stdout
            .write_all(b"# signed LEB128\n")
            .and_then(|_| stdout.write_all(leb.as_bytes()))
            .and_then(|_| stdout.write_all(b"\n"))
            .expect("failed to write to stdout");

        stdout.flush().expect("failed to flush stdout");
    }
}
