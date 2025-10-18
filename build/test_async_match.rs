use std::http::*;


#[tokio::main]
async fn main() {
    match http::serve("0.0.0.0:8000", router).await {
        Ok(x) => println!("OK"),
        Err(e) => println!("Error"),
    }
}

