use crate::address::Address;
use crate::link::Link;
use crate::Error;
use axum::{extract, Extension, Json};
use futures::future::ready;
use futures::stream::TryStreamExt;
use futures::StreamExt;
use rtnetlink::packet::nlas::link::{Info, InfoKind, Nla};
use rtnetlink::packet::{LinkMessage, IFF_BROADCAST, IFF_LOWER_UP, IFF_MULTICAST, IFF_UP};
use rtnetlink::Handle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Check {
    link: Option<Link>,
    addr: Option<Vec<Address>>,
}

pub async fn check(Extension(handle): Extension<Handle>) -> Result<Json<Check>, Error> {
    let link: Option<Link> = handle
        .link()
        .get()
        .execute()
        .map_ok(Link::from)
        .try_filter(|l| {
            ready(
                l.linklayer == 1
                    && (l.flags & IFF_LOWER_UP != 0)
                    && (l.flags & IFF_UP != 0)
                    && (l.flags & IFF_BROADCAST != 0)
                    && (l.flags & IFF_MULTICAST != 0),
            )
        })
        .try_next()
        .await?;
    let addr: Option<Vec<Address>> = if let Some(ref link) = link {
        Some(
            handle
                .address()
                .get()
                .set_link_index_filter(link.index)
                .execute()
                .map_ok(Address::from)
                .try_filter(|a| ready(a.scope == 0))
                .try_collect()
                .await?,
        )
    } else {
        None
    };
    Ok(Json(Check { link, addr }))
}
