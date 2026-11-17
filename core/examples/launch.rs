//! Launches Roblox the same way the app does: `cargo run --example launch -- [place id or link]`.
use novastrap_core::roblox::{deployment, detect, launch};

#[tokio::main]
async fn main() {
    let input = std::env::args().nth(1).unwrap_or_default();
    let target = launch::LaunchTarget::parse_user_input(&input).expect("bad input");
    let installs: Vec<_> = detect::official_versions_dirs().iter().flat_map(|d| detect::scan_versions_dir(d)).collect();
    let handler = detect::classify_handler(detect::read_protocol_command().as_deref());
    let latest = deployment::latest(&deployment::http_client().unwrap(), None).await.ok();
    let exe = launch::choose_executable(&installs, latest.as_ref().map(|v| v.client_version_upload.as_str()), &handler)
        .expect("Roblox not installed");
    println!("exe: {}", exe.display());
    println!("arg: {:?}", target.client_argument());
    println!("pid: {}", launch::spawn(exe, &target).unwrap());
}
