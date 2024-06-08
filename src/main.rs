use anyhow::Result;
use delete_ecs_clusters_rs::run;
use std::env::set_var;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    set_env_vars();

    if let Err(e) = run().await {
        log::error!("Application error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn set_env_vars() {
    set_var(
        "AWS_ACCESS_KEY_ID",
        dotenvy::var("AWS_ACCESS_KEY_ID").unwrap(),
    );
    set_var(
        "AWS_SECRET_ACCESS_KEY",
        dotenvy::var("AWS_SECRET_ACCESS_KEY").unwrap(),
    );
    set_var("AWS_REGION", dotenvy::var("AWS_REGION").unwrap());
}
