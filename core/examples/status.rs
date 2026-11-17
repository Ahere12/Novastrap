//! Prints the Roblox status for this machine as JSON: `cargo run --example status`.
#[tokio::main]
async fn main() {
    let client = novastrap_core::roblox::deployment::http_client().unwrap();
    let status = novastrap_core::roblox::status(&client).await;
    println!("{}", serde_json::to_string_pretty(&status).unwrap());
}
