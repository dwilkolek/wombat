use base64::Engine;
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use log::info;
use serde::{Deserialize, Serialize};

pub struct WombatApi {
    user_id: uuid::Uuid,
    user: String,
    password: String,
    jwt: Option<String>,
    url: String,
}

impl WombatApi {
    pub fn new(url: String, user: String, password: String, user_id: uuid::Uuid) -> Self {
        WombatApi {
            user_id,
            jwt: None,
            user,
            password,
            url,
        }
    }

    pub async fn ping(&self) -> bool {
        let client = reqwest::Client::new();
        let to = format!("{}{}", &self.url, "/health");
        log::info!("pinging: {}", &to);
        let response = client.get(to).send().await;
        if let Ok(response) = response {
            return response.status() == reqwest::StatusCode::OK;
        }

        false
    }

    pub async fn status(&mut self, required_feature: &str) -> Result<String, String> {
        let mut wombat_api_status = Err("Not connected".to_string());
        if self.ping().await {
            wombat_api_status = Err("Not authenticated".to_string());
            if self.auth().await {
                wombat_api_status = Err("Requirements not met".to_string());
                if self.is_feature_enabled(required_feature).await {
                    wombat_api_status = Ok("Connected".to_string());
                }
            }
        }
        wombat_api_status
    }

    pub fn is_token_valid(&self) -> bool {
        self.jwt
            .as_ref()
            .map(|token| Claims::from_token(token))
            .is_some_and(|claims| claims.is_valid())
    }

    pub async fn auth(&mut self) -> bool {
        if self.is_token_valid() {
            return true;
        }

        info!("Invalid wombat jwt token, getting new one");
        let client = reqwest::Client::new();
        let to = format!("{}{}", &self.url, "/api/login");
        log::info!("authenticating with: {}", &to);
        let response = client
            .post(to)
            .basic_auth(self.user.clone(), Some(self.password.clone()))
            .body(format!("{}", &self.user_id))
            .send()
            .await;
        if let Ok(response) = response {
            if response.status() == reqwest::StatusCode::OK {
                let jwt = response.text().await;
                if let Ok(jwt) = jwt {
                    info!(
                        "JWT token valid untill {}",
                        Claims::from_token(&jwt)
                            .expiry_date()
                            .format("%Y-%m-%d %H:%M:%S")
                            .to_string()
                    );
                    self.jwt = Some(jwt);
                    return true;
                }
            }
        }
        false
    }

    fn client(&self) -> Option<reqwest::Client> {
        if let Some(jwt) = self.jwt.as_ref() {
            let mut headers = reqwest::header::HeaderMap::new();
            headers.append(
                "Authorization",
                reqwest::header::HeaderValue::from_str(&format!("Bearer {}", jwt)).unwrap(),
            );
            let client = reqwest::Client::builder().default_headers(headers).build();
            if let Ok(client) = client {
                return Some(client);
            }
        }
        None
    }

    pub async fn is_feature_enabled(&mut self, feature: &str) -> bool {
        log::info!("checking feature {}", feature);
        self.auth().await;
        if let Some(client) = self.client() {
            let response = client
                .get(format!("{}/api/features/{}", self.url, feature))
                .send()
                .await;
            if let Ok(response) = response {
                if let Ok(body) = response.text().await {
                    return body == "true";
                }
            }
        }

        false
    }

    pub async fn all_features_enabled(&mut self) -> Vec<String> {
        log::info!("checking all_features_enabled");
        self.auth().await;
        if let Some(client) = self.client() {
            let response = client
                .get(format!("{}/api/features", self.url))
                .send()
                .await;
            if let Ok(response) = response {
                if let Ok(body) = response.json::<Vec<String>>().await {
                    return body;
                }
            }
        }

        Vec::new()
    }

    pub async fn log_filters(&mut self) -> Vec<LogFilter> {
        log::info!("getting log filters");
        self.auth().await;
        if let Some(client) = self.client() {
            let body = client
                .get(format!("{}/api/log-filters", self.url))
                .send()
                .and_then(|response| response.json::<Vec<LogFilter>>())
                .await;
            return match body {
                Ok(body) => body,
                Err(e) => {
                    log::error!("fetching log filters failed, error: {}", e);
                    vec![]
                }
            };
        }
        vec![]
    }

    pub async fn get_proxy_auth_configs(&mut self) -> Vec<ProxyAuthConfig> {
        log::info!("getting proxy auth configs");
        self.auth().await;
        if let Some(client) = self.client() {
            let body = client
                .get(format!("{}/api/proxy-auth-configs", self.url))
                .send()
                .and_then(|response| response.json::<Vec<ProxyAuthConfig>>())
                .await;
            return match body {
                Ok(body) => body,
                Err(e) => {
                    log::error!("fetching proxy auth configs failed, error: {}", e);
                    vec![]
                }
            };
        }
        vec![]
    }

    pub async fn report_versions(&mut self, browser_extension: Option<String>) -> bool {
        if !self.is_token_valid() {
            return false;
        }
        if let Some(client) = self.client() {
            let result = client
                .post(format!("{}/api/versions", self.url))
                .json(&Versions {
                    browser_extension: browser_extension.clone(),
                    app: env!("CARGO_PKG_VERSION").to_owned(),
                })
                .send()
                .await;
            info!(
                "Reporting versions app={} browser_ext={:?}, result_ok={}",
                env!("CARGO_PKG_VERSION").to_owned(),
                browser_extension,
                result.is_ok()
            );
            true
        } else {
            false
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TrackedEvent {
    name: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Versions {
    browser_extension: Option<String>,
    app: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    id: i64,
    filter: String,
    services: Vec<String>,
    label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyAuthConfig {
    pub id: i64,
    pub from_app: String,
    pub to_app: String,
    pub env: String,

    pub auth_type: String,
    pub api_path: String,

    pub jepsen_auth_api: Option<String>,
    pub jepsen_api_name: Option<String>,
    pub jepsen_client_id: Option<String>,

    pub basic_user: Option<String>,

    pub secret_name: String,

    pub require_sso_profile: bool,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Claims {
    app_user_id: String,
    email: String,
    role: String,
    exp: i64,
}

impl Claims {
    fn from_token(token: &str) -> Self {
        let parts = token.split('.').collect::<Vec<&str>>();
        let payload = parts.get(1).unwrap();
        let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
            .decode(*payload)
            .unwrap();
        serde_json::from_slice::<Claims>(&decoded).unwrap()
    }

    fn expiry_date(&self) -> DateTime<Utc> {
        chrono::DateTime::from_timestamp(self.exp, 1000).unwrap()
    }

    fn is_valid(&self) -> bool {
        chrono::Utc::now() < self.expiry_date()
    }
}
