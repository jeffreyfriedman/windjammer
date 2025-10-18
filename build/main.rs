


use std::http::*;

use std::json::*;


#[derive(Serialize, Deserialize)]
struct User {
    id: i64,
    name: String,
    email: String,
}

#[derive(Serialize, Deserialize)]
struct ApiResponse {
    message: String,
    status: String,
}

#[tokio::main]
async fn main() {
    println!("Starting web application...");
    println!("This is a template for building web services with Windjammer");
    println!("");
    println!("Example: Making an HTTP request");
    match http.get("https://jsonplaceholder.typicode.com/users/1").await {
        Ok(response) => {
            println!("Status: {}", response.status_code());
            match response.text().await {
                Ok(body) => println!("Response: {}", body),
                Err(e) => println!("Error reading response: {:?}", e),
            }
        },
        Err(e) => println!("Error making request: {:?}", e),
    }
}

