use axum::{routing::get, Router};
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let app = Router::new()
        .route(
            "/link",
            get(nm::link::get)
                .delete(nm::link::delete)
                .post(nm::link::add)
                .put(nm::link::change),
        )
        .route(
            "/route",
            get(nm::route::get)
                .delete(nm::route::delete)
                .post(nm::route::add),
        )
        .route(
            "/address",
            get(nm::address::get)
                .delete(nm::address::delete)
                .post(nm::address::add),
        )
        .route("/check", get(nm::check::check))
        .route_layer(axum::middleware::from_fn(nm::netlink))
        .layer(CorsLayer::very_permissive());

    let port = std::option_env!("PORT").unwrap_or("3000");
    let addr = SocketAddr::from(([127, 0, 0, 1], port.parse().unwrap()));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
