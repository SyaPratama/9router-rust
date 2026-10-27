use axum::{extract::{Path, Query, State}, http::{header::ORIGIN, HeaderMap}, response::Html, Json};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use url::Url;
use crate::{api::middleware::{AppState, OAuthSession}, db::repos::connections::{ConnectionsRepo, ProviderConnection}};

struct OAuthProvider {
    id: &'static str,
    client_id_env: &'static str,
    client_secret_env: Option<&'static str>,
    authorize_url: &'static str,
    token_url: &'static str,
    userinfo_url: &'static str,
    scopes: &'static str,
    pkce: bool,
}

fn provider(id: &str) -> Option<OAuthProvider> {
    match id {
        "antigravity" => Some(OAuthProvider {
            id: "antigravity",
            client_id_env: "ANTIGRAVITY_OAUTH_CLIENT_ID",
            client_secret_env: Some("ANTIGRAVITY_OAUTH_CLIENT_SECRET"),
            authorize_url: "https://accounts.google.com/o/oauth2/v2/auth",
            token_url: "https://oauth2.googleapis.com/token",
            userinfo_url: "https://www.googleapis.com/oauth2/v2/userinfo",
            scopes: "https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/cloud-platform openid email profile",
            pkce: false,
        }),
        "codex" => Some(OAuthProvider {
            id: "codex",
            client_id_env: "CODEX_OAUTH_CLIENT_ID",
            client_secret_env: None,
            authorize_url: "https://auth.openai.com/oauth/authorize",
            token_url: "https://auth.openai.com/oauth/token",
            userinfo_url: "https://auth.openai.com/userinfo",
            scopes: "openid profile email offline_access",
            pkce: true,
        }),
        _ => None,
    }
}

fn validate_redirect_uri(value: &str) -> Result<(), &'static str> {
    let url = Url::parse(value).map_err(|_| "ROUTER_OAUTH_REDIRECT_URI is not a valid URL.")?;
    let local_http = url.scheme() == "http" && url.host_str().is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]"));
    if (url.scheme() != "https" && !local_http) || url.path() != "/api/oauth/callback" || url.query().is_some() || url.fragment().is_some() {
        return Err("OAuth callback must be HTTPS, or loopback HTTP, and end in /api/oauth/callback.");
    }
    Ok(())
}

fn redirect_uri() -> Result<String, &'static str> {
    let value = std::env::var("ROUTER_OAUTH_REDIRECT_URI").map_err(|_| "Set ROUTER_OAUTH_REDIRECT_URI to the registered OAuth callback URL.")?;
    validate_redirect_uri(&value)?;
    Ok(value)
}

fn code_verifier() -> String {
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}

