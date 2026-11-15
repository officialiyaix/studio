//! Microsoft account sign-in for Minecraft: Java Edition.
//!
//! Flow (all in the system browser — Stabled never sees a password):
//! 1. OAuth 2.0 authorization code + PKCE against the Microsoft identity platform (`consumers` tenant), with a
//!    loopback redirect `http://localhost:<port>` handled by a one-shot local listener.
//! 2. Xbox Live user token → XSTS token for `rp://api.minecraftservices.com/`.
//! 3. Minecraft services login → access token; entitlement check; game profile.
//!
//! Only the Microsoft refresh token, the current Minecraft token and the public profile are stored, encrypted with
//! Windows DPAPI (see `store`). Tokens are never sent to the frontend, written to settings, or logged.

mod store;

use crate::bail;
use crate::error::{AppError, AppResult};
use crate::util::download::client;
use crate::util::log;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Digest;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::oneshot;

const AUTHORIZE: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
const TOKEN: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub skin_url: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    ms_refresh_token: String,
    mc_access_token: String,
    mc_expires_at: i64,
    owns_java: bool,
    profile: Option<Profile>,
}

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account").field("owns_java", &self.owns_java).field("profile", &self.profile).finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthStatus {
    /// "unconfigured" | "signed_out" | "signing_in" | "signed_in" | "no_java" | "offline"
    pub state: String,
    pub profile: Option<PublicProfile>,
    pub avatar: Option<String>,
    pub message: Option<String>,
    pub persistent: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicProfile {
    pub id: String,
    pub name: String,
}

pub struct AuthManager {
    account: Mutex<Option<Account>>,
    status: Mutex<AuthStatus>,
    cancel: Mutex<Option<oneshot::Sender<()>>>,
    refresh_lock: tokio::sync::Mutex<()>,
}

fn status(state: &str, account: Option<&Account>, message: Option<String>) -> AuthStatus {
    AuthStatus {
        state: state.into(),
        profile: account.and_then(|a| a.profile.as_ref()).map(|p| PublicProfile { id: p.id.clone(), name: p.name.clone() }),
        avatar: account.and_then(|a| a.profile.as_ref()).and_then(|p| cached_avatar(&p.id)),
        message,
        persistent: store::is_persistent(),
    }
}

fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

impl AuthManager {
    pub fn new() -> Self {
        let initial = if crate::config::get().microsoft_client_id.is_none() { "unconfigured" } else { "signed_out" };
        AuthManager {
            account: Mutex::new(None),
            status: Mutex::new(status(initial, None, None)),
            cancel: Mutex::new(None),
            refresh_lock: tokio::sync::Mutex::new(()),
        }
    }

    pub fn status(&self) -> AuthStatus {
        self.status.lock().unwrap().clone()
    }

    fn set(&self, s: AuthStatus) -> AuthStatus {
        *self.status.lock().unwrap() = s.clone();
        s
    }

    fn client_id() -> AppResult<String> {
        crate::config::get()
            .microsoft_client_id
            .clone()
            .ok_or_else(|| AppError::new("AUTH_UNCONFIGURED", "Microsoft sign-in is not configured in this build of Stabled (MICROSOFT_CLIENT_ID is missing)."))
    }

    /// Called on every startup: restore the stored session and refresh it if needed.
    pub async fn restore(&self) -> AuthStatus {
        if Self::client_id().is_err() {
            return self.set(status("unconfigured", None, None));
        }
        let Some(account) = store::load::<Account>() else {
            return self.set(status("signed_out", None, None));
        };
        log::register_secret(&account.ms_refresh_token);
        log::register_secret(&account.mc_access_token);
        *self.account.lock().unwrap() = Some(account.clone());
        if account.mc_expires_at - now() > 600 {
            let st = if account.owns_java && account.profile.is_some() { "signed_in" } else { "no_java" };
            let msg = no_java_message(&account);
            ensure_avatar(&account).await;
            return self.set(status(st, Some(&account), msg));
        }
        match self.refresh().await {
            Ok(s) => s,
            Err(e) if e.code() == "AUTH_EXPIRED" => {
                store::clear();
                *self.account.lock().unwrap() = None;
                self.set(status("signed_out", None, Some("Your Microsoft session expired. Sign in again to play.".into())))
            }
            Err(e) => self.set(status("offline", Some(&account), Some(format!("Couldn't refresh your session: {e}")))),
        }
    }

