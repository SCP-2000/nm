use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use futures::{stream::TryStreamExt, StreamExt, TryStream};
use rtnetlink::{new_connection, packet::RouteMessage, Handle, IpVersion};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;

#[derive(Debug, Serialize)]
struct Route {
    pub address_family: u8,
    pub destination_prefix_length: u8,
    pub source_prefix_length: u8,
    pub tos: u8,
    pub table: u8,
    pub protocol: u8,
    pub scope: u8,
    pub kind: u8,
}

impl From<RouteMessage> for Route {
    fn from(route: RouteMessage) -> Self {
        Self {
            address_family: route.header.address_family,
            destination_prefix_length: route.header.destination_prefix_length,
            source_prefix_length: route.header.source_prefix_length,
            tos: route.header.tos,
            table: route.header.table,
            protocol: route.header.protocol,
            scope: route.header.scope,
            kind: route.header.kind,
        }
    }
}

async fn routes() -> Json<Vec<Route>> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);
    let routes: Vec<Route> = handle
        .route()
        .get(IpVersion::V4)
        .execute()
        .map_ok(Route::from)
        .try_collect()
        .await
        .unwrap();
    Json(routes)
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let app = Router::new().route("/routes", get(routes));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
