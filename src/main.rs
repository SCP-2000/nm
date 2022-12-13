use axum::{
    extract::{self, Path},
    http::StatusCode,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use futures::stream::TryStreamExt;
use nm::link::Link;
use nm::route::Route;
use rtnetlink::packet::nlas::link::Nla as LinkNla;
use rtnetlink::packet::nlas::route::Nla as RouteNla;
use rtnetlink::packet::{AddressMessage, LinkMessage};
use rtnetlink::packet::{AF_INET, AF_INET6};
use rtnetlink::{packet::nlas::address::Nla as AddrNla, Handle};
use rtnetlink::{packet::RouteMessage, IpVersion};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};


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
