use axum::{extract::{Path, Query, State}, http::{header::{COOKIE, ORIGIN, SET_COOKIE}, HeaderMap, HeaderValue, StatusCode}, response::{Html, IntoResponse, Response}, Json};
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
    userinfo_url: Option<&'static str>,
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
            userinfo_url: Some("https://www.googleapis.com/oauth2/v2/userinfo"),
            scopes: "https://www.googleapis.com/auth/cclog https://www.googleapis.com/auth/cloud-platform openid email profile",
            pkce: false,
        }),
        "codex" => Some(OAuthProvider {
            id: "codex",
            client_id_env: "CODEX_OAUTH_CLIENT_ID",
            client_secret_env: None,
            authorize_url: "https://auth.openai.com/oauth/authorize",
            token_url: "https://auth.openai.com/oauth/token",
            userinfo_url: None,
            scopes: "openid profile email offline_access",
            pkce: true,
        }),
        _ => None,
    }
}

fn jwt_claims(token: &str) -> Option<Value> {
    let payload = token.split('.').nth(1)?;
    serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload).ok()?).ok()
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

fn oauth_binding<'a>(headers: &'a HeaderMap, state: &str) -> Option<&'a str> {
    let name = format!("oauth_binding_{state}=");
    headers.get(COOKIE)?.to_str().ok()?.split(';').map(str::trim)
        .find_map(|cookie| cookie.strip_prefix(&name))
}

fn binding_matches(headers: &HeaderMap, state: &str, expected: &str) -> bool {
    let actual = oauth_binding(headers, state);
    actual.is_some_and(|a| a == expected)
}

fn callback_response(success: bool, provider: &str, state: &str) -> Response {
    let mut response = callback_page(success, provider).into_response();
    if let Ok(cookie) = HeaderValue::from_str(&format!(
        "oauth_binding_{state}=; HttpOnly; SameSite=Lax; Path=/api/oauth; Max-Age=0"
    )) {
        response.headers_mut().append(SET_COOKIE, cookie);
    }
    response
}

pub async fn start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider_id): Path<String>,
) -> Result<Response, (StatusCode, Json<Value>)> {
    let fail = |status, message: &str| (status, Json(json!({ "error": message })));
    let profile = provider(&provider_id).ok_or_else(|| fail(StatusCode::NOT_IMPLEMENTED, "OAuth flow is not implemented for this provider."))?;
    let client_id = std::env::var(profile.client_id_env).map_err(|_| fail(StatusCode::SERVICE_UNAVAILABLE, "OAuth client is not configured for this provider."))?;
    if let Some(secret_env) = profile.client_secret_env {
        std::env::var(secret_env).map_err(|_| fail(StatusCode::SERVICE_UNAVAILABLE, "OAuth client secret is not configured for this provider."))?;
    }
    let redirect_uri = redirect_uri().map_err(|message| fail(StatusCode::SERVICE_UNAVAILABLE, message))?;
    if let Some(origin) = headers.get(ORIGIN).and_then(|value| value.to_str().ok()) {
        let origin_url = Url::parse(origin).map_err(|_| fail(StatusCode::FORBIDDEN, "Invalid OAuth request origin."))?;
        let callback_url = Url::parse(&redirect_uri).map_err(|_| fail(StatusCode::SERVICE_UNAVAILABLE, "Invalid OAuth callback URL."))?;
        let local = |url: &Url| url.scheme() == "http" && url.host_str().is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]"));
        let same_origin = origin_url.origin() == callback_url.origin();
        let same_loopback_host = local(&origin_url) && local(&callback_url)
            && origin_url.scheme() == callback_url.scheme()
            && origin_url.host_str() == callback_url.host_str();
        if !same_origin && !same_loopback_host {
            return Err(fail(StatusCode::FORBIDDEN, "Use the same hostname for the dashboard and registered OAuth callback."));
        }
    }
    let state_token = uuid::Uuid::new_v4().to_string();
    let browser_binding = uuid::Uuid::new_v4().simple().to_string();
    let verifier = profile.pkce.then(code_verifier);
    let mut auth_url = Url::parse(profile.authorize_url).map_err(|_| fail(StatusCode::INTERNAL_SERVER_ERROR, "Invalid OAuth provider configuration."))?;
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
    let mut sessions = state.oauth_sessions.lock().map_err(|_| fail(StatusCode::INTERNAL_SERVER_ERROR, "OAuth session store is unavailable."))?;
    let now = Utc::now();
    sessions.retain(|_, session| session.expires_at > now);
    if sessions.len() >= 1024 {
        return Err(fail(StatusCode::TOO_MANY_REQUESTS, "Too many pending OAuth sessions."));
    }
    let secure = if Url::parse(&redirect_uri).is_ok_and(|url| url.scheme() == "https") { "; Secure" } else { "" };
    sessions.insert(state_token.clone(), OAuthSession {
        provider: profile.id.to_string(),
        redirect_uri,
        browser_binding: browser_binding.clone(),
        verifier,
        expires_at: now + Duration::minutes(10),
    });
    let cookie = HeaderValue::from_str(&format!(
        "oauth_binding_{state_token}={browser_binding}; HttpOnly; SameSite=Lax; Path=/api/oauth; Max-Age=600{secure}"
    )).map_err(|_| fail(StatusCode::INTERNAL_SERVER_ERROR, "Could not create OAuth browser binding."))?;
    let mut response = Json(json!({ "authUrl": auth_url.as_str() })).into_response();
    response.headers_mut().append(SET_COOKIE, cookie);
    Ok(response)
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    state: Option<String>,
    code: Option<String>,
    error: Option<String>,
}

