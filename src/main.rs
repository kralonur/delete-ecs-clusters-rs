use anyhow::Result;
use clap::{Parser, ValueEnum};
use delete_ecs_clusters_rs::{
    run_delete_clusters, run_delete_task_definitions, run_deregister_task_definitions,
    MultiRegionOption,
};
use std::env::set_var;

#[derive(Debug, Clone, ValueEnum)] // ArgEnum here
enum RunType {
    DeleteClusterSingle,
    DeleteClusterMultiple,
    DeregisterTaskDefinitionSingle,
    DeregisterTaskDefinitionMultiple,
    DeleteTaskDefinitionSingle,
    DeleteTaskDefinitionMultiple,
}

/// A simple program to delete clusters on ECS and deregister task definitions
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    #[arg(short, long, value_enum, default_value_t = RunType::DeleteClusterSingle)]
    run_type: RunType,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();

    set_env_vars();

    let run_type = match args.run_type {
        RunType::DeleteClusterSingle => run_delete_clusters(MultiRegionOption::Single).await,
        RunType::DeleteClusterMultiple => run_delete_clusters(MultiRegionOption::Multiple).await,
        RunType::DeregisterTaskDefinitionSingle => {
            run_deregister_task_definitions(MultiRegionOption::Single).await
        }
        RunType::DeregisterTaskDefinitionMultiple => {
            run_deregister_task_definitions(MultiRegionOption::Multiple).await
        }
        RunType::DeleteTaskDefinitionSingle => {
            run_delete_task_definitions(MultiRegionOption::Single).await
        }
        RunType::DeleteTaskDefinitionMultiple => {
            run_delete_task_definitions(MultiRegionOption::Multiple).await
        }
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
