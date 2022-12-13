use crate::util::octets_to_addr;
use axum::{Extension, Json};
use futures::stream::TryStreamExt;
use rtnetlink::packet::AddressMessage;
use rtnetlink::{packet::nlas::address::Nla as AddrNla, Handle};
use serde::Serialize;
use std::net::IpAddr;

#[derive(Debug, Serialize, Default)]
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
                AddrNla::Address(addr) => address.address = Some(octets_to_addr(&addr)),
                AddrNla::Local(local) => address.local = Some(octets_to_addr(&local)),
                AddrNla::Label(label) => address.label = Some(label),
                AddrNla::Broadcast(broadcast) => {
                    address.broadcast = Some(octets_to_addr(&broadcast))
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
