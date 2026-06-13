use axum::{extract::{Query, Request, State}, middleware::Next, response::{IntoResponse, Redirect}};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use reqwest::Client;
use rusqlite::OptionalExtension;
use time::Duration;
use tower_sessions::{Expiry, Session};
use crate::structs::{AppData, HcCallbackParams, HcClaims, HcTokenRequest, HcTokenResponse, User};
use rand::RngExt;

pub async fn root_handler(session: Session) -> Redirect {
    if session.get::<User>("userdata").await.unwrap_or(None).is_some() { Redirect::to("/chat") }
    else { Redirect::to("/login") }
}

pub async fn require_auth(
    session: Session,
    req: Request,
    next: Next,
) -> impl IntoResponse {
    let ok = session
        .get::<User>("userdata")
        .await
        .ok()
        .flatten()
        .is_some();

    if !ok {
        return Redirect::to("/login").into_response();
    }

    next.run(req).await
}

pub async fn logout(session: Session) -> Redirect {
    let _ = session.remove::<User>("userdata").await;
    Redirect::to("/")
}

fn gen_guest_name() -> String {
    let mut rng = rand::rng();
    format!("Guest#{}", rng.random_range(1000..9999))
}

fn gen_acc_name() -> String {
    let mut rng = rand::rng();
    format!("User#{}", rng.random_range(1000..9999))
}

pub async fn login_guest(session: Session) -> Redirect {
    session.set_expiry(Some(Expiry::OnSessionEnd));
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        name: gen_guest_name(),
        authenticated: false
    };

    session.insert("userdata", &user).await.unwrap();
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
        "https://auth.hackclub.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=openid+name+slack_id",
        dotenvy::var("HC_APP_ID").unwrap(),
        urlencoding::encode(&redirect_uri)
    );

    Redirect::to(&url)
}

#[axum::debug_handler]
pub async fn hc_callback(
    State(state): State<AppData>, Query(params): Query<HcCallbackParams>, session: Session, req: Request
) -> Result<Redirect, String> {
    let code = params.code;
    println!("got code: {}", code);

    let client_id = std::env::var("HC_APP_ID").map_err(|e| e.to_string())?;
    let client_secret = std::env::var("HC_APP_SECRET").map_err(|e| e.to_string())?;
    println!("got creds: {} / [secret hidden]", client_id);

    let client = Client::new();

    let host = req
        .headers()
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost:8080");

    let scheme = if cfg!(debug_assertions) { "http" } else { "https" };
    let redirect_uri = format!("{scheme}://{host}/auth/hc/callback");

    let res = client
        .post("https://auth.hackclub.com/oauth/token")
        .json(&HcTokenRequest {
            client_id: &client_id,
            client_secret: &client_secret,
            redirect_uri: &redirect_uri,
            code: &code,
            grant_type: "authorization_code",
        })
        .send()
        .await
        .map_err(|e| e.to_string())?;
    println!("token request status: {}", res.status());

    let status = res.status();
    let body = res.text().await.map_err(|e| e.to_string())?;
    println!("HC token status: {}", status);
    println!("HC token body: {}", body);

    if !status.is_success() {
        return Err(format!("{}: {}", status, body));
    }

    let token: HcTokenResponse = serde_json::from_str(&body).map_err(|e| e.to_string())?;
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
    let id = format!("hc-{}", claims.claims.slack_id.unwrap_or_else(|| panic!("Failed to retrieve slack id during auth!")));

    let user = {
        let conn = state.conn.lock().unwrap();

        let existing_user: Option<User> = conn.query_row(
            "SELECT id, username FROM users WHERE id = ?1",
            [&id],
            |row| Ok(User {
                id: row.get(0)?,
                name: row.get(1)?,
                authenticated: true,
            }),
        ).optional().map_err(|e| e.to_string())?;

        if let Some(existing) = existing_user {
            existing
        } else {
            let new_user = User {
                id: id,
                name: gen_acc_name(),
                authenticated: true,
            };
            conn.execute(
                "INSERT INTO users (id, username) VALUES (?1, ?2)",
                (&new_user.id, &new_user.name),
            ).map_err(|e| e.to_string())?;
            new_user
        }
    };
    
    session.set_expiry(Some(Expiry::AtDateTime(
        time::OffsetDateTime::now_utc() + Duration::days(30)
    )));
    session.insert("userdata", user).await.unwrap();

    Ok(Redirect::to("/"))
}