use axum::{
    extract::{self, Path},
    http::StatusCode,
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

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct Link {
    pub interface_family: u8,
    pub index: u32,
    pub link_layer_type: u16,
    pub flags: u32,

    pub ifname: Option<String>,
    pub mtu: Option<u32>,
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

pub async fn change(
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

pub async fn get(Extension(handle): Extension<Handle>) -> Json<Vec<Link>> {
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
