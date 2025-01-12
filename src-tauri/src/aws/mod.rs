#[cfg(feature = "arh")]
mod arh_client;
pub mod aws_resolver;
pub mod profile_resolver;
#[cfg(not(feature = "arh"))]
mod sdk_client;
pub mod sdk_functions;
pub mod types;

use crate::aws::aws_resolver::IAwsProvider;

// #[cfg(feature = "arh")]
// fn resolver() -> impl IAwsProvider + Sync + Send {
//     arh_client::ArhAwsResolver::new()
// }

// #[cfg(not(feature = "arh"))]
// fn resolver() -> sdk_client::SdkAwsResolver {
//     sdk_client::SdkAwsResolver::new()
// }

#[cfg(not(feature = "arh"))]
const RESOLVER: sdk_client::SdkAwsResolver = sdk_client::SdkAwsResolver {};

#[cfg(feature = "arh")]
const RESOLVER: arh_client::ArhAwsResolver = arh_client::ArhAwsResolver {};

pub async fn bastions(profile: &str) -> Vec<types::Bastion> {
    return RESOLVER.bastions(profile).await;
}
