use anyhow::Result;
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion, Region};
use aws_sdk_ecs::{types::Cluster, types::TaskDefinitionStatus, Client};
use futures::{future::join_all, StreamExt};
use std::fs;

const PROCESS_AMOUNT_AT_ONCE: usize = 5;

pub enum MultiRegionOption {
    Single,
    Multiple,
}

pub async fn run_delete_clusters(region_option: MultiRegionOption) -> Result<()> {
    match region_option {
        MultiRegionOption::Single => delete_clusters_single(get_client(None).await).await,
        MultiRegionOption::Multiple => delete_clusters_multiple(get_clients().await).await,
    }
}

pub async fn run_deregister_task_definitions(region_option: MultiRegionOption) -> Result<()> {
    match region_option {
        MultiRegionOption::Single => {
            deregister_task_definitions_single(get_client(None).await).await
        }
        MultiRegionOption::Multiple => {
            deregister_task_definitions_multiple(get_clients().await).await
        }
    }
}

pub async fn run_delete_task_definitions(region_option: MultiRegionOption) -> Result<()> {
    match region_option {
        MultiRegionOption::Single => delete_task_definitions_single(get_client(None).await).await,
        MultiRegionOption::Multiple => delete_task_definitions_multiple(get_clients().await).await,
    }
}

async fn delete_clusters_single(client: Client) -> Result<()> {
    log::info!(
        "Deleting clusters for region: {}",
        client.config().region().expect("Region not found")
    );

    let list_clusters = client.list_clusters().send().await?;

    let cluster_arns = list_clusters.cluster_arns();
    log::info!("Found {} clusters", cluster_arns.len());

    let clusters = client
        .describe_clusters()
        .set_clusters(Some(cluster_arns.into()))
        .send()
        .await?;

    let delete_clusters_iter = clusters
        .clusters()
        .iter()
        .map(|cluster| delete_cluster(&client, cluster));

    let delete_clusters_stream = futures::stream::iter(delete_clusters_iter);

    delete_clusters_stream
        .for_each_concurrent(PROCESS_AMOUNT_AT_ONCE, |result| async move {
            if let Err(e) = result.await {
                log::error!("Error deleting cluster: {:?}", e);
            }
        })
        .await;

    Ok(())
}

async fn delete_clusters_multiple(clients: Vec<Client>) -> Result<()> {
    for client in clients {
        if let Err(e) = delete_clusters_single(client).await {
            log::error!("Error deleting clusters: {:?}", e);
        }
    }

    Ok(())
}

async fn deregister_task_definitions_multiple(clients: Vec<Client>) -> Result<()> {
    for client in clients {
        if let Err(e) = deregister_task_definitions_single(client).await {
            log::error!("Error running task definitions: {:?}", e);
        }
    }

    Ok(())
}

async fn deregister_task_definitions_single(client: Client) -> Result<()> {
    log::info!(
        "Deregistering task definitions for region: {}",
        client.config().region().expect("Region not found")
    );
    let list_task_definitions = client.list_task_definitions().send().await?;

    let task_definitions = list_task_definitions.task_definition_arns();

    log::info!("Found {} task definitions", task_definitions.len());

    let deregister_task_definition_iter = task_definitions.iter().map(|td| {
        client
            .deregister_task_definition()
            .set_task_definition(Some(td.to_owned()))
            .send()
    });

    let deregister_task_definition_stream = futures::stream::iter(deregister_task_definition_iter);

    deregister_task_definition_stream
        .for_each_concurrent(PROCESS_AMOUNT_AT_ONCE, |result| async move {
            if let Err(e) = result.await {
                log::error!("Error deregistering task definition: {:?}", e);
            }
        })
        .await;

    Ok(())
}

async fn delete_task_definitions_multiple(clients: Vec<Client>) -> Result<()> {
    for client in clients {
        if let Err(e) = delete_task_definitions_single(client).await {
            log::error!("Error running task definitions: {:?}", e);
        }
    }

    Ok(())
}

async fn delete_task_definitions_single(client: Client) -> Result<()> {
    log::info!(
        "Deleting task definitions for region: {}",
        client.config().region().expect("Region not found")
    );

    let list_task_definitions = client
        .list_task_definitions()
        .status(TaskDefinitionStatus::Inactive)
        .send()
        .await?;

    let task_definitions = list_task_definitions.task_definition_arns();

    log::info!("Found {} task definitions", task_definitions.len());

    let chunk_size = 10;
    let task_definition_chunks = task_definitions.chunks(chunk_size).collect::<Vec<_>>();

    let delete_task_definitions_iter = task_definition_chunks.iter().map(|td| {
        client
            .delete_task_definitions()
            .set_task_definitions(Some(td.to_vec()))
            .send()
    });

    let delete_task_definitions_stream = futures::stream::iter(delete_task_definitions_iter);

    delete_task_definitions_stream
        .for_each_concurrent(PROCESS_AMOUNT_AT_ONCE, |result| async move {
            if let Err(e) = result.await {
                log::error!("Error deleting task definition: {:?}", e);
            }
        })
        .await;

    Ok(())
}

async fn get_client(region: Option<&str>) -> Client {
    let region_provider = match region {
        Some(region) => {
            RegionProviderChain::first_try(Region::new(region.to_owned())).or_default_provider()
        }
        None => RegionProviderChain::default_provider(),
    };

    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;
    Client::new(&config)
}

async fn get_clients() -> Vec<Client> {
    let file_path = "regions.txt";
    let file_content = fs::read_to_string(file_path).expect("Failed to read regions file");
    let regions = file_content
        .lines()
        .map(|line| line.trim())
        .collect::<Vec<_>>();

    let client_futures = regions.into_iter().map(|region| get_client(Some(region)));

    join_all(client_futures).await
}

async fn delete_cluster(client: &Client, cluster: &Cluster) -> Result<()> {
    let name = cluster.cluster_name().expect("Cluster name not found");

    log::info!("Deleting cluster: {}", name);

    delete_cluster_services(client, name)
        .await
        .expect("Error deleting services");

    let cluster_delete = client.delete_cluster().cluster(name).send().await?;
    log::info!(
        "Cluster {} deleted",
        cluster_delete.cluster.unwrap().cluster_name.unwrap()
    );

    Ok(())
}

async fn delete_cluster_services(client: &Client, cluster_name: &str) -> Result<()> {
    let services = client
        .list_services()
        .cluster(cluster_name)
        .send()
        .await
        .expect("Error listing services");

    for service_arn in services.service_arns() {
        log::info!("Service: {}", service_arn);

        let update_service = client
            .update_service()
            .cluster(cluster_name)
            .service(service_arn)
            .desired_count(0)
            .send()
            .await
            .expect("Error updating service");

        log::info!(
            "Service updated: {:?}",
            update_service.service.unwrap().service_name.unwrap()
        );

        let delete_service = client
            .delete_service()
            .cluster(cluster_name)
            .service(service_arn)
            .send()
            .await?;

        log::info!(
            "Service deleted: {:?}",
            delete_service.service.unwrap().service_name.unwrap()
        );
    }

    Ok(())
}
