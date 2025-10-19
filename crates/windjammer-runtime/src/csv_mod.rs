//! CSV parsing and writing
//!
//! Windjammer's `std::csv` module maps to these functions.

use csv::{Reader, Writer};
use std::io::Cursor;

/// Parse CSV string into rows
pub fn parse(data: &str) -> Result<Vec<Vec<String>>, String> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false) // Treat all rows as data, not headers
        .from_reader(Cursor::new(data));
    let mut rows = Vec::new();

    for result in reader.records() {
        let record = result.map_err(|e| e.to_string())?;
        let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
        rows.push(row);
    }

    Ok(rows)
}

/// Write rows to CSV string
pub fn write(rows: &[Vec<String>]) -> Result<String, String> {
    let mut writer = Writer::from_writer(vec![]);

    for row in rows {
        writer.write_record(row).map_err(|e| e.to_string())?;
    }

    let data = writer.into_inner().map_err(|e| e.to_string())?;
    String::from_utf8(data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let csv = "a,b,c\n1,2,3\n4,5,6";
        let rows = parse(csv).unwrap();
        // CSV reader treats first row as data, not headers (unless configured)
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0], vec!["a", "b", "c"]);
        assert_eq!(rows[1], vec!["1", "2", "3"]);
        assert_eq!(rows[2], vec!["4", "5", "6"]);
    }

    #[test]
    fn test_write() {
        let rows = vec![
            vec!["a".to_string(), "b".to_string()],
            vec!["1".to_string(), "2".to_string()],
        ];
        let csv = write(&rows).unwrap();
        assert!(csv.contains("a,b"));
        assert!(csv.contains("1,2"));
    }
}