    async fn refresh(&self) -> AppResult<AuthStatus> {
        let _guard = self.refresh_lock.lock().await;
        let Some(account) = self.account.lock().unwrap().clone() else { bail!("SIGN_IN_REQUIRED", "Sign in with Microsoft to play.") };
        let client_id = Self::client_id()?;
        let resp = client()
            .post(TOKEN)
            .form(&[("client_id", client_id.as_str()), ("grant_type", "refresh_token"), ("refresh_token", account.ms_refresh_token.as_str()), ("scope", SCOPE)])
            .send()
            .await?;
        let body: Value = resp.json().await?;
        if body.get("error").is_some() {
            bail!("AUTH_EXPIRED", "Microsoft session expired");
        }
        let fresh = complete_chain(&body).await?;
        self.accept(fresh).await
    }

    async fn accept(&self, account: Account) -> AppResult<AuthStatus> {
        log::register_secret(&account.ms_refresh_token);
        log::register_secret(&account.mc_access_token);
        store::save(&account)?;
        ensure_avatar(&account).await;
        let st = if account.owns_java && account.profile.is_some() { "signed_in" } else { "no_java" };
        let msg = no_java_message(&account);
        *self.account.lock().unwrap() = Some(account.clone());
        Ok(self.set(status(st, Some(&account), msg)))
    }

    pub fn cancel(&self) {
        if let Some(tx) = self.cancel.lock().unwrap().take() {
            let _ = tx.send(());
        }
    }

    pub async fn sign_in(&self, open_url: impl FnOnce(&str) -> AppResult<()>) -> AppResult<AuthStatus> {
        let client_id = Self::client_id()?;
        self.cancel();
        let previous = self.status();
        self.set(status("signing_in", None, None));
        let result = self.sign_in_inner(&client_id, open_url).await;
        match result {
            Ok(account) => self.accept(account).await,
            Err(e) => {
                let restored = self.account.lock().unwrap().clone();
                let base = if restored.is_some() { previous.state.clone() } else { "signed_out".into() };
                self.set(status(&base, restored.as_ref(), None));
                Err(e)
            }
        }
    }

