use crate::aws::types::{Bastion, Env};
use aws_sdk_ec2 as ec2;
use tracing_unwrap::{OptionExt, ResultExt};

use super::profile_resolver;

pub struct SdkAwsResolver {}
impl SdkAwsResolver {
    pub async fn new() -> Self {
        SdkAwsResolver {}
    }
}

impl crate::aws::aws_resolver::IAwsProvider for SdkAwsResolver {
    async fn bastions(&self, profile: &str) -> Vec<Bastion> {
        let config = profile_resolver::sdk_config(profile).await;
        let ec2_client = ec2::Client::new(&config);
        let filter = ec2::types::Filter::builder()
            .name("tag:Name")
            .values("*-bastion*")
            .build();
        let res = ec2_client.describe_instances().filters(filter).send().await;

        let res = res.expect_or_log("Failed to get ec2 bastion instances");
        let res = res.reservations();
        res.iter()
            .flat_map(|r| {
                let instances = r.instances();
                instances
                    .iter()
                    .map(|instance| {
                        let env = Env::from_exact(
                            instance
                                .tags()
                                .iter()
                                .find(|env_tag_maybe| {
                                    env_tag_maybe.key().unwrap_or("unknown").eq("Environment")
                                })
                                .map(|env_tag| env_tag.value().unwrap_or_default())
                                .unwrap_or_default(),
                        );

                        Bastion {
                            instance_id: instance.instance_id().unwrap_or_log().to_owned(),
                            env,
                        }
                    })
                    .collect::<Vec<Bastion>>()
            })
            .collect::<Vec<Bastion>>()
    }
}