fn code_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub async fn start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Json<Value>, (axum::http::StatusCode, Json<Value>)> {
    let fail = |status, message: &str| (status, Json(json!({ "error": message })));
    let profile = provider(&provider_id).ok_or_else(|| fail(axum::http::StatusCode::NOT_IMPLEMENTED, "OAuth flow is not implemented for this provider."))?;
    let client_id = std::env::var(profile.client_id_env).map_err(|_| fail(axum::http::StatusCode::SERVICE_UNAVAILABLE, "OAuth client is not configured for this provider."))?;
    if let Some(secret_env) = profile.client_secret_env {
        std::env::var(secret_env).map_err(|_| fail(axum::http::StatusCode::SERVICE_UNAVAILABLE, "OAuth client secret is not configured for this provider."))?;
    }
    let redirect_uri = redirect_uri().map_err(|message| fail(axum::http::StatusCode::SERVICE_UNAVAILABLE, message))?;
    if let Some(origin) = headers.get(ORIGIN).and_then(|value| value.to_str().ok()) {
        let callback_origin = Url::parse(&redirect_uri).map_err(|_| fail(axum::http::StatusCode::SERVICE_UNAVAILABLE, "Invalid OAuth callback URL."))?.origin().ascii_serialization();
        if origin != callback_origin {
            return Err(fail(axum::http::StatusCode::FORBIDDEN, "OAuth authorization must start from the configured dashboard origin."));
        }
    }
    let state_token = uuid::Uuid::new_v4().to_string();
    let verifier = profile.pkce.then(code_verifier);
    let mut auth_url = Url::parse(profile.authorize_url).map_err(|_| fail(axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Invalid OAuth provider configuration."))?;
    {
        let mut query = auth_url.query_pairs_mut();
        query.append_pair("client_id", &client_id)
            .append_pair("redirect_uri", &redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", profile.scopes)
            .append_pair("state", &state_token);
        if profile.id == "antigravity" {
            query.append_pair("access_type", "offline").append_pair("prompt", "consent");
        }
        if let Some(verifier) = &verifier {
            query.append_pair("code_challenge", &code_challenge(verifier))
                .append_pair("code_challenge_method", "S256");
        }
    }
    let mut sessions = state.oauth_sessions.lock().map_err(|_| fail(axum::http::StatusCode::INTERNAL_SERVER_ERROR, "OAuth session store is unavailable."))?;
    let now = Utc::now();
    sessions.retain(|_, session| session.expires_at > now);
    if sessions.len() >= 1024 {
        return Err(fail(axum::http::StatusCode::TOO_MANY_REQUESTS, "Too many pending OAuth sessions."));
    }
    sessions.insert(state_token.clone(), OAuthSession {
        provider: profile.id.to_string(),
        redirect_uri,
        verifier,
        expires_at: now + Duration::minutes(10),
    });
    Ok(Json(json!({ "authUrl": auth_url.as_str() })))
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
}

pub async fn callback(
    State(state): State<AppState>,
    Query(query): Query<CallbackQuery>,
) -> Html<String> {
    let Some(state_token) = query.state else { return callback_page(false, "") };
    let session = state.oauth_sessions.lock().ok().and_then(|mut sessions| sessions.remove(&state_token));
    let Some(session) = session.filter(|session| session.expires_at > Utc::now()) else { return callback_page(false, "") };
    if query.error.is_some() { return callback_page(false, "") }
    let Some(code) = query.code.filter(|code| !code.is_empty()) else { return callback_page(false, "") };
    let Some(profile) = provider(&session.provider) else { return callback_page(false, "") };
    let Ok(client_id) = std::env::var(profile.client_id_env) else { return callback_page(false, "") };
    let client_secret = profile.client_secret_env.and_then(|name| std::env::var(name).ok());
    if profile.client_secret_env.is_some() && client_secret.is_none() { return callback_page(false, "") }
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build();
    let Ok(client) = client else { return callback_page(false, "") };
    let mut form = vec![
        ("grant_type", "authorization_code".to_string()),
        ("client_id", client_id),
        ("code", code),
        ("redirect_uri", session.redirect_uri),
    ];
    if let Some(secret) = client_secret { form.push(("client_secret", secret)); }
    if let Some(verifier) = session.verifier { form.push(("code_verifier", verifier)); }
    let token_response = client.post(profile.token_url)
        .header(reqwest::header::ACCEPT, "application/json")
        .form(&form).send().await;
    let Ok(token_response) = token_response else { return callback_page(false, "") };
    if !token_response.status().is_success() { return callback_page(false, "") }
    let Ok(tokens) = token_response.json::<Value>().await else { return callback_page(false, "") };
    let Some(access_token) = tokens.get("access_token").and_then(Value::as_str).filter(|value| !value.is_empty()) else {
        return callback_page(false, "");
    };
    let user_response = client.get(profile.userinfo_url).bearer_auth(access_token).send().await;
    let Ok(user_response) = user_response else { return callback_page(false, "") };
    if !user_response.status().is_success() { return callback_page(false, "") }
    let Ok(user) = user_response.json::<Value>().await else { return callback_page(false, "") };
    let Some(email) = user.get("email").and_then(Value::as_str).filter(|email| !email.is_empty()) else {
        return callback_page(false, "");
    };
    if profile.id == "antigravity" && user.get("verified_email").and_then(Value::as_bool) != Some(true) {
        return callback_page(false, "");
    }
    let now = Utc::now();
    let expires_at = tokens.get("expires_in").and_then(Value::as_i64)
        .filter(|seconds| *seconds > 0).map(|seconds| now + Duration::seconds(seconds));
    let mut data = json!({
        "accessToken": access_token,
        "refreshToken": tokens.get("refresh_token").and_then(Value::as_str),
        "tokenType": tokens.get("token_type").and_then(Value::as_str).unwrap_or("Bearer"),
        "scope": tokens.get("scope").and_then(Value::as_str).unwrap_or(profile.scopes),
        "expiresAt": expires_at.map(|time| time.to_rfc3339()),
        "lastRefreshAt": now.to_rfc3339(),
    });
    if let Some(name) = user.get("name").and_then(Value::as_str) {
        data["accountName"] = json!(name);
    }
    let connection = ProviderConnection {
        id: uuid::Uuid::new_v4().to_string(),
        provider: profile.id.to_string(),
        auth_type: "oauth".into(),
        name: user.get("name").and_then(Value::as_str).map(str::to_owned).or_else(|| Some(email.to_owned())),
        email: Some(email.to_owned()),
        priority: Some(1),
        is_active: true,
        data,
        created_at: now.to_rfc3339(),
        updated_at: now.to_rfc3339(),
    };
    let saved = state.engine.pool.get().ok()
        .and_then(|conn| ConnectionsRepo::upsert(&conn, &connection).ok()).is_some();
    callback_page(saved, profile.id)
}

fn callback_page(success: bool, provider: &str) -> Html<String> {
    let provider = if matches!(provider, "antigravity" | "codex") { provider } else { "" };
    let message = if success { "Account connected. You can close this window." } else { "Sign-in failed. Return to 9Router and try again." };
    let notification = if success {
        format!("if(window.opener)window.opener.postMessage({{type:'oauth_callback',provider:'{provider}',success:true}},window.location.origin);")
    } else { String::new() };
    Html(format!("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>9Router OAuth</title><body style=\"font:16px system-ui;max-width:36rem;margin:15vh auto;padding:1rem;background:#0b1020;color:#eef2ff\"><h1>{message}</h1><p>Return to your 9Router dashboard.</p><script>{notification}if(window.opener&&{success})window.close();</script></body></html>"))
}
pub async fn access_token(
    pool: &crate::db::DbPool,
    connection: &ProviderConnection,
) -> Result<String, &'static str> {
    let access_token = connection.data.get("accessToken").and_then(Value::as_str).filter(|value| !value.is_empty());
    let expires_at = connection.data.get("expiresAt").and_then(Value::as_str)
        .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok());
    if let Some(token) = access_token.filter(|_| expires_at.is_none_or(|expiry| expiry > Utc::now() + Duration::seconds(60))) {
        return Ok(token.to_owned());
    }
    let profile = provider(&connection.provider).ok_or("OAuth refresh is not supported for this provider.")?;
    let refresh_token = connection.data.get("refreshToken").and_then(Value::as_str).filter(|value| !value.is_empty())
        .ok_or("OAuth token expired; reconnect this account.")?;
    let client_id = std::env::var(profile.client_id_env).map_err(|_| "OAuth client is not configured; reconnect this account after setup.")?;
    let client_secret = profile.client_secret_env.and_then(|name| std::env::var(name).ok());
    if profile.client_secret_env.is_some() && client_secret.is_none() {
        return Err("OAuth client secret is not configured; reconnect this account after setup.");
    }
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build()
        .map_err(|_| "Could not initialize OAuth token refresh.")?;
    let mut form = vec![
        ("grant_type", "refresh_token".to_string()),
        ("client_id", client_id),
        ("refresh_token", refresh_token.to_owned()),
    ];
    if let Some(secret) = client_secret { form.push(("client_secret", secret)); }
    let response = client.post(profile.token_url).form(&form).send().await
        .map_err(|_| "OAuth token refresh request failed.")?;
    if !response.status().is_success() { return Err("OAuth token refresh was rejected; reconnect this account.") }
    let tokens: Value = response.json().await.map_err(|_| "OAuth token refresh response was invalid.")?;
    let token = tokens.get("access_token").and_then(Value::as_str).filter(|value| !value.is_empty())
        .ok_or("OAuth token refresh response did not include an access token.")?;
    let now = Utc::now();
    let mut refreshed = connection.clone();
    let data = refreshed.data.as_object_mut().ok_or("Saved OAuth connection data is invalid.")?;
    data.insert("accessToken".into(), json!(token));
    if let Some(value) = tokens.get("refresh_token").and_then(Value::as_str) {
        data.insert("refreshToken".into(), json!(value));
    }
    if let Some(seconds) = tokens.get("expires_in").and_then(Value::as_i64).filter(|seconds| *seconds > 0) {
        data.insert("expiresAt".into(), json!((now + Duration::seconds(seconds)).to_rfc3339()));
    }
    data.insert("lastRefreshAt".into(), json!(now.to_rfc3339()));
    refreshed.updated_at = now.to_rfc3339();
    let conn = pool.get().map_err(|_| "Could not save refreshed OAuth credentials.")?;
    ConnectionsRepo::upsert(&conn, &refreshed).map_err(|_| "Could not save refreshed OAuth credentials.")?;
    Ok(token.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{code_challenge, provider, validate_redirect_uri};

    #[test]
    fn pkce_challenge_uses_s256_base64url_without_padding() {
        assert_eq!(code_challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
    }

    #[test]
    fn oauth_support_is_explicit() {
        assert!(provider("antigravity").is_some());
        assert!(provider("codex").is_some());
        assert!(provider("cursor").is_none());
    }

    #[test]
    fn redirect_uri_requires_registered_path_and_https_or_loopback() {
        assert!(validate_redirect_uri("http://localhost:20129/api/oauth/callback").is_ok());
        assert!(validate_redirect_uri("https://router.example/api/oauth/callback").is_ok());
        assert!(validate_redirect_uri("http://example.com/api/oauth/callback").is_err());
        assert!(validate_redirect_uri("https://router.example/callback").is_err());
    }
}
