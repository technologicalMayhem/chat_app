use eyre::Result;
use warp::Filter;

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let echo_get = warp::path("echo")
        .and(warp::get())
        .map(|| "You used Get".to_string());
    let echo_post = warp::path("echo")
        .and(warp::post())
        .map(|| "You used Post".to_string());

    let routes = echo_get.or(echo_post);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}