    async fn sign_in_inner(&self, client_id: &str, open_url: impl FnOnce(&str) -> AppResult<()>) -> AppResult<Account> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let redirect = format!("http://localhost:{port}");
        let verifier = random_token(64);
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(verifier.as_bytes()));
        let state = random_token(24);
        let mut url = url::Url::parse(AUTHORIZE).unwrap();
        url.query_pairs_mut()
            .append_pair("client_id", client_id)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", &redirect)
            .append_pair("scope", SCOPE)
            .append_pair("code_challenge", &challenge)
            .append_pair("code_challenge_method", "S256")
            .append_pair("state", &state)
            .append_pair("prompt", "select_account");
        let (cancel_tx, cancel_rx) = oneshot::channel();
        *self.cancel.lock().unwrap() = Some(cancel_tx);
        open_url(url.as_str())?;
        let code = tokio::select! {
            r = wait_for_code(listener, &state) => r?,
            _ = cancel_rx => bail!("AUTH_CANCELLED", "Sign-in was cancelled."),
            _ = tokio::time::sleep(Duration::from_secs(300)) => bail!("AUTH_TIMEOUT", "Sign-in timed out. Try again when you're ready."),
        };
        log::register_secret(&code);
        let resp = client()
            .post(TOKEN)
            .form(&[
                ("client_id", client_id),
                ("grant_type", "authorization_code"),
                ("code", code.as_str()),
                ("redirect_uri", redirect.as_str()),
                ("code_verifier", verifier.as_str()),
                ("scope", SCOPE),
            ])
            .send()
            .await?;
        let body: Value = resp.json().await?;
        if let Some(err) = body.get("error_description").and_then(Value::as_str) {
            bail!("AUTH_FAILED", "Microsoft sign-in failed: {}", err.lines().next().unwrap_or(err));
        }
        complete_chain(&body).await
    }

    pub fn sign_out(&self) -> AuthStatus {
        self.cancel();
        store::clear();
        *self.account.lock().unwrap() = None;
        let st = if Self::client_id().is_ok() { "signed_out" } else { "unconfigured" };
        self.set(status(st, None, None))
    }

    /// Server-side enforcement for launching: returns credentials only for a signed-in account that owns Java Edition.
    pub async fn launch_credentials(&self) -> AppResult<(Profile, String)> {
        Self::client_id()?;
        let account = self.account.lock().unwrap().clone();
        let Some(mut account) = account else { bail!("SIGN_IN_REQUIRED", "Sign in with Microsoft to play Minecraft.") };
        if account.mc_expires_at - now() < 300 {
            self.refresh().await.map_err(|e| {
                if e.code() == "AUTH_EXPIRED" {
                    AppError::new("SIGN_IN_REQUIRED", "Your Microsoft session expired. Sign in again to play.")
                } else {
                    e
                }
            })?;
            account = self.account.lock().unwrap().clone().ok_or_else(|| AppError::new("SIGN_IN_REQUIRED", "Sign in to play."))?;
        }
        if !account.owns_java {
            bail!("NO_JAVA_ACCESS", "This Microsoft account doesn't have Minecraft: Java Edition.");
        }
        let Some(profile) = account.profile.clone() else {
            bail!("NO_JAVA_ACCESS", "This account owns Minecraft but has no Java Edition profile yet. Create your profile name at minecraft.net, then try again.");
        };
        Ok((profile, account.mc_access_token.clone()))
    }
}

fn no_java_message(a: &Account) -> Option<String> {
    if !a.owns_java {
        Some("Signed in to Microsoft, but this account doesn't have Minecraft: Java Edition. You can still use everything except Launch.".into())
    } else if a.profile.is_none() {
        Some("This account owns Minecraft but has no Java Edition profile yet. Create your profile name at minecraft.net.".into())
    } else {
        None
    }
}

fn random_token(len: usize) -> String {
    let mut bytes = vec![0u8; len];
    getrandom::fill(&mut bytes).expect("system randomness");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

async fn wait_for_code(listener: tokio::net::TcpListener, expected_state: &str) -> AppResult<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buf = vec![0u8; 8192];
        let n = tokio::time::timeout(Duration::from_secs(10), stream.read(&mut buf)).await.unwrap_or(Ok(0)).unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]).to_string();
        let path = request.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/");
        let Ok(url) = url::Url::parse(&format!("http://localhost{path}")) else { continue };
        let params: std::collections::HashMap<String, String> = url.query_pairs().into_owned().collect();
        if params.is_empty() {
            // Browsers sometimes probe /favicon.ico; ignore anything without a result.
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
            continue;
        }
        let (ok, title, detail) = if params.get("state").map(String::as_str) != Some(expected_state) {
            (false, "Sign-in could not be verified", "Please return to Stabled and try again.")
        } else if params.contains_key("code") {
            (true, "You're signed in", "You can close this tab and return to Stabled.")
        } else {
            (false, "Sign-in was cancelled", "You can close this tab. Stabled is still available without signing in.")
        };
        let page = callback_page(title, detail, ok);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{}",
            page.len(),
            page
        );
        let _ = stream.write_all(response.as_bytes()).await;
        let _ = stream.shutdown().await;
        if params.get("state").map(String::as_str) != Some(expected_state) {
            bail!("AUTH_FAILED", "The sign-in response could not be verified.");
        }
        if let Some(code) = params.get("code") {
            return Ok(code.clone());
        }
        let reason = params.get("error").cloned().unwrap_or_default();
        if reason == "access_denied" {
            bail!("AUTH_CANCELLED", "Sign-in was cancelled.");
        }
        bail!("AUTH_FAILED", "Microsoft sign-in did not complete ({reason}).");
    }
}

