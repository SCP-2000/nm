use axum::{
    extract::{self, Path},
    routing::{get, put},
    Json, Router,
};
use futures::{stream::TryStreamExt, StreamExt};
use rtnetlink::packet::nlas::address::Nla as AddrNla;
use rtnetlink::packet::nlas::link::Nla as LinkNla;
use rtnetlink::packet::nlas::route::Nla as RouteNla;
use rtnetlink::packet::{AddressMessage, LinkMessage};
use rtnetlink::packet::{AF_INET, AF_INET6};
use rtnetlink::{new_connection, packet::RouteMessage, IpVersion};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

#[derive(Debug, Serialize, Default, Deserialize)]
struct Link {
    pub interface_family: u8,
    pub index: u32,
    pub link_layer_type: u16,
    pub flags: u32,

    pub ifname: Option<String>,
    pub mtu: Option<u32>,
}

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

#[derive(Debug, Serialize, Clone)]
struct Route {
    pub table: Option<u32>,
    pub dst: Option<(IpAddr, u8)>,
    pub src: Option<(IpAddr, u8)>,
    pub gateway: Option<IpAddr>,
    pub dev: Option<u32>,
    pub proto: u8,
    pub prefsrc: Option<IpAddr>,
    pub metric: Option<u32>,
}

fn octets_to_addr(octets: &[u8]) -> IpAddr {
    if octets.len() == 4 {
        let mut ary: [u8; 4] = Default::default();
        ary.copy_from_slice(octets);
        IpAddr::from(ary)
    } else if octets.len() == 16 {
        let mut ary: [u8; 16] = Default::default();
        ary.copy_from_slice(octets);
        IpAddr::from(ary)
    } else {
        unreachable!()
    }
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

impl From<LinkMessage> for Link {
    fn from(msg: LinkMessage) -> Self {
        let mut link = Self {
            interface_family: msg.header.interface_family,
            index: msg.header.index,
            link_layer_type: msg.header.link_layer_type,
            flags: msg.header.flags,
            ..Default::default()
        };
        for nla in msg.nlas {
            match nla {
                LinkNla::IfName(ifname) => link.ifname = Some(ifname),
                LinkNla::Mtu(mtu) => link.mtu = Some(mtu),
                nla => log::debug!("ignored unsupported link nla: {:?}", nla),
            }
        }
        link
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
        Self {
            table: msg.nlas.iter().find_map(|nla| {
                if let RouteNla::Table(table) = nla {
                    Some(*table)
                } else {
                    None
                }
            }),
            dst: msg.destination_prefix(),
            src: msg.source_prefix(),
            gateway: msg.gateway(),
            dev: msg.output_interface(),
            proto: msg.header.protocol,
            prefsrc: msg.nlas.iter().find_map(|nla| {
                if let RouteNla::PrefSource(prefsrc) = nla {
                    Some(octets_to_addr(prefsrc))
                } else {
                    None
                }
            }),
            metric: msg.nlas.iter().find_map(|nla| {
                if let RouteNla::Priority(metric) = nla {
                    Some(*metric)
                } else {
                    None
                }
            }),
        }
    }
}

async fn link(Path(index): Path<u32>, extract::Json(payload): extract::Json<Link>) -> Json<Link> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);
    let mut req = handle.link().set(index);
    if let Some(ref ifname) = payload.ifname {
        // req = req.name(ifname.clone());
    }
    if let Some(mtu) = payload.mtu {
        req = req.mtu(mtu);
    }
    req.execute().await.unwrap();
    Json(payload)
}

async fn links() -> Json<Vec<Link>> {
    let (connection, handle, _) = new_connection().unwrap();
    tokio::spawn(connection);
    let links: Vec<Link> = handle
        .link()
        .get()
        .execute()
        .map_ok(Link::from)
        .try_collect()
        .await
        .unwrap();
    Json(links)
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
        .route("/links", get(links))
        .route("/links/:index", put(link))
        .route("/routes", get(routes))
        .route("/addresses", get(addresses));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
