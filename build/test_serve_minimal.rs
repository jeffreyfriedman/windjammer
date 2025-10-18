



use std::http::*;

use std::fs::*;

use std::mime::*;


fn main() {
    let path = "test";
    match fs::read_to_string(path) {
        Ok(content) => {
            let mime_type = mime::from_path(path);
            println!("OK: {}", mime_type);
        },
        Err(e) => {
            println!("Error");
        },
    }
}

