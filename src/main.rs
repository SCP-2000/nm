use axum::{routing::get, Router};
use std::net::SocketAddr;

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
        .route_layer(axum::middleware::from_fn(nm::netlink));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
