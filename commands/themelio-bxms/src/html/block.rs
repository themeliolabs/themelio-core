use crate::{to_badgateway, to_badreq};
use askama::Template;
use blkstructs::Header;
use nodeprot::ValClient;

use super::RenderTimeTracer;

#[derive(Template)]
#[template(path = "block.html")]
struct BlockTemplate {
    header: Header,
    txcount: usize,
    txweight: u128,
}

#[tracing::instrument(skip(req))]
pub async fn get_blockpage(req: tide::Request<ValClient>) -> tide::Result<tide::Body> {
    let _render = RenderTimeTracer::new("blockpage");

    let height: u64 = req.param("height").unwrap().parse().map_err(to_badreq)?;
    let last_snap = req.state().snapshot().await.map_err(to_badgateway)?;
    let block = last_snap
        .get_older(height)
        .await
        .map_err(to_badgateway)?
        .current_block()
        .await?;

    let mut body: tide::Body = BlockTemplate {
        header: block.header,
        txcount: block.transactions.len(),
        txweight: block.transactions.iter().map(|v| v.weight()).sum(),
    }
    .render()
    .unwrap()
    .into();
    body.set_mime("text/html");
    Ok(body)
}