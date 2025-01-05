pub mod sdk_functions; 
#[cfg(feature = "arh")]
mod arh_client; 
#[cfg(not(feature = "arh"))]
mod sdk_client; 
pub mod aws_resolver; 
pub mod profile_resolver;
pub mod types;

use crate::aws::aws_resolver::IAwsProvider;

#[cfg(feature = "arh")]
pub async fn bastions(profile: &str) -> Vec<types::Bastion> {
    let resolver = arh_client::ArhAwsResolver::new().await;
    resolver.bastions(profile).await
}

#[cfg(not(feature = "arh"))]
pub async fn bastions(profile: &str) -> Vec<types::Bastion> {
    let resolver = sdk_client::SdkAwsResolver::new().await;
    resolver.bastions(profile).await
}