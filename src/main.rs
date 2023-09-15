#![allow(clippy::from_str_radix_10)]

use std::io;
use std::process::exit;

fn main() {
    let mut args = std::env::args().skip(1);

    let cmd = match args.next() {
        Some(str) => match str.as_str() {
            "es" => Cmd::EncodeSigned,
            "eu" => Cmd::EncodeUnsigned,
            "ds" => Cmd::DecodeSigned,
            "du" => Cmd::DecodeUnsigned,
            other => {
                eprintln!("Unrecognized command: {}", other);
                show_usage();
                exit(1);
            }
        },
        None => {
            show_usage();
            return;
        }
    };

    let arg = match args.next() {
        Some(arg) => arg,
        None => {
            eprintln!("Argument to command {} is missing.", cmd_str(cmd));
            show_usage();
            exit(1);
        }
    };

    if let Some(arg) = args.next() {
        eprintln!("Extra argument passed: {}", arg);
        show_usage();
        exit(1);
    }

    match cmd {
        Cmd::EncodeSigned => encode_signed(&arg),
        Cmd::EncodeUnsigned => encode_unsigned(&arg),
        Cmd::DecodeSigned => decode_signed(&arg),
        Cmd::DecodeUnsigned => decode_unsigned(&arg),
    }
}

#[derive(Debug, Clone, Copy)]
enum Cmd {
    EncodeSigned,
    EncodeUnsigned,
    DecodeSigned,
    DecodeUnsigned,
}

fn cmd_str(cmd: Cmd) -> &'static str {
    match cmd {
        Cmd::EncodeSigned => "es",
        Cmd::EncodeUnsigned => "eu",
        Cmd::DecodeSigned => "ds",
        Cmd::DecodeUnsigned => "du",
    }
}

fn show_usage() {
    // TODO
}

fn encode_signed(mut s: &str) {
    let negative = if s.starts_with('-') {
        s = &s[1..];
        true
    } else {
        false
    };

    let mut i = if s.starts_with("0x") || s.starts_with("0X") {
        i128::from_str_radix(&s[2..], 16).unwrap()
    } else {
        i128::from_str_radix(s, 10).unwrap()
    };

    if negative {
        i = -i;
    }

    let mut buf = [0u8; 1024];
    let mut writable = &mut buf[..];
    let written = write::signed(&mut writable, i).unwrap();
    for byte in &buf[0..written] {
        print!("{:02x}", byte);
    }
    println!();
}

fn encode_unsigned(s: &str) {
    let i = if s.starts_with("0x") || s.starts_with("0X") {
        u128::from_str_radix(&s[2..], 16).unwrap()
    } else {
        u128::from_str_radix(s, 10).unwrap()
    };

    let mut buf = [0u8; 1024];
    let mut writable = &mut buf[..];
    let written = write::unsigned(&mut writable, i).unwrap();
    for byte in &buf[0..written] {
        print!("{:02x}", byte);
    }
    println!();
}

fn decode_signed(s: &str) {
    let bytes: Vec<u8> = s
        .trim()
        .split(' ')
        .map(|byte_str| {
            u8::from_str_radix(byte_str, 16)
                .unwrap_or_else(|_| panic!("Invalid hex byte: {:?}", byte_str))
        })
        .collect();
    let mut readable = &bytes[..];
    let i = read::signed(&mut readable).unwrap();
    println!("{}", i);
    print_rest(&bytes, readable);
}

fn decode_unsigned(s: &str) {
    let bytes: Vec<u8> = s
        .trim()
        .split(' ')
        .map(|byte_str| {
            u8::from_str_radix(byte_str, 16)
                .unwrap_or_else(|_| panic!("Invalid hex byte: {:?}", byte_str))
        })
        .collect();
    let mut readable = &bytes[..];
    let i = read::unsigned(&mut readable).unwrap();
    println!("{}", i);
    print_rest(&bytes, readable);
}

fn print_rest(input_bytes: &[u8], unread_bytes: &[u8]) {
    let n_read = input_bytes.len() - unread_bytes.len();
    println!("{} bytes read. Unread bytes:", n_read);
    for byte in unread_bytes {
        print!("{:02x}", byte);
    }
    println!();
}

