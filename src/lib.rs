use axum::{
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

pub mod link;
pub mod route;
pub mod address;

pub async fn netlink<B>(
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    if let Ok((conn, handle, _)) = rtnetlink::new_connection() {
        tokio::spawn(conn);
        req.extensions_mut().insert(handle);
        Ok(next.run(req).await)
    } else {
        Err(
            StatusCode::INTERNAL_SERVER_ERROR,
        )
    }
}
