
use crate::aws::types::Bastion;
pub trait IAwsProvider {
    async fn bastions(&self, profile: &str) -> Vec<Bastion>;
}
