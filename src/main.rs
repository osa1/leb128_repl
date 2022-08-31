use std::fmt::Write;

fn display(bytes: &[u8]) -> String {
    let mut s = String::new();

    // Base 10
    writeln!(&mut s, "{:?}", bytes).unwrap();

    // Hex
    write!(&mut s, "[").unwrap();
    for (i, b) in bytes.iter().enumerate() {
        if i != 0 {
            write!(&mut s, ", ").unwrap();
        }
        write!(&mut s, "{:0x}", b).unwrap();
    }
    writeln!(&mut s, "]").unwrap();

    // Binary
    write!(&mut s, "[").unwrap();
    for (i, b) in bytes.iter().enumerate() {
        if i != 0 {
            write!(&mut s, ", ").unwrap();
        }
        write!(&mut s, "{:08b}", b).unwrap();
    }
    writeln!(&mut s, "]").unwrap();

    s
}

fn main() {
    println!(
        "
LEB128 Read-Eval-Print-Loop!

Converts numbers to signed and unsigned LEB128 and displays the results in
base-10, hex, and binary.
"
    );

    let mut rl = rustyline::Editor::<()>::with_config(
        rustyline::config::Builder::new()
            .max_history_size(usize::MAX)
            .auto_add_history(true)
            .build(),
    );

    loop {
        let line = match rl.readline("> ") {
            Err(_) => break,
            Ok(line) => line,
        };

        let line = line.trim();

        match line.parse::<u64>() {
            Ok(u64) => {
                let mut s = vec![];
                leb128::write::unsigned(&mut s, u64).unwrap();
                println!("# unsigned LEB128");
                println!("{}", display(&s));
            }
            Err(err) => {
                println!("Input is not u64: {}", err);
            }
        }

        match line.parse::<i64>() {
            Ok(i64) => {
                let mut s = vec![];
                leb128::write::signed(&mut s, i64).unwrap();
                println!("# signed LEB128");
                println!("{}", display(&s));
            }
            Err(err) => {
                println!("Input is not i64: {}", err);
            }
        }
    }
}
