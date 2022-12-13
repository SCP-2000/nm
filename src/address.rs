use crate::util::{addr_to_octets, octets_to_addr};
use axum::{Extension, Json};
use futures::stream::TryStreamExt;
use rtnetlink::packet::AddressMessage;
use rtnetlink::{packet::nlas::address::Nla, Handle};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Address {
    pub family: u8,
    pub plen: u8,
    pub flags: u8,
    pub scope: u8,
    pub index: u32,

    pub address: Option<IpAddr>,
    pub local: Option<IpAddr>,
    pub label: Option<String>,
    pub broadcast: Option<IpAddr>,
}

fn push_nlas(addr: &Address, nlas: &mut Vec<Nla>) {
    if let Some(address) = addr.address {
        nlas.push(Nla::Address(addr_to_octets(address)));
    }
    if let Some(local) = addr.address {
        nlas.push(Nla::Local(addr_to_octets(local)));
    }
    if let Some(label) = &addr.label {
        nlas.push(Nla::Label(label.to_string()));
    }
    if let Some(broadcast) = addr.broadcast {
        nlas.push(Nla::Broadcast(addr_to_octets(broadcast)));
    }
}

impl From<Address> for AddressMessage {
    fn from(addr: Address) -> Self {
        let mut msg = Self::default();
        msg.header.family = addr.family;
        msg.header.prefix_len = addr.plen;
        msg.header.flags = addr.flags;
        msg.header.scope = addr.scope;
        msg.header.index = addr.index;
        push_nlas(&addr, &mut msg.nlas);
        msg
    }
}

impl From<AddressMessage> for Address {
    fn from(msg: AddressMessage) -> Self {
        let mut address = Self {
            family: msg.header.family,
            plen: msg.header.prefix_len,
            flags: msg.header.flags,
            scope: msg.header.scope,
            index: msg.header.index,
            ..Default::default()
        };
        for nla in msg.nlas {
            match nla {
                Nla::Address(addr) => address.address = Some(octets_to_addr(&addr)),
                Nla::Local(local) => address.local = Some(octets_to_addr(&local)),
                Nla::Label(label) => address.label = Some(label),
                Nla::Broadcast(broadcast) => address.broadcast = Some(octets_to_addr(&broadcast)),
                _ => continue,
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
