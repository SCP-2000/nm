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

#[derive(Debug, Serialize, Default)]
pub struct Address {
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

pub async fn get(Extension(handle): Extension<Handle>) -> Json<Vec<Address>> {
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