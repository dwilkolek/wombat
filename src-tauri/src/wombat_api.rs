use serde::{Deserialize, Serialize};

pub static REQUIRED_FEATURE: &'static str = "wombat-2.2.0";

pub struct WombatApi {
    user_id: uuid::Uuid,
    user: String,
    password: String,
    jwt: Option<String>,
    url: String,
}

impl WombatApi {
    pub fn new(url: String, user: String, password: String, user_id: uuid::Uuid) -> Self {
        return WombatApi {
            user_id,
            jwt: None,
            user,
            password,
            url,
        };
    }

    pub async fn ping(&self) -> bool {
        let client = reqwest::Client::new();
        let to = format!("{}{}", &self.url, "/health");
        log::info!("pinging: {}", &to);
        let response = client.get(to).send().await;
        if let Ok(response) = response {
            return response.status() == reqwest::StatusCode::OK;
        }

        return false;
    }

    pub async fn status(&mut self) -> Result<String, String> {
        let mut wombat_api_status = Err("Not connected".to_string());
        if self.ping().await {
            wombat_api_status = Err("Not authenticated".to_string());
            if self.auth().await {
                wombat_api_status = Err("Your client is outdated".to_string());
                if self.is_feature_enabled(REQUIRED_FEATURE).await {
                    wombat_api_status = Ok("Connected & up-to-date".to_string());
                }
            }
        }
        return wombat_api_status;
    }

    pub async fn auth(&mut self) -> bool {
        let client = reqwest::Client::new();
        let to = format!("{}{}", &self.url, "/api/login");
        log::info!("authenticating with: {}", &to);
        if self.jwt.is_some() {
            return true;
        }
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
                    self.jwt = Some(jwt);
                    return true;
                }
            }
        }

        return false;
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
        return None;
    }

    pub async fn is_feature_enabled(&self, feature: &str) -> bool {
        log::info!("checking feature {}", feature);
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

        return false;
    }

    pub async fn log_filters(&self) -> Vec<LogFilter> {
        log::info!("getting log filters");
        if let Some(client) = self.client() {
            let response = client
                .get(format!("{}/api/log-filters", self.url))
                .send()
                .await;
            if let Ok(response) = response {
                if let Ok(body) = response.json::<Vec<LogFilter>>().await {
                    return body;
                }
            }
        }
        return vec![];
    }

    pub async fn get_proxy_auth_configs(&self) -> Vec<ProxyAuthConfig> {
        log::info!("getting proxy auth configs");
        if let Some(client) = self.client() {
            let response = client
                .get(format!("{}/api/proxy-auth-configs", self.url))
                .send()
                .await;
            if let Ok(response) = response {
                if let Ok(body) = response.json::<Vec<ProxyAuthConfig>>().await {
                    return body;
                }
            }
        }
        return vec![];
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilter {
    id: i64,
    filter: String,
    services: Vec<String>,
    label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}
