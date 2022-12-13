use crate::Error;
use axum::{
    extract::{self, Path},
    Extension, Json,
};
use futures::stream::TryStreamExt;
use rtnetlink::packet::nlas::link::{Info, InfoKind, Nla};
use rtnetlink::packet::LinkMessage;
use rtnetlink::Handle;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Kind {
    Dummy,
    Ifb,
    Bridge,
    Tun,
    Vrf,
    Wireguard,
    Other,
}

impl From<InfoKind> for Kind {
    fn from(kind: InfoKind) -> Self {
        match kind {
            InfoKind::Dummy => Self::Dummy,
            InfoKind::Ifb => Self::Ifb,
            InfoKind::Bridge => Self::Bridge,
            InfoKind::Tun => Self::Tun,
            InfoKind::Vrf => Self::Vrf,
            InfoKind::Wireguard => Self::Wireguard,
            _ => Self::Other,
        }
    }
}

impl From<&Kind> for InfoKind {
    fn from(kind: &Kind) -> Self {
        match kind {
            Kind::Dummy => InfoKind::Dummy,
            Kind::Ifb => InfoKind::Ifb,
            Kind::Bridge => InfoKind::Bridge,
            Kind::Tun => InfoKind::Tun,
            Kind::Vrf => InfoKind::Vrf,
            Kind::Wireguard => InfoKind::Wireguard,
            _ => unimplemented!(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Link {
    pub family: u8,
    pub index: u32,
    pub linklayer: u16,
    pub flags: u32,

    pub ifname: Option<String>,
    pub mtu: Option<u32>,
    pub kind: Option<Kind>,
}

fn push_nlas(link: &Link, nlas: &mut Vec<Nla>) {
    if let Some(ifname) = &link.ifname {
        nlas.push(Nla::IfName(ifname.to_string()));
    }
    if let Some(mtu) = link.mtu {
        nlas.push(Nla::Mtu(mtu));
    }
    if let Some(kind) = &link.kind {
        nlas.push(Nla::Info(vec![Info::Kind(kind.into())]));
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
                Nla::Info(info) => {
                    for i in info {
                        match i {
                            Info::Kind(kind) => link.kind = Some(kind.into()),
                            _ => continue,
                        }
                    }
                }
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

pub async fn add(
    Extension(handle): Extension<Handle>,
    extract::Json(payload): extract::Json<Link>,
) -> Result<(), Error> {
    let mut req = handle.link().add();
    req.message_mut().header.interface_family = payload.family;
    req.message_mut().header.index = payload.index;
    req.message_mut().header.link_layer_type = payload.linklayer;
    req.message_mut().header.flags = payload.flags;
    push_nlas(&payload, &mut req.message_mut().nlas);
    Ok(req.execute().await?)
}

pub async fn delete(
    Extension(handle): Extension<Handle>,
    extract::Json(payload): extract::Json<Link>,
) -> Result<(), Error> {
    Ok(handle.link().del(payload.index).execute().await?)
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
