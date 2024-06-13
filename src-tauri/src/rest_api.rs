use warp::Filter;
use warp::{self, http::StatusCode};
async fn put_cookie(
    name: String,
    value: String,
) -> std::result::Result<StatusCode, warp::Rejection> {
    log::info!("Storing {name} = {value}");
    Ok(StatusCode::OK)
}

async fn delete_cookie(name: String) -> std::result::Result<StatusCode, warp::Rejection> {
    log::info!("Delete {name}");
    Ok(StatusCode::OK)
}
async fn health() -> std::result::Result<StatusCode, warp::Rejection> {
    log::info!("health check: ok");
    Ok(StatusCode::OK)
}

pub async fn serve() {
    warp::serve(
        warp::put()
            .and(warp::path("cookies"))
            .and(warp::path::param::<String>())
            .and(warp::body::json())
            .and_then(put_cookie)
            .or(warp::delete()
                .and(warp::path("cookies"))
                .and(warp::path::param::<String>())
                .and_then(delete_cookie))
            .or(warp::delete().and(warp::path("health")).and_then(health)),
    )
    .run(([127, 0, 0, 1], 6891))
    .await;
}
