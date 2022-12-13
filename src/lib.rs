use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use thiserror::Error;

pub mod address;
pub mod link;
pub mod route;
pub mod util;

pub async fn netlink<B>(
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, &'static str)> {
    if let Ok((conn, handle, _)) = rtnetlink::new_connection() {
        tokio::spawn(conn);
        req.extensions_mut().insert(handle);
        Ok(next.run(req).await)
    } else {
        Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            "failed to get netlink handle",
        ))
    }
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("netlink error")]
    Netlink(#[from] rtnetlink::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
