//! Module 2: account manager commands.

use super::now;
use crate::state::AppState;
use novastrap_core::accounts::{self, SavedAccount, SessionStatus, oauth};
use novastrap_core::error::{AppError, AppResult};
use serde::Serialize;
use tauri::State;
use tauri_plugin_opener::OpenerExt;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    #[serde(flatten)]
    pub account: SavedAccount,
    pub status: SessionStatus,
    pub active: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountsView {
    pub accounts: Vec<AccountView>,
    pub oauth_configured: bool,
    pub redirect_uri: &'static str,
}

#[tauri::command]
pub async fn accounts_list(state: State<'_, AppState>) -> Result<AccountsView, ()> {
    let settings = state.settings.get();
    let client_id = settings.accounts.oauth_client_id.clone();
    let mut out = Vec::new();
    for a in state.accounts.get().recent() {
        let status = accounts::session_status(state.api.client(), &client_id, &a).await;
        out.push(AccountView { active: settings.accounts.active_user_id == Some(a.user_id), status, account: a });
    }
    Ok(AccountsView { accounts: out, oauth_configured: !client_id.trim().is_empty(), redirect_uri: oauth::REDIRECT_URI })
}

/// Saves a public profile (no sign-in) by username.
#[tauri::command]
pub async fn account_add_profile(state: State<'_, AppState>, username: String) -> AppResult<SavedAccount> {
    let acc = accounts::profile_by_username(&state.api, &username, now()).await?;
    let saved = acc.clone();
    state.accounts.update(|f| {
        let linked = f.accounts.iter().any(|a| a.user_id == acc.user_id && a.linked);
        f.upsert(SavedAccount { linked, ..acc });
    })?;
    if state.settings.get().accounts.active_user_id.is_none() {
        state.settings.update(|s| s.accounts.active_user_id = Some(saved.user_id))?;
    }
    Ok(saved)
}

/// Official "Sign in with Roblox": browser → loopback redirect → PKCE token exchange.
#[tauri::command]
pub async fn account_sign_in(app: tauri::AppHandle, state: State<'_, AppState>) -> AppResult<SavedAccount> {
    let client_id = state.settings.get().accounts.oauth_client_id.trim().to_owned();
    let pending = oauth::begin(&client_id)?;
    let listener = oauth::bind_listener()?;
    app.opener().open_url(pending.url.clone(), None::<&str>).map_err(|e| AppError::Launch(format!("couldn't open the browser: {e}")))?;
    let expected = pending.state.clone();
    let code = tokio::task::spawn_blocking(move || oauth::wait_for_code(listener, &expected, std::time::Duration::from_secs(300)))
        .await
        .map_err(|e| AppError::Launch(e.to_string()))??;
    let tokens = oauth::exchange(state.api.client(), &client_id, &code, &pending.verifier).await?;
    let info = oauth::userinfo(state.api.client(), &tokens.access_token).await?;
    let user_id: u64 = info.sub.parse().map_err(|_| AppError::UpstreamChanged("unexpected user id from Roblox".into()))?;
    accounts::save_tokens(user_id, &tokens)?;
    let mut acc = accounts::profile_by_id(&state.api, user_id, now()).await.unwrap_or(SavedAccount {
        user_id,
        username: info.preferred_username.unwrap_or_default(),
        display_name: info.nickname.or(info.name).unwrap_or_default(),
        avatar_url: None,
        linked: true,
        added_at: now(),
        last_used_at: None,
    });
    acc.linked = true;
    acc.last_used_at = Some(now());
    state.accounts.update(|f| f.upsert(acc.clone()))?;
    state.settings.update(|s| s.accounts.active_user_id = Some(user_id))?;
    log::info!("signed in Roblox user {user_id}");
    Ok(acc)
}

#[tauri::command]
pub fn account_switch(state: State<'_, AppState>, user_id: u64) -> AppResult<()> {
    if !state.accounts.get().accounts.iter().any(|a| a.user_id == user_id) {
        return Err(AppError::InvalidInput("That account isn't saved".into()));
    }
    state.accounts.update(|f| f.touch(user_id, now()))?;
    state.settings.update(|s| s.accounts.active_user_id = Some(user_id))
}

/// Removes a saved account; signs it out (revokes and deletes stored tokens).
#[tauri::command]
pub async fn account_remove(state: State<'_, AppState>, user_id: u64) -> AppResult<()> {
    if let Some(t) = accounts::load_tokens(user_id) {
        oauth::revoke(state.api.client(), &state.settings.get().accounts.oauth_client_id, &t.refresh_token).await;
    }
    accounts::delete_tokens(user_id);
    state.accounts.update(|f| f.remove(user_id))?;
    state.settings.update(|s| {
        if s.accounts.active_user_id == Some(user_id) {
            s.accounts.active_user_id = None;
        }
    })
}

#[tauri::command]
pub async fn avatar_render(state: State<'_, AppState>, user_id: u64) -> AppResult<Option<String>> {
    novastrap_core::catalog::avatar(&state.api, user_id).await
}