////////////////////////////////////////////////////////////////////////////////////////////////////

const CONTINUATION_BIT: u8 = 1 << 7;

const SIGN_BIT: u8 = 1 << 6;

fn low_bits_of_byte(byte: u8) -> u8 {
    byte & !CONTINUATION_BIT
}

fn low_bits_of_u128(val: u128) -> u8 {
    let byte = val & (std::u8::MAX as u128);
    low_bits_of_byte(byte as u8)
}

/// A module for reading LEB128-encoded signed and unsigned integers.
mod read {
    use super::*;

    use std::fmt;

    /// An error type for reading LEB128-encoded values.
    #[derive(Debug)]
    pub enum Error {
        /// There was an underlying IO error.
        IoError(io::Error),

        /// The number being read is larger than can be represented.
        Overflow,
    }

    impl From<io::Error> for Error {
        fn from(e: io::Error) -> Self {
            Error::IoError(e)
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
            match *self {
                Error::IoError(ref e) => e.fmt(f),
                Error::Overflow => {
                    write!(f, "The number being read is larger than can be represented")
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match *self {
                Error::IoError(ref e) => Some(e),
                Error::Overflow => None,
            }
        }
    }

    pub fn unsigned<R>(r: &mut R) -> Result<u128, Error>
    where
        R: ?Sized + io::Read,
    {
        let mut result = 0;
        let mut shift = 0;

        loop {
            let mut buf = [0];
            r.read_exact(&mut buf)?;

            if shift == 126 && buf[0] != 0x00 && buf[0] != 0x01 {
                while buf[0] & CONTINUATION_BIT != 0 {
                    r.read_exact(&mut buf)?;
                }
                return Err(Error::Overflow);
            }

            let low_bits = low_bits_of_byte(buf[0]) as u128;
            result |= low_bits << shift;

            if buf[0] & CONTINUATION_BIT == 0 {
                return Ok(result);
            }

            shift += 7;
        }
    }

    pub fn signed<R>(r: &mut R) -> Result<i128, Error>
    where
        R: ?Sized + io::Read,
    {
        let mut result = 0;
        let mut shift = 0;
        let size = 128;
        let mut byte;

        loop {
            let mut buf = [0];
            r.read_exact(&mut buf)?;

            byte = buf[0];
            if shift == 126 && byte != 0x00 && byte != 0x7f {
                while buf[0] & CONTINUATION_BIT != 0 {
                    r.read_exact(&mut buf)?;
                }
                return Err(Error::Overflow);
            }

            let low_bits = low_bits_of_byte(byte) as i128;
            result |= low_bits << shift;
            shift += 7;

            if byte & CONTINUATION_BIT == 0 {
                break;
            }
        }

        if shift < size && (SIGN_BIT & byte) == SIGN_BIT {
            // Sign extend the result.
            result |= !0 << shift;
        }

        Ok(result)
    }
}

mod write {
    use super::*;

    pub fn unsigned<W>(w: &mut W, mut val: u128) -> Result<usize, io::Error>
    where
        W: ?Sized + io::Write,
    {
        let mut bytes_written = 0;
        loop {
            let mut byte = low_bits_of_u128(val);
            val >>= 7;
            if val != 0 {
                // More bytes to come, so set the continuation bit.
                byte |= CONTINUATION_BIT;
            }

            let buf = [byte];
            w.write_all(&buf)?;
            bytes_written += 1;

            if val == 0 {
                return Ok(bytes_written);
            }
        }
    }