pub async fn callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CallbackQuery>,
) -> Response {
    let Some(state_token) = query.state else { return callback_page(false, "").into_response() };
    let session = state.oauth_sessions.lock().ok().and_then(|mut sessions| sessions.remove(&state_token));
    let Some(session) = session.filter(|session| session.expires_at > Utc::now()) else {
        return callback_response(false, "", &state_token);
    };
    if !binding_matches(&headers, &state_token, &session.browser_binding) || query.error.is_some() {
        return callback_response(false, "", &state_token);
    }
    let Some(code) = query.code.filter(|code| !code.is_empty()) else {
        return callback_response(false, "", &state_token);
    };
    let Some(profile) = provider(&session.provider) else { return callback_response(false, "", &state_token) };
    let Ok(client_id) = std::env::var(profile.client_id_env) else { return callback_response(false, "", &state_token) };
    let client_secret = profile.client_secret_env.and_then(|name| std::env::var(name).ok());
    if profile.client_secret_env.is_some() && client_secret.is_none() { return callback_response(false, "", &state_token) }
    let client = reqwest::Client::builder().timeout(std::time::Duration::from_secs(20)).build();
    let Ok(client) = client else { return callback_response(false, "", &state_token) };
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
    let Ok(token_response) = token_response else { return callback_response(false, "", &state_token) };
    if !token_response.status().is_success() { return callback_response(false, "", &state_token) }
    let Ok(tokens) = token_response.json::<Value>().await else { return callback_response(false, "", &state_token) };
    let Some(access_token) = tokens.get("access_token").and_then(Value::as_str).filter(|value| !value.is_empty()) else {
        return callback_response(false, "", &state_token);
    };
    let user = if profile.id == "codex" {
        tokens.get("id_token").and_then(Value::as_str).and_then(jwt_claims)
            .or_else(|| jwt_claims(access_token))
    } else {
        let user_response = client.get(profile.userinfo_url.unwrap()).bearer_auth(access_token).send().await;
        let Ok(user_response) = user_response else { return callback_response(false, "", &state_token) };
        if !user_response.status().is_success() { return callback_response(false, "", &state_token) }
        let Ok(user) = user_response.json::<Value>().await else { return callback_response(false, "", &state_token) };
        Some(user)
    };
    let Some(user) = user else { return callback_response(false, "", &state_token) };
    let email = user.get("email").or_else(|| user.get("preferred_username"))
        .or_else(|| user.get("sub")).and_then(Value::as_str).filter(|email| !email.is_empty());
    let Some(email) = email else { return callback_response(false, "", &state_token) };
    if profile.id == "antigravity" && user.get("verified_email").and_then(Value::as_bool) != Some(true) {
        return callback_response(false, "", &state_token);
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
    callback_response(saved, profile.id, &state_token)
}

fn callback_page(success: bool, provider_id: &str) -> Html<String> {
    let provider = if provider(provider_id).is_some() { serde_json::to_string(provider_id).unwrap_or_else(|_| "\"\"".into()) } else { "\"\"".into() };
    let message = if success { "Account connected. You can close this window." } else { "Sign-in failed. Return to 9Router and try again." };
    let notification = if success {
        format!("if(window.opener)window.opener.postMessage({{type:'oauth_callback',provider:{provider},success:true}},window.location.origin);")
    } else { String::new() };
    Html(format!("<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"referrer\" content=\"no-referrer\"><title>9Router OAuth</title><body style=\"font:16px system-ui;max-width:36rem;margin:15vh auto;padding:1rem;background:#0b1020;color:#eef2ff\"><h1>{message}</h1><p>Return to your 9Router dashboard.</p><script>{notification}if(window.opener&&{success})window.close();</script></body></html>"))
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
    use super::{binding_matches, code_challenge, jwt_claims, provider, validate_redirect_uri};
    use axum::http::{header::COOKIE, HeaderMap, HeaderValue};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use serde_json::json;

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
    fn codex_identity_is_read_from_id_token_claims() {
        let claims = json!({ "email": "codex@example.com", "name": "Codex" });
        let token = format!("header.{}.signature", URL_SAFE_NO_PAD.encode(claims.to_string()));
        assert_eq!(jwt_claims(&token).unwrap()["email"], "codex@example.com");
        assert!(jwt_claims("not-a-jwt").is_none());
    }

    #[test]
    fn callback_requires_the_matching_browser_binding_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(COOKIE, HeaderValue::from_static("oauth_binding_state-1=secret"));
        assert!(binding_matches(&headers, "state-1", "secret"));
        assert!(!binding_matches(&headers, "state-1", "other"));
        assert!(!binding_matches(&headers, "state-2", "secret"));
    }

    #[test]
    fn redirect_uri_requires_registered_path_and_https_or_loopback() {
        assert!(validate_redirect_uri("http://localhost:20129/api/oauth/callback").is_ok());
        assert!(validate_redirect_uri("https://router.example/api/oauth/callback").is_ok());
        assert!(validate_redirect_uri("http://example.com/api/oauth/callback").is_err());
        assert!(validate_redirect_uri("https://router.example/callback").is_err());
    }
}
