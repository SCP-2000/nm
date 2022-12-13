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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Route {
    pub family: u8,
    pub table: u8,
    pub scope: u8,
    pub dst: Option<(IpAddr, u8)>,
    pub src: Option<(IpAddr, u8)>,
    pub gateway: Option<IpAddr>,
    pub dev: Option<u32>,
    pub proto: u8,
    pub prefsrc: Option<IpAddr>,
    pub metric: Option<u32>,
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

impl Into<RouteMessage> for Route {
    fn into(self) -> RouteMessage {
        let mut route = RouteMessage::default();
        route.header.address_family = self.family;
        route.header.table = self.table;
        route.header.scope = self.scope;
        route.header.protocol = self.proto;
        if let Some(dst) = self.dst {
            route.header.destination_prefix_length = dst.1;
            route
                .nlas
                .push(RouteNla::Destination(addr_to_octets(dst.0)));
        }
        if let Some(src) = self.src {
            route.header.source_prefix_length = src.1;
            route.nlas.push(RouteNla::Source(addr_to_octets(src.0)));
        }
        if let Some(gateway) = self.gateway {
            route.nlas.push(RouteNla::Gateway(addr_to_octets(gateway)));
        }
        if let Some(dev) = self.dev {
            route.nlas.push(RouteNla::Oif(dev));
        }
        if let Some(prefsrc) = self.prefsrc {
            route
                .nlas
                .push(RouteNla::PrefSource(addr_to_octets(prefsrc)));
        }
        if let Some(metric) = self.metric {
            route.nlas.push(RouteNla::Priority(metric));
        }
        route
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

pub async fn add(
    Extension(handle): Extension<Handle>,
    extract::Json(payload): extract::Json<Route>,
) -> Result<(), (StatusCode, String)> {
    let mut req = handle.route().add();
    req = req.table(payload.table);
    req = req.scope(payload.scope);
    req = req.protocol(payload.proto);
    if let Some(dst) = payload.dst {
        req.message_mut().header.destination_prefix_length = dst.1;
        req.message_mut()
            .nlas
            .push(RouteNla::Destination(addr_to_octets(dst.0)));
    }
    if let Some(src) = payload.src {
        req.message_mut().header.source_prefix_length = src.1;
        req.message_mut()
            .nlas
            .push(RouteNla::Source(addr_to_octets(src.0)));
    }
    if let Some(gateway) = payload.gateway {
        req.message_mut()
            .nlas
            .push(RouteNla::Gateway(addr_to_octets(gateway)));
    }
    if let Some(dev) = payload.dev {
        req.message_mut().nlas.push(RouteNla::Oif(dev));
    }
    if let Some(prefsrc) = payload.prefsrc {
        req.message_mut()
            .nlas
            .push(RouteNla::PrefSource(addr_to_octets(prefsrc)));
    }
    if let Some(metric) = payload.metric {
        req.message_mut().nlas.push(RouteNla::Priority(metric));
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