    pub fn signed<W>(w: &mut W, mut val: i128) -> Result<usize, io::Error>
    where
        W: ?Sized + io::Write,
    {
        let mut bytes_written = 0;
        loop {
            let mut byte = val as u8;
            // Keep the sign bit for testing.
            val >>= 6;
            let done = val == 0 || val == -1;
            if done {
                byte &= !CONTINUATION_BIT;
            } else {
                // Remove the sign bit.
                val >>= 1;
                // More bytes to come, so set the continuation bit.
                byte |= CONTINUATION_BIT;
            }

            let buf = [byte];
            w.write_all(&buf)?;
            bytes_written += 1;

            if done {
                return Ok(bytes_written);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std;

    #[test]
    fn test_low_bits_of_byte() {
        for i in 0..127 {
            assert_eq!(i, low_bits_of_byte(i));
            assert_eq!(i, low_bits_of_byte(i | CONTINUATION_BIT));
        }
    }

    #[test]
    fn test_low_bits_of_u128() {
        for i in 0u128..127 {
            assert_eq!(i as u8, low_bits_of_u128(1 << 16 | i));
            assert_eq!(
                i as u8,
                low_bits_of_u128(i << 16 | i | (CONTINUATION_BIT as u128))
            );
        }
    }

    #[test]
    fn test_read_unsigned() {
        let buf = [2u8];
        let mut readable = &buf[..];
        assert_eq!(
            2,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [127u8];
        let mut readable = &buf[..];
        assert_eq!(
            127,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            128,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [1u8 | CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            129,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [2u8 | CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            130,
            read::unsigned(&mut readable).expect("Should read number")
        );

        let buf = [57u8 | CONTINUATION_BIT, 100];
        let mut readable = &buf[..];
        assert_eq!(
            12857,
            read::unsigned(&mut readable).expect("Should read number")
        );
    }

    #[test]
    fn test_read_unsigned_thru_dyn_trait() {
        fn read(r: &mut dyn io::Read) -> u128 {
            read::unsigned(r).expect("Should read number")
        }

        let buf = [0u8];

        let mut readable = &buf[..];
        assert_eq!(0, read(&mut readable));

        let mut readable = io::Cursor::new(buf);
        assert_eq!(0, read(&mut readable));
    }

    // Examples from the DWARF 4 standard, section 7.6, figure 23.
    #[test]
    fn test_read_signed() {
        let buf = [2u8];
        let mut readable = &buf[..];
        assert_eq!(2, read::signed(&mut readable).expect("Should read number"));

        let buf = [0x7eu8];
        let mut readable = &buf[..];
        assert_eq!(-2, read::signed(&mut readable).expect("Should read number"));

        let buf = [127u8 | CONTINUATION_BIT, 0];
        let mut readable = &buf[..];
        assert_eq!(
            127,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [1u8 | CONTINUATION_BIT, 0x7f];
        let mut readable = &buf[..];
        assert_eq!(
            -127,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            128,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [CONTINUATION_BIT, 0x7f];
        let mut readable = &buf[..];
        assert_eq!(
            -128,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [1u8 | CONTINUATION_BIT, 1];
        let mut readable = &buf[..];
        assert_eq!(
            129,
            read::signed(&mut readable).expect("Should read number")
        );

        let buf = [0x7fu8 | CONTINUATION_BIT, 0x7e];
        let mut readable = &buf[..];
        assert_eq!(
            -129,
            read::signed(&mut readable).expect("Should read number")
        );
    }

    #[test]
    fn test_read_signed_thru_dyn_trait() {
        fn read(r: &mut dyn io::Read) -> i128 {
            read::signed(r).expect("Should read number")
        }

        let buf = [0u8];

        let mut readable = &buf[..];
        assert_eq!(0, read(&mut readable));

        let mut readable = io::Cursor::new(buf);
        assert_eq!(0, read(&mut readable));
    }

    #[test]
    fn test_read_signed_127_bits() {
        let buf = [
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            CONTINUATION_BIT,
            0x40,
        ];
        let mut readable = &buf[..];
        assert_eq!(
            -0x400000000000000000000000000000,
            read::signed(&mut readable).expect("Should read number")
        );
    }

    #[test]
    fn test_read_unsigned_not_enough_data() {
        let buf = [CONTINUATION_BIT];
        let mut readable = &buf[..];
        match read::unsigned(&mut readable) {
            Err(read::Error::IoError(e)) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn test_read_signed_not_enough_data() {
        let buf = [CONTINUATION_BIT];
        let mut readable = &buf[..];
        match read::signed(&mut readable) {
            Err(read::Error::IoError(e)) => assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn test_write_unsigned_not_enough_space() {
        let mut buf = [0; 1];
        let mut writable = &mut buf[..];
        match write::unsigned(&mut writable, 128) {
            Err(e) => assert_eq!(e.kind(), io::ErrorKind::WriteZero),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn test_write_signed_not_enough_space() {
        let mut buf = [0; 1];
        let mut writable = &mut buf[..];
        match write::signed(&mut writable, 128) {
            Err(e) => assert_eq!(e.kind(), io::ErrorKind::WriteZero),
            otherwise => panic!("Unexpected: {:?}", otherwise),
        }
    }

    #[test]
    fn test_write_unsigned_thru_dyn_trait() {
        fn write(w: &mut dyn io::Write, val: u128) -> usize {
            write::unsigned(w, val).expect("Should write number")
        }
        let mut buf = [0u8; 1];

        let mut writable = &mut buf[..];
        assert_eq!(write(&mut writable, 0), 1);
        assert_eq!(buf[0], 0);

        let mut writable = Vec::from(&buf[..]);
        assert_eq!(write(&mut writable, 0), 1);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn test_write_signed_thru_dyn_trait() {
        fn write(w: &mut dyn io::Write, val: i128) -> usize {
            write::signed(w, val).expect("Should write number")
        }
        let mut buf = [0u8; 1];

        let mut writable = &mut buf[..];
        assert_eq!(write(&mut writable, 0), 1);
        assert_eq!(buf[0], 0);

        let mut writable = Vec::from(&buf[..]);
        assert_eq!(write(&mut writable, 0), 1);
        assert_eq!(buf[0], 0);
    }

    #[test]
    fn dogfood_signed() {
        fn inner(i: i128) {
            let mut buf = [0u8; 1024];

            let mut writable = &mut buf[..];
            write::signed(&mut writable, i).expect("Should write signed number");

            let mut readable = &buf[..];
            let result = read::signed(&mut readable).expect("Should be able to read it back again");
            assert_eq!(i, result);
        }
        for i in -513..513 {
            inner(i);
        }
        inner(std::i64::MIN as i128);
    }

    #[test]
    fn dogfood_unsigned() {
        for i in 0..1025 {
            let mut buf = [0u8; 1024];

            {
                let mut writable = &mut buf[..];
                write::unsigned(&mut writable, i).expect("Should write signed number");
            }

            let mut readable = &buf[..];
            let result =
                read::unsigned(&mut readable).expect("Should be able to read it back again");
            assert_eq!(i, result);
        }
    }

    #[test]
    fn test_read_unsigned_overflow() {
        let buf = [
            2u8 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            1,
        ];
        let mut readable = &buf[..];
        assert!(read::unsigned(&mut readable).is_err());
    }

    #[test]
    fn test_read_signed_overflow() {
        let buf = [
            2u8 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            2 | CONTINUATION_BIT,
            1,
        ];
        let mut readable = &buf[..];
        assert!(read::signed(&mut readable).is_err());
    }

    #[test]
    fn test_read_multiple() {
        let buf = [2u8 | CONTINUATION_BIT, 1u8, 1u8];

        let mut readable = &buf[..];
        assert_eq!(
            read::unsigned(&mut readable).expect("Should read first number"),
            130u128
        );
        assert_eq!(
            read::unsigned(&mut readable).expect("Should read first number"),
            1u128
        );
    }

    #[test]
    fn test_read_multiple_with_overflow() {
        let buf = [
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b1111_1111,
            0b0111_1111, // Overflow!
            0b1110_0100,
            0b1110_0000,
            0b0000_0010, // 45156
        ];
        let mut readable = &buf[..];

        assert!(if let read::Error::Overflow =
            read::unsigned(&mut readable).expect_err("Should fail with Error::Overflow")
        {
            true
        } else {
            false
        });
        assert_eq!(
            read::unsigned(&mut readable).expect("Should succeed with correct value"),
            45156
        );
    }
}
