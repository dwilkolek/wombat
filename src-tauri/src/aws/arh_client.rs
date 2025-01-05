pub mod arh {
    tonic::include_proto!("arh");
}

use crate::aws::types::{Bastion, Env};
use arh::arh_client::ArhClient;

async fn create_client() -> arh::arh_client::ArhClient<tonic::transport::Channel> {
    ArhClient::connect("http://[::1]:6666")
        .await
        .expect("Failed to connect to server")
}

pub struct ArhAwsResolver {}
impl ArhAwsResolver {
    pub async fn new() -> Self {
        ArhAwsResolver {}
    }
}
impl crate::aws::aws_resolver::IAwsProvider for ArhAwsResolver {
    async fn bastions(&self, profile: &str) -> Vec<Bastion> {
        let mut client = create_client().await;
        let request = tonic::Request::new(arh::DescribeBastionsRequest {
            profile: profile.to_string(),
        });
        let response = client.describe_bastions(request).await.unwrap();
        response
            .into_inner()
            .results
            .into_iter()
            .map(|b| Bastion {
                instance_id: b.instance_id,
                env: arh::Environment::try_from(b.env)
                    .map(|e| e.into())
                    .expect("Failed to convert to Env"),
            })
            .collect()
    }
}

impl Into<Env> for arh::Environment {
    fn into(self) -> Env {
        match self {
            arh::Environment::Dev => Env::DEV,
            arh::Environment::Demo => Env::DEMO,
            arh::Environment::Prod => Env::PROD,
            _ => Env::DEVNULL,
        }
    }
}
