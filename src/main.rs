use axum::{routing::get, Json, Router};
use futures::stream::TryStreamExt;
use rtnetlink::packet::nlas::address::Nla as AddrNla;
use rtnetlink::packet::nlas::route::Nla as RouteNla;
use rtnetlink::packet::AddressMessage;
use rtnetlink::packet::{AF_INET, AF_INET6};
use rtnetlink::{new_connection, packet::RouteMessage, IpVersion};
use serde::Serialize;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Serialize, Default)]
struct Address {
    pub family: u8,
    pub prefix_len: u8,
    pub flags: u8,
    pub scope: u8,
    pub index: u32,

    pub address: Option<IpAddr>,
    pub local: Option<IpAddr>,
    pub label: Option<String>,
    pub broadcast: Option<IpAddr>,
}

#[derive(Debug, Serialize, Default, Clone)]
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

fn to_address(family: u8, addr: Vec<u8>) -> IpAddr {
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

impl From<AddressMessage> for Address {
    fn from(msg: AddressMessage) -> Self {
        let mut address = Self {
            family: msg.header.family,
            prefix_len: msg.header.prefix_len,
            flags: msg.header.flags,
            scope: msg.header.scope,
            index: msg.header.index,
            ..Default::default()
        };
        for nla in msg.nlas {
            match nla {
                AddrNla::Address(addr) => {
                    address.address = Some(to_address(msg.header.family, addr))
                }
                AddrNla::Local(local) => address.local = Some(to_address(msg.header.family, local)),
                AddrNla::Label(label) => address.label = Some(label),
                AddrNla::Broadcast(broadcast) => {
                    address.broadcast = Some(to_address(msg.header.family, broadcast))
                }
                nla => log::debug!("ignored unsupported address nla: {:?}", nla),
            }
        }
        address
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
                RouteNla::Destination(dst) => {
                    route.destination = Some(to_address(msg.header.address_family, dst));
                }
                RouteNla::Gateway(gateway) => {
                    route.gateway = Some(to_address(msg.header.address_family, gateway));
                }
                RouteNla::PrefSource(prefsrc) => {
                    route.gateway = Some(to_address(msg.header.address_family, prefsrc));
                }
                RouteNla::Oif(oif) => {
                    route.oif = Some(oif);
                }
                nla => log::debug!("ignored unsupported route nla: {:?}", nla),
            }
        }
        route
    }
}

async fn addresses() -> Json<Vec<Address>> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);
    let addresses: Vec<Address> = handle
        .address()
        .get()
        .execute()
        .map_ok(Address::from)
        .try_collect()
        .await
        .unwrap();
    Json(addresses)
}

async fn routes() -> Json<Vec<Route>> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);
    let v4 = handle.route().get(IpVersion::V4).execute();
    let v4: Vec<Route> = v4.map_ok(Route::from).try_collect().await.unwrap();
    let v6 = handle.route().get(IpVersion::V6).execute();
    let v6: Vec<Route> = v6.map_ok(Route::from).try_collect().await.unwrap();
    let routes = [v4, v6].concat();
    Json(routes)
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let app = Router::new()
        .route("/routes", get(routes))
        .route("/addresses", get(addresses));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
