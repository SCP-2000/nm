use axum::extract::Path;
use axum::{routing::get, Json, Router};
use futures::stream::TryStreamExt;
use rtnetlink::packet::{nlas::route::Nla, AF_INET, AF_INET6};
use rtnetlink::{new_connection, packet::RouteMessage, IpVersion};
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Serialize, Default)]
struct Route {
    pub address_family: u8,
    pub destination_prefix_length: u8,
    pub source_prefix_length: u8,
    pub tos: u8,
    pub table: u8,
    pub protocol: u8,
    pub scope: u8,
    pub kind: u8,
    // TODO: flags
    pub destination: Option<IpAddr>,
    pub gateway: Option<IpAddr>,
    pub prefsrc: Option<IpAddr>,
    pub oif: Option<u32>,
}

fn address(family: u8, addr: Vec<u8>) -> IpAddr {
    match family as u16 {
        AF_INET => {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&addr);
            IpAddr::V4(Ipv4Addr::from(buf))
        }
        AF_INET6 => {
            let mut buf = [0u8; 16];
            buf.copy_from_slice(&addr);
            IpAddr::V6(Ipv6Addr::from(buf))
        }
        _ => unreachable!(),
    }
}

impl From<RouteMessage> for Route {
    fn from(msg: RouteMessage) -> Self {
        let mut route = Self {
            address_family: msg.header.address_family,
            destination_prefix_length: msg.header.destination_prefix_length,
            source_prefix_length: msg.header.source_prefix_length,
            tos: msg.header.tos,
            table: msg.header.table,
            protocol: msg.header.protocol,
            scope: msg.header.scope,
            kind: msg.header.kind,
            ..Default::default()
        };
        for nla in msg.nlas {
            match nla {
                Nla::Destination(dst) => {
                    route.destination = Some(address(msg.header.address_family, dst));
                }
                Nla::Gateway(gateway) => {
                    route.gateway = Some(address(msg.header.address_family, gateway));
                }
                Nla::PrefSource(prefsrc) => {
                    route.gateway = Some(address(msg.header.address_family, prefsrc));
                }
                Nla::Oif(oif) => {
                    route.oif = Some(oif);
                }
                nla => log::debug!("ignored unsupported nla: {:?}", nla),
            }
        }
        route
    }
}

async fn routes(Path(af): Path<String>) -> Json<Vec<Route>> {
    let af = match af.as_str() {
        "inet" => IpVersion::V4,
        "inet6" => IpVersion::V6,
        _ => unreachable!(),
    };
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);
    let routes: Vec<Route> = handle
        .route()
        .get(af)
        .execute()
        .map_ok(Route::from)
        .try_collect()
        .await
        .unwrap();
    Json(routes)
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let app = Router::new().route("/routes/:af", get(routes));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
