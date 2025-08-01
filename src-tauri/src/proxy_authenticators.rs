use crate::proxy::ProxyInterceptor;
use crate::shared::Env;
use crate::{aws, shared::CookieJar, wombat_api};
use async_trait::async_trait;
use headers::authorization::Credentials;
use headers::Authorization;
use log::{info, warn};
use tracing_unwrap::ResultExt;
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

#[allow(dead_code)]
#[derive(serde::Deserialize, Clone, Debug)]
struct JepsenResponse {
    issued_at: String,
    client_id: String,
    access_token: String,
}

pub struct JepsenAutheticator {
    aws_config: aws_config::SdkConfig,
    path_prefix: String,
    jepsen_url: String,
    api_name: String,
    client_id: String,
    secret_arn: String,
}
impl JepsenAutheticator {
    pub fn from_proxy_auth_config(
        aws_config: &aws_config::SdkConfig,
        jepsen_config: wombat_api::ProxyAuthConfig,
    ) -> Self {
        JepsenAutheticator {
            aws_config: aws_config.clone(),
            api_name: jepsen_config.jepsen_api_name.unwrap(),
            jepsen_url: jepsen_config.jepsen_auth_api.unwrap(),
            path_prefix: jepsen_config.api_path,
            client_id: jepsen_config.jepsen_client_id.unwrap(),
            secret_arn: jepsen_config.secret_name,
        }
    }

    pub async fn get_jepsen_token(&self) -> Result<String, String> {
        info!("Getting token {}", &self.secret_arn);
        let client_secret = aws::get_secret(&self.aws_config, &self.secret_arn)
            .await
            .unwrap_or_log();
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
                        Err(format!("Failed to deserialize, {e}"))
                    }
                }
            }
            Err(e) => {
                warn!("Failed to get jepsen secret, {}", &e);
                Err(format!("Failed to get jepsen secret, {e}"))
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
        if let Ok(token) = self.get_jepsen_token().await {
            info!("adding jepsen headers");
            headers.insert(
                "Authorization",
                format!("Bearer {}", &token).parse().unwrap(),
            );
        }
    }
}

pub struct BasicAuthenticator {
    path_prefix: String,
    user: String,
    password: Option<String>,
}
impl BasicAuthenticator {
    pub async fn from_proxy_auth_config(
        aws_config: &aws_config::SdkConfig,
        basic_config: wombat_api::ProxyAuthConfig,
    ) -> Self {
        BasicAuthenticator {
            user: basic_config.basic_user.unwrap(),
            path_prefix: basic_config.api_path,
            password: aws::get_secret(aws_config, basic_config.secret_name.as_str())
                .await
                .ok(),
        }
    }
}

#[async_trait]
impl ProxyInterceptor for BasicAuthenticator {
    fn applies(&self, uri: &str) -> bool {
        uri.starts_with(&self.path_prefix)
    }

    async fn modify_headers(&self, headers: &mut Headers) {
        if let Some(password) = self.password.clone() {
            let credentials = Authorization::basic(&self.user, &password).0.encode();
            let credentials_value = credentials.to_str().unwrap();
            info!("adding basic auth header");
            headers.insert("Authorization", credentials_value.parse().unwrap());
        } else {
            warn!("passowrd not found, skipping basic auth header");
        }
    }
}

pub struct CookieAutheticator {
    pub env: Env,
    pub jar: std::sync::Arc<tokio::sync::Mutex<CookieJar>>,
}

#[async_trait]
impl ProxyInterceptor for CookieAutheticator {
    fn applies(&self, _uri: &str) -> bool {
        true
    }
    async fn modify_headers(&self, headers: &mut Headers) {
        let jar = self.jar.lock().await;
        let header_value = jar.header_value_for_env(&self.env);
        if !header_value.is_empty() {
            info!("Injecting cookie: {header_value}");
            headers.insert("Cookie", header_value.parse().unwrap());
        }
    }
}
