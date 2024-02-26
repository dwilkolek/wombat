use crate::aws;
use crate::proxy::ProxyInterceptor;
use async_trait::async_trait;
use log::warn;
use reqwest::header::HeaderValue;
use warp_reverse_proxy::Headers;

#[derive(serde::Serialize)]
struct JepsenBody {
    grant_type: String,
    api_name: String,
    client_id: String,
    client_secret: String,
}

impl JepsenBody {
    fn new(api_name: String, client_id: String, client_secret: String) -> Self {
        JepsenBody {
            grant_type: String::from("client_credentials"),
            api_name,
            client_id,
            client_secret,
        }
    }
}

#[derive(serde::Deserialize, Clone, Debug)]
struct JepsenResponse {
    issued_at: String,
    client_id: String,
    access_token: String,
}

pub struct JepsenAutheticator {
    pub aws_config: aws_config::SdkConfig,
    pub path_prefix: String,
    pub jepsen_url: String,
    pub api_name: String,
    pub client_id: String,
    pub secret_arn: String,
}
impl JepsenAutheticator {
    pub async fn get_jepsen_token(&self) -> Result<String, String> {
        log::info!("Getting token");
        let client_secret = aws::get_secret(&self.aws_config, &self.secret_arn)
            .await
            .unwrap();
        let client = reqwest::Client::new();
        let response = client
            .post(&self.jepsen_url)
            .json(&JepsenBody::new(
                self.api_name.clone(),
                self.client_id.clone(),
                client_secret,
            ))
            .send()
            .await;
        match response {
            Ok(response) => {
                let response_body = response.json::<JepsenResponse>().await;
                match response_body {
                    Ok(body) => Ok(body.access_token),
                    Err(e) => {
                        warn!("Failed to deserialize, {}", &e);
                        return Err(format!("Failed to deserialize, {}", e));
                    }
                }
            }
            Err(e) => {
                warn!("Failed to get jepsen secret, {}", &e);
                return Err(format!("Failed to get jepsen secret, {}", e));
            }
        }
    }
}

#[async_trait]
impl ProxyInterceptor for JepsenAutheticator {
    fn applies(&self, uri: &str) -> bool {
        uri.starts_with(&self.path_prefix)
    }
    async fn modify_headers(&self, headers: &mut Headers) {
        log::info!("Jepsen applies");
        if let Ok(token) = self.get_jepsen_token().await {
            headers.insert(
                "Authorization",
                HeaderValue::from_str(format!("Bearer {}", &token).as_str()).unwrap(),
            );
        }
    }
}
