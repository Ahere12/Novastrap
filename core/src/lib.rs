//! Novastrap core services. Kept free of Tauri so it can be unit-tested and reused
//! (e.g. by a future lightweight protocol-handler binary).

pub mod accounts;
pub mod activity;
pub mod catalog;
pub mod discord;
pub mod discover;
pub mod error;
pub mod favorites;
pub mod instances;
pub mod links;
pub mod mods;
pub mod notices;
pub mod performance;
pub mod releases;
pub mod roblox;
pub mod servers;
pub mod settings;
pub mod shortcut;
pub mod store;
pub mod trading;
pub mod web;

pub use reqwest;
