use anyhow::{anyhow, Result};
use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, info};

const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OAuthCredentials {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub obtained_at: i64,
}

impl OAuthCredentials {
    pub fn is_expired(&self) -> bool {
        let obtained_at = DateTime::from_timestamp(self.obtained_at / 1000, 0)
            .unwrap_or(Utc::now());
        let expires_at = obtained_at + Duration::seconds(self.expires_in - 300);
        
        Utc::now() >= expires_at
    }
}

pub struct AuthManager {
    client: Client,
    client_id: String,
    client_secret: String,
}

impl AuthManager {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
        }
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<OAuthCredentials> {
        info!("Refreshing Google OAuth token");

        let params = [
            ("refresh_token", refresh_token),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("grant_type", "refresh_token"),
        ];

        let response = self.client
            .post(GOOGLE_TOKEN_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(anyhow!("Failed to refresh token: {}", error_text));
        }

        let token_response: TokenResponse = response.json().await?;
        
        Ok(OAuthCredentials {
            access_token: token_response.access_token,
            refresh_token: refresh_token.to_string(),
            token_type: token_response.token_type,
            expires_in: token_response.expires_in,
            obtained_at: Utc::now().timestamp_millis(),
        })
    }

    pub async fn ensure_valid_token(&self, creds: &mut OAuthCredentials) -> Result<()> {
        if creds.is_expired() {
            debug!("Token expired, refreshing");
            let new_creds = self.refresh_token(&creds.refresh_token).await?;
            *creds = new_creds;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i64,
}