fn callback_page(title: &str, detail: &str, ok: bool) -> String {
    let accent = if ok { "#22c55e" } else { "#f5a524" };
    format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>Stabled</title>\
<style>body{{margin:0;min-height:100vh;display:grid;place-items:center;background:#0e0e13;color:#ecebf5;font-family:Segoe UI,system-ui,sans-serif}}\
.card{{max-width:420px;padding:32px;border-radius:16px;background:#17161f;border:1px solid #2d2a3c;text-align:center}}\
.dot{{width:12px;height:12px;border-radius:50%;background:{accent};display:inline-block;margin-bottom:12px}}h1{{font-size:20px;margin:0 0 8px}}p{{color:#9b98ae;margin:0}}</style></head>\
<body><div class=\"card\"><span class=\"dot\"></span><h1>{title}</h1><p>{detail}</p></div></body></html>"
    )
}

async fn post_json(url: &str, body: Value) -> AppResult<(reqwest::StatusCode, Value)> {
    let resp = client().post(url).header("Accept", "application/json").json(&body).send().await?;
    let status = resp.status();
    let v = resp.json::<Value>().await.unwrap_or(Value::Null);
    Ok((status, v))
}

/// Microsoft token response → Xbox Live → XSTS → Minecraft token, entitlement and profile.
async fn complete_chain(ms: &Value) -> AppResult<Account> {
    let access = ms["access_token"].as_str().ok_or_else(|| AppError::new("AUTH_FAILED", "Microsoft did not return an access token."))?;
    let refresh = ms["refresh_token"].as_str().ok_or_else(|| AppError::new("AUTH_FAILED", "Microsoft did not return a refresh token."))?;
    log::register_secret(access);
    log::register_secret(refresh);

    let (st, xbl) = post_json(
        "https://user.auth.xboxlive.com/user/authenticate",
        json!({ "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={access}") },
                "RelyingParty": "http://auth.xboxlive.com", "TokenType": "JWT" }),
    )
    .await?;
    if !st.is_success() {
        bail!("AUTH_FAILED", "Xbox Live sign-in failed ({st}).");
    }
    let xbl_token = xbl["Token"].as_str().unwrap_or_default().to_string();
    let uhs = xbl["DisplayClaims"]["xui"][0]["uhs"].as_str().unwrap_or_default().to_string();

    let (st, xsts) = post_json(
        "https://xsts.auth.xboxlive.com/xsts/authorize",
        json!({ "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl_token] }, "RelyingParty": "rp://api.minecraftservices.com/", "TokenType": "JWT" }),
    )
    .await?;
    if !st.is_success() {
        let message = match xsts["XErr"].as_u64() {
            Some(2148916233) => "This Microsoft account has no Xbox profile yet. Sign in once at xbox.com to create one, then try again.",
            Some(2148916235) => "Xbox Live is not available in this account's country or region.",
            Some(2148916236) | Some(2148916237) => "This account needs adult verification on the Xbox website before it can play.",
            Some(2148916238) => "This is a child account. An adult needs to add it to a Microsoft family before it can play.",
            _ => "Xbox Live authorization failed.",
        };
        bail!("NO_JAVA_ACCESS", "{message}");
    }
    let xsts_token = xsts["Token"].as_str().unwrap_or_default();

    let (st, mc) = post_json(
        "https://api.minecraftservices.com/authentication/login_with_xbox",
        json!({ "identityToken": format!("XBL3.0 x={uhs};{xsts_token}") }),
    )
    .await?;
    if !st.is_success() {
        bail!("AUTH_FAILED", "Minecraft services rejected the sign-in ({st}). If this keeps happening, the Stabled build's Microsoft application may not be approved for Minecraft.");
    }
    let mc_token = mc["access_token"].as_str().unwrap_or_default().to_string();
    log::register_secret(&mc_token);
    let expires_in = mc["expires_in"].as_i64().unwrap_or(86_400);

    let owns_java = check_entitlement(&mc_token).await?;
    let profile = fetch_profile(&mc_token).await?;
    Ok(Account { ms_refresh_token: refresh.to_string(), mc_access_token: mc_token, mc_expires_at: now() + expires_in, owns_java, profile })
}

async fn check_entitlement(token: &str) -> AppResult<bool> {
    let url = format!("https://api.minecraftservices.com/entitlements/license?requestId={}", uuid::Uuid::new_v4());
    let resp = client().get(url).bearer_auth(token).send().await?;
    if !resp.status().is_success() {
        let resp = client().get("https://api.minecraftservices.com/entitlements/mcstore").bearer_auth(token).send().await?;
        let v: Value = resp.json().await.unwrap_or(Value::Null);
        return Ok(has_java(&v));
    }
    let v: Value = resp.json().await.unwrap_or(Value::Null);
    Ok(has_java(&v))
}

fn has_java(v: &Value) -> bool {
    v["items"].as_array().into_iter().flatten().any(|i| matches!(i["name"].as_str(), Some("product_minecraft") | Some("game_minecraft")))
}

async fn fetch_profile(token: &str) -> AppResult<Option<Profile>> {
    let resp = client().get("https://api.minecraftservices.com/minecraft/profile").bearer_auth(token).send().await?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        bail!("AUTH_FAILED", "Could not load your Minecraft profile ({}).", resp.status());
    }
    let v: Value = resp.json().await?;
    let skin = v["skins"].as_array().into_iter().flatten().find(|s| s["state"] == "ACTIVE").and_then(|s| s["url"].as_str()).map(str::to_owned);
    Ok(Some(Profile { id: v["id"].as_str().unwrap_or_default().into(), name: v["name"].as_str().unwrap_or_default().into(), skin_url: skin }))
}

fn avatar_path(uuid: &str) -> Option<std::path::PathBuf> {
    uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-').then(|| crate::paths::cache_dir().join(format!("avatar-{uuid}.png")))
}

fn cached_avatar(uuid: &str) -> Option<String> {
    let bytes = std::fs::read(avatar_path(uuid)?).ok()?;
    Some(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// Fetches the player's actual skin (Mojang texture server) and stores the rendered face; best effort.
async fn ensure_avatar(a: &Account) {
    let Some(p) = &a.profile else { return };
    let Some(url) = &p.skin_url else { return };
    let Some(path) = avatar_path(&p.id) else { return };
    let ok_host = url::Url::parse(url).ok().and_then(|u| u.host_str().map(|h| h == "textures.minecraft.net")).unwrap_or(false);
    if !ok_host {
        return;
    }
    let url = url.replacen("http://", "https://", 1);
    if let Ok(resp) = client().get(&url).timeout(Duration::from_secs(10)).send().await {
        if let Ok(bytes) = resp.bytes().await {
            if let Some(face) = crate::images::avatar_from_skin(&bytes) {
                let _ = crate::util::fsx::write_atomic(&path, &face);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Launch enforcement lives in the backend: without a verified session no credentials are released, so
    /// `save_launch` returns before any download or process is started.
    #[test]
    fn launch_refused_without_session() {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let auth = AuthManager::new();
        let err = rt.block_on(auth.launch_credentials()).unwrap_err();
        let expected = if crate::config::get().microsoft_client_id.is_some() { "SIGN_IN_REQUIRED" } else { "AUTH_UNCONFIGURED" };
        assert_eq!(err.code(), expected);
        assert_ne!(auth.status().state, "signed_in");
    }

    #[test]
    fn account_debug_never_prints_tokens() {
        let a = Account {
            ms_refresh_token: "refresh-secret-value".into(),
            mc_access_token: "access-secret-value".into(),
            mc_expires_at: 0,
            owns_java: true,
            profile: None,
        };
        let shown = format!("{a:?}");
        assert!(!shown.contains("secret-value"));
    }
}
