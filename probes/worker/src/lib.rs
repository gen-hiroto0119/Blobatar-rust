#[cfg(any(target_arch = "wasm32", test))]
mod response;

#[cfg(target_arch = "wasm32")]
use worker::{Context, Env, Request, Response, Result, event};

#[cfg(target_arch = "wasm32")]
#[event(fetch)]
async fn fetch(request: Request, _env: Env, _context: Context) -> Result<Response> {
    let reply = response::handle(request.method().as_ref(), &request.url()?);
    let mut response = Response::ok(reply.body)?.with_status(reply.status);
    let headers = response.headers_mut();
    headers.set("Content-Type", reply.content_type)?;
    headers.set("Cache-Control", "no-store")?;
    headers.set("X-Content-Type-Options", "nosniff")?;
    if reply.status == 405 {
        headers.set("Allow", "GET, HEAD")?;
    }
    Ok(response)
}
