use crate::shared::{BrowserExtension, Cookie, CookieJar, Env};
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use warp::reply::Reply;
use warp::Filter;
use warp::{self, http::StatusCode};

#[derive(Clone, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct NewCookieDto {
    name: String,
    value: String,
    env: Env,
    stored_at: i64,
}
impl From<NewCookieDto> for Cookie {
    fn from(dto: NewCookieDto) -> Self {
        Cookie {
            name: dto.name,
            env: dto.env,
            value: dto.value,
            stored_at: Utc.timestamp_millis_opt(dto.stored_at).unwrap(),
        }
    }
}

async fn put_cookie(
    dto: NewCookieDto,
    jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>,
) -> Result<warp::reply::Response, warp::Rejection> {
    log::info!(
        "Storing cookie for env={}, name={}, value={}",
        &dto.env,
        &dto.name,
        &dto.value
    );
    let mut jar = jar.lock().await;
    jar.cookies.retain(|cookie| cookie.name != dto.name);
    jar.cookies.push(Cookie::from(dto));
    Ok(warp::reply().into_response())
}

async fn delete_cookie(
    name: String,
    jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>,
) -> Result<warp::reply::Response, warp::Rejection> {
    log::info!("Deleted cookie: {name}");
    let mut jar = jar.lock().await;
    jar.cookies.retain(|cookie| cookie.name != name);
    Ok(warp::reply().into_response())
}
async fn health(
    version: String,
    browser_ext: std::sync::Arc<tokio::sync::Mutex<BrowserExtension>>,
) -> Result<warp::reply::Response, warp::Rejection> {
    // log::info!("health check: ok");
    let mut browser_ext = browser_ext.lock().await;
    browser_ext.last_health_check = chrono::Utc::now();
    browser_ext.version = Some(version);
    Ok(
        warp::reply::with_status(env!("CARGO_PKG_VERSION").to_owned(), StatusCode::OK)
            .into_response(),
    )
}

fn with_jar(
    jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>,
) -> impl Filter<
    Extract = (std::sync::Arc<tokio::sync::Mutex<CookieJar>>,),
    Error = std::convert::Infallible,
> + Clone {
    warp::any().map(move || jar.clone())
}

fn with_browser_extension(
    browser_ext: std::sync::Arc<tokio::sync::Mutex<BrowserExtension>>,
) -> impl Filter<
    Extract = (std::sync::Arc<tokio::sync::Mutex<BrowserExtension>>,),
    Error = std::convert::Infallible,
> + Clone {
    warp::any().map(move || browser_ext.clone())
}

pub async fn serve(
    jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>,
    browser_ext: std::sync::Arc<tokio::sync::Mutex<BrowserExtension>>,
) {
    warp::serve(
        warp::put()
            .and(warp::path("cookies"))
            .and(warp::body::json())
            .and(with_jar(jar.clone()))
            .and_then(put_cookie)
            .or(warp::delete()
                .and(warp::path("cookies"))
                .and(warp::path::param::<String>())
                .and(with_jar(jar.clone()))
                .and_then(delete_cookie))
            .or(warp::post()
                .and(warp::path("health"))
                .and(warp::body::json())
                .and(with_browser_extension(browser_ext.clone()))
                .and_then(health)),
    )
    .run(([127, 0, 0, 1], 6891))
    .await;
}
