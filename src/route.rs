use crate::util::{addr_to_octets, octets_to_addr};
use axum::{extract, http::StatusCode, Extension, Json};
use futures::stream::TryStreamExt;
use rtnetlink::packet::nlas::route::Nla;
use rtnetlink::packet::{AF_INET, AF_INET6};
use rtnetlink::Handle;
use rtnetlink::{packet::RouteMessage, IpVersion};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Route {
    pub family: u8,
    pub table: u8,
    pub scope: u8,
    pub proto: u8,

    pub dst: Option<(IpAddr, u8)>,
    pub src: Option<(IpAddr, u8)>,
    pub gateway: Option<IpAddr>,
    pub dev: Option<u32>,
    pub prefsrc: Option<IpAddr>,
    pub metric: Option<u32>,
}

fn push_nlas(route: &Route, nlas: &mut Vec<Nla>) {
    if let Some(dst) = route.dst {
        // WARN: also set destination_prefix_length
        nlas.push(Nla::Destination(addr_to_octets(dst.0)));
    }
    if let Some(src) = route.src {
        // WARN: also set source_prefix_length
        nlas.push(Nla::Source(addr_to_octets(src.0)));
    }
    if let Some(gateway) = route.gateway {
        nlas.push(Nla::Gateway(addr_to_octets(gateway)));
    }
    if let Some(dev) = route.dev {
        nlas.push(Nla::Oif(dev));
    }
    if let Some(prefsrc) = route.prefsrc {
        nlas.push(Nla::PrefSource(addr_to_octets(prefsrc)));
    }
    if let Some(metric) = route.metric {
        nlas.push(Nla::Priority(metric));
    }
}

impl From<Route> for RouteMessage {
    fn from(route: Route) -> Self {
        let mut msg = Self::default();
        msg.header.address_family = route.family;
        msg.header.table = route.table;
        msg.header.scope = route.scope;
        msg.header.protocol = route.proto;
        push_nlas(&route, &mut msg.nlas);
        if let Some(dst) = route.dst {
            msg.header.destination_prefix_length = dst.1;
        }
        if let Some(src) = route.src {
            msg.header.source_prefix_length = src.1;
        }
        msg
    }
}

impl From<RouteMessage> for Route {
    fn from(msg: RouteMessage) -> Self {
        Self {
            family: msg.header.address_family,
            table: msg.header.table,
            scope: msg.header.scope,
            dst: msg.destination_prefix(),
            src: msg.source_prefix(),
            gateway: msg.gateway(),
            dev: msg.output_interface(),
            proto: msg.header.protocol,
            prefsrc: msg.nlas.iter().find_map(|nla| {
                if let Nla::PrefSource(prefsrc) = nla {
                    Some(octets_to_addr(prefsrc))
                } else {
                    None
                }
            }),
            metric: msg.nlas.iter().find_map(|nla| {
                if let Nla::Priority(metric) = nla {
                    Some(*metric)
                } else {
                    None
                }
            }),
        }
    }
}

pub async fn add(
    Extension(handle): Extension<Handle>,
    extract::Json(payload): extract::Json<Route>,
) -> Result<(), (StatusCode, String)> {
    let mut req = handle.route().add();
    req = req.table(payload.table);
    req = req.scope(payload.scope);
    req = req.protocol(payload.proto);
    push_nlas(&payload, &mut req.message_mut().nlas);
    if let Some(dst) = payload.dst {
        req.message_mut().header.destination_prefix_length = dst.1;
    }
    if let Some(src) = payload.src {
        req.message_mut().header.source_prefix_length = src.1;
    }
    let res = match payload.family as u16 {
        AF_INET => req.v4().execute().await,
        AF_INET6 => req.v6().execute().await,
        _ => unreachable!(),
    };
    res.map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

pub async fn delete(
    Extension(handle): Extension<Handle>,
    extract::Json(payload): extract::Json<Route>,
) -> Result<(), (StatusCode, String)> {
    handle
        .route()
        .del(payload.into())
        .execute()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

pub async fn get(Extension(handle): Extension<Handle>) -> Json<Vec<Route>> {
    let v4 = handle.route().get(IpVersion::V4).execute();
    let v4: Vec<Route> = v4
        .map_ok(Route::from)
        .try_filter(|route| futures::future::ready(route.table == 254))
        .try_collect()
        .await
        .unwrap();
    let v6 = handle.route().get(IpVersion::V6).execute();
    let v6: Vec<Route> = v6
        .map_ok(Route::from)
        .try_filter(|route| futures::future::ready(route.table == 254))
        .try_collect()
        .await
        .unwrap();
    let routes = [v4, v6].concat();
    Json(routes)
}
