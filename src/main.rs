use axum::{
    routing::{get, put},
    Router,
};

use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), ()> {
    let app = Router::new()
        .route("/links", get(nm::link::get))
        .route("/links/:index", put(nm::link::change))
        .route(
            "/routes",
            get(nm::route::get)
                .delete(nm::route::delete)
                .post(nm::route::add),
        )
        .route("/addresses", get(nm::address::get))
        .route_layer(axum::middleware::from_fn(nm::netlink));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
