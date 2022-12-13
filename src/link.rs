use crate::Error;
use axum::{
    extract::{self, Path},
    Extension, Json,
};
use futures::stream::TryStreamExt;
use rtnetlink::packet::nlas::link::Nla;
use rtnetlink::packet::LinkMessage;
use rtnetlink::Handle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Link {
    pub family: u8,
    pub index: u32,
    pub linklayer: u16,
    pub flags: u32,

    pub ifname: Option<String>,
    pub mtu: Option<u32>,
}

fn push_nlas(link: &Link, nlas: &mut Vec<Nla>) {
    if let Some(ifname) = &link.ifname {
        nlas.push(Nla::IfName(ifname.to_string()));
    }
    if let Some(mtu) = link.mtu {
        nlas.push(Nla::Mtu(mtu));
    }
}

impl From<Link> for LinkMessage {
    fn from(link: Link) -> Self {
        let mut msg = Self::default();
        msg.header.interface_family = link.family;
        msg.header.index = link.index;
        msg.header.link_layer_type = link.linklayer;
        msg.header.flags = link.flags;
        push_nlas(&link, &mut msg.nlas);
        msg
    }
}

impl From<LinkMessage> for Link {
    fn from(msg: LinkMessage) -> Self {
        let mut link = Self {
            family: msg.header.interface_family,
            index: msg.header.index,
            linklayer: msg.header.link_layer_type,
            flags: msg.header.flags,
            ..Default::default()
        };
        for nla in msg.nlas {
            match nla {
                Nla::IfName(ifname) => link.ifname = Some(ifname),
                Nla::Mtu(mtu) => link.mtu = Some(mtu),
                _ => continue,
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
    if let Some(ref _ifname) = payload.ifname {
        // req = req.name(ifname.clone());
    }
    if let Some(mtu) = payload.mtu {
        req = req.mtu(mtu);
    }
    req.execute().await.unwrap();
    Json(payload)
}

pub async fn get(Extension(handle): Extension<Handle>) -> Result<Json<Vec<Link>>, Error> {
    let links: Vec<Link> = handle
        .link()
        .get()
        .execute()
        .map_ok(Link::from)
        .try_collect()
        .await?;
    Ok(Json(links))
}
