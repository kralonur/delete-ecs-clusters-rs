use anyhow::Result;
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_ecs::{types::Cluster, Client};
use futures::StreamExt;

const PROCESS_AMOUNT_AT_ONCE: usize = 5;

pub async fn run_task_definitions() -> Result<()> {
    let client = get_client().await;

    let list_task_definitions = client.list_task_definitions().send().await?;

    let task_definitions = list_task_definitions.task_definition_arns();

    log::info!("Found {} task definitions:", task_definitions.len());

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
                log::error!("Error deleting task definition: {:?}", e);
            }
        })
        .await;

    Ok(())
}

pub async fn run() -> Result<()> {
    let client = get_client().await;

    let list_clusters = client.list_clusters().send().await?;

    let cluster_arns = list_clusters.cluster_arns();
    log::info!("Found {} clusters:", cluster_arns.len());

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

async fn get_client() -> Client {
    let region_provider = RegionProviderChain::default_provider();
    log::info!("Region: {:?}", region_provider.region().await);
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;
    Client::new(&config)
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
