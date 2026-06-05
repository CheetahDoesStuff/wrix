use axum::{extract::{Query, Request}, response::Redirect};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use reqwest::Client;
use time::Duration;
use tower_sessions::{Expiry, Session};
use crate::structs::{HcCallbackParams, HcClaims, HcTokenRequest, HcTokenResponse, User};
use rand::RngExt;

pub async fn root_handler(session: Session) -> Redirect {
    if session.get::<User>("user").await.unwrap_or(None).is_some() { Redirect::to("/chat") }
    else { Redirect::to("/login") }
}

fn gen_guest_name() -> String {
    let mut rng = rand::rng();
    format!("Guest#{}", rng.random_range(1000..9999))
}

pub async fn login_guest(session: Session) -> Redirect {
    session.set_expiry(Some(Expiry::OnSessionEnd));
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        name: gen_guest_name(),
        authenticated: false
    };

    session.insert("user", &user).await.unwrap();
    Redirect::to("/")
}

pub async fn hc_auth_redirect(req: Request) -> Redirect {
    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost:8080");

    let scheme = if cfg!(debug_assertions) { "http" } else { "https" };
    let redirect_uri = format!("{scheme}://{host}/auth/hc/callback");
    let url = format!(
        "https://auth.hackclub.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid+profile",
        dotenvy::var("HC_APP_ID").unwrap(),
        urlencoding::encode(&redirect_uri)
    );

    Redirect::to(&url)
}

pub async fn hc_callback(
    Query(params): Query<HcCallbackParams>, session: Session
) -> Result<Redirect, String> {
    let code = params.code;
    println!("got code: {}", code);

    let client_id = std::env::var("HC_APP_ID").map_err(|e| e.to_string())?;
    let client_secret = std::env::var("HC_APP_SECRET").map_err(|e| e.to_string())?;
    println!("got creds: {} / [secret hidden]", client_id);

    let client = Client::new();

    let res = client
        .post("https://auth.hackclub.com/oauth/token")
        .json(&HcTokenRequest {
            client_id: &client_id,
            client_secret: &client_secret,
            redirect_uri: "http://localhost:8080/auth/hc/callback",
            code: &code,
            grant_type: "authorization_code",
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    println!("token request status: {}", res.status());

    let res = res.error_for_status().map_err(|e| e.to_string())?;
    println!("status ok");

    let token: HcTokenResponse = res.json().await.map_err(|e| e.to_string())?;
    println!("got token: {:?}", token);

    let jwks: serde_json::Value = Client::new()
        .get("https://auth.hackclub.com/oauth/discovery/keys")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    println!("got jwks");

    let keys = jwks["keys"].as_array().ok_or("no keys")?;
    let key = &keys[0];
    let n = key["n"].as_str().ok_or("no n")?;
    let e = key["e"].as_str().ok_or("no e")?;

    let decoding_key = DecodingKey::from_rsa_components(n, e)
        .map_err(|e| e.to_string())?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[std::env::var("HC_APP_ID").map_err(|e| e.to_string())?]);

    let claims = decode::<HcClaims>(
        &token.id_token.ok_or("no id_token")?,
        &decoding_key,
        &validation,
    ).map_err(|e| e.to_string())?;
    println!("decoded claims: {:?}", claims.claims);

    let first_name = claims.claims.given_name.unwrap_or("Unknown".to_string());

    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        name: first_name,
        authenticated: true,
    };

    session.set_expiry(Some(Expiry::AtDateTime(
        time::OffsetDateTime::now_utc() + Duration::days(30)
    )));
    session.insert("user", user).await.unwrap();
    println!("inserted user into session, redirecting");

    Ok(Redirect::to("/"))
}