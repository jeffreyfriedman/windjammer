//! I/O utilities
//!
//! Windjammer's io module provides buffered reading and writing capabilities
//! with simplified error handling.

use std::fs::File;
use std::io::{
    BufRead as StdBufRead, BufReader as StdBufReader, BufWriter as StdBufWriter, Read as StdRead,
    Write as StdWrite,
};

/// Re-export std::io types
pub use std::io::{BufRead, BufReader, BufWriter, Error, Read, Result, Write};

/// Check if stdout is connected to a terminal (tty)
pub fn is_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

/// Create a buffered reader from a file
pub fn buf_reader(file: File) -> StdBufReader<File> {
    StdBufReader::new(file)
}

/// Create a buffered writer from a file
pub fn buf_writer(file: File) -> StdBufWriter<File> {
    StdBufWriter::new(file)
}

/// Read all lines from a buffered reader
pub fn read_lines<R: StdBufRead>(reader: R) -> Vec<String> {
    reader.lines().map_while(Result::ok).collect()
}

/// Read all bytes from a reader
pub fn read_all<R: StdRead>(mut reader: R) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;
    Ok(buffer)
}

/// Read all text from a reader
pub fn read_to_string<R: StdRead>(mut reader: R) -> Result<String> {
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;
    Ok(buffer)
}

/// Write all bytes to a writer
pub fn write_all<W: StdWrite>(mut writer: W, data: &[u8]) -> Result<()> {
    writer.write_all(data)
}

/// Write a string to a writer
pub fn write_str<W: StdWrite>(mut writer: W, s: &str) -> Result<()> {
    writer.write_all(s.as_bytes())
}

/// Flush a writer
pub fn flush<W: StdWrite>(mut writer: W) -> Result<()> {
    writer.flush()
}

/// stdin handle
pub fn stdin() -> std::io::Stdin {
    std::io::stdin()
}

/// stdout handle
pub fn stdout() -> std::io::Stdout {
    std::io::stdout()
}

/// stderr handle
pub fn stderr() -> std::io::Stderr {
    std::io::stderr()
}

/// Read a line from stdin
pub fn read_line() -> Result<String> {
    let mut buffer = String::new();
    stdin().read_line(&mut buffer)?;
    Ok(buffer.trim_end().to_string())
}

/// Print to stdout
pub fn print(s: &str) {
    print!("{}", s);
}

/// Print to stdout with newline
pub fn println(s: &str) {
    println!("{}", s);
}

/// Print to stderr
pub fn eprint(s: &str) {
    eprint!("{}", s);
}

/// Print to stderr with newline
pub fn eprintln(s: &str) {
    eprintln!("{}", s);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_lines() {
        let data = b"line1\nline2\nline3";
        let reader = std::io::Cursor::new(data);
        let lines = read_lines(StdBufReader::new(reader));

        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "line1");
        assert_eq!(lines[1], "line2");
        assert_eq!(lines[2], "line3");
    }

    #[test]
    fn test_read_all() {
        let data = b"hello world";
        let reader = std::io::Cursor::new(data);
        let result = read_all(reader).unwrap();

        assert_eq!(result, data);
    }

    #[test]
    fn test_write_operations() {
        let mut buffer = Vec::new();
        write_str(&mut buffer, "hello").unwrap();
        assert_eq!(buffer, b"hello");

        write_all(&mut buffer, b" world").unwrap();
        assert_eq!(buffer, b"hello world");
    }
}
