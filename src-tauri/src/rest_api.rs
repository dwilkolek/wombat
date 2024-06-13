use crate::shared::CookieJar;
use warp::Filter;
use warp::{self, http::StatusCode};

async fn put_cookie(
    name: String,
    value: String,
    jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>,
) -> std::result::Result<StatusCode, warp::Rejection> {
    // log::info!("Storing {name} = {value}");
    let mut jar = jar.lock().await;
    match jar.cookies.get(&name) {
        Some(old_value) => {
            if old_value.1 != value {
                jar.cookies.insert(name, (chrono::Utc::now(), value));
            }
        }
        None => {
            jar.cookies.insert(name, (chrono::Utc::now(), value));
        }
    }

    Ok(StatusCode::OK)
}

async fn delete_cookie(
    name: String,
    jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>,
) -> std::result::Result<StatusCode, warp::Rejection> {
    // log::info!("Delete {name}");
    let mut jar = jar.lock().await;
    jar.cookies.remove(&name);
    Ok(StatusCode::OK)
}
async fn health(
    jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>,
) -> std::result::Result<StatusCode, warp::Rejection> {
    // log::info!("health check: ok");
    let mut jar = jar.lock().await;
    jar.last_health_check = chrono::Utc::now();
    Ok(StatusCode::OK)
}

fn with_jar(
    jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>,
) -> impl Filter<
    Extract = (std::sync::Arc<tokio::sync::Mutex<CookieJar>>,),
    Error = std::convert::Infallible,
> + Clone {
    warp::any().map(move || jar.clone())
}

pub async fn serve(jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>) {
    warp::serve(
        warp::put()
            .and(warp::path("cookies"))
            .and(warp::path::param::<String>())
            .and(warp::body::json())
            .and(with_jar(jar.clone()))
            .and_then(put_cookie)
            .or(warp::delete()
                .and(warp::path("cookies"))
                .and(warp::path::param::<String>())
                .and(with_jar(jar.clone()))
                .and_then(delete_cookie))
            .or(warp::get()
                .and(warp::path("health"))
                .and(with_jar(jar.clone()))
                .and_then(health)),
    )
    .run(([127, 0, 0, 1], 6891))
    .await;
}
