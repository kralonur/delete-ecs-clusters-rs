use anyhow::Result;
use clap::{Parser, ValueEnum};
use delete_ecs_clusters_rs::{run, run_task_definitions};
use std::env::set_var;

#[derive(Debug, Clone, ValueEnum)] // ArgEnum here
enum RunType {
    Cluster,
    TaskDefinition,
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    #[arg(short, long)]
    run_type: RunType,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    set_env_vars();

    let run_type = match args.run_type {
        RunType::Cluster => run().await,
        RunType::TaskDefinition => run_task_definitions().await,
    };

    if let Err(e) = run_type {
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
