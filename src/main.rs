use axum::{
    extract::{self, Path},
    http::StatusCode,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use futures::stream::TryStreamExt;
use rtnetlink::packet::nlas::link::Nla as LinkNla;
use rtnetlink::packet::nlas::route::Nla as RouteNla;
use rtnetlink::packet::{AddressMessage, LinkMessage};
use rtnetlink::packet::{AF_INET, AF_INET6};
use rtnetlink::{packet::nlas::address::Nla as AddrNla, Handle};
use rtnetlink::{packet::RouteMessage, IpVersion};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use nm::route::Route;

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

fn addr_to_octets(addr: IpAddr) -> Vec<u8> {
    match addr {
        IpAddr::V4(addr) => addr.octets().to_vec(),
        IpAddr::V6(addr) => addr.octets().to_vec(),
    }
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

async fn link(
    Extension(handle): Extension<Handle>,
    Path(index): Path<u32>,
    extract::Json(payload): extract::Json<Link>,
) -> Json<Link> {
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

async fn links(Extension(handle): Extension<Handle>) -> Json<Vec<Link>> {
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

async fn addresses(Extension(handle): Extension<Handle>) -> Json<Vec<Address>> {
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

#[tokio::main]
async fn main() -> Result<(), ()> {
    let app = Router::new()
        .route("/links", get(links))
        .route("/links/:index", put(link))
        .route(
            "/routes",
            get(nm::route::get)
                .delete(nm::route::delete)
                .post(nm::route::add),
        )
        .route("/addresses", get(addresses))
        .route_layer(axum::middleware::from_fn(nm::netlink));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}
