//! Manual check of Discord Rich Presence: `cargo run --example discord_test -- <application id>`.
//! Shows a "Novastrap test" status for 8 seconds, then clears it.

use novastrap_core::discord::{Presence, Rpc};

fn main() {
    let id = std::env::args().nth(1).expect("pass a Discord application ID");
    let mut rpc = Rpc::new(&id).expect("invalid application ID");
    let start = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
    let p = Presence {
        details: "Novastrap test".into(),
        state: Some("Checking Rich Presence".into()),
        start,
        large_image: None,
        large_text: None,
        join_url: None,
    };
    match rpc.set(&p) {
        Ok(()) => println!("presence set; check your Discord profile"),
        Err(e) => {
            println!("failed: {e}");
            return;
        }
    }
    std::thread::sleep(std::time::Duration::from_secs(8));
    rpc.clear();
    println!("cleared");
}
