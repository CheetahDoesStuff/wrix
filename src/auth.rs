use axum::response::Redirect;
use tower_sessions::Session;
use crate::structs::User;
use rand::RngExt;

fn gen_guest_name() -> String {
    let mut rng = rand::rng();
    format!("Guest#{}", rng.random_range(1000..9999))
}

pub async fn login_guest(session: Session) -> Redirect {
    let user = User {
        id: uuid::Uuid::new_v4().to_string(),
        name: gen_guest_name(),
        authenticated: false
    };

    session.insert("user", &user).await.unwrap();
    Redirect::to("/")
}