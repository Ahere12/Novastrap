//! Novastrap core services. Kept free of Tauri so it can be unit-tested and reused
//! (e.g. by a future lightweight protocol-handler binary).

pub mod error;
pub mod roblox;
pub use reqwest;
