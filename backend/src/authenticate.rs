use crate::booker::User;
use anyhow::Result;
use axum_extra::extract::{
    cookie::{self, Cookie},
    CookieJar,
};
use base64::prelude::*;
use chrono::{DateTime, Utc};
use rand::{RngCore, SeedableRng};
use reqwest::StatusCode;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::hash::Hash;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::warn;
#[allow(unused_imports)]
use tracing::{debug, info, trace};

#[derive(Deserialize, JsonSchema)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct TokenId([u8; 60]);

impl std::fmt::Display for TokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", BASE64_STANDARD.encode(self.0))
    }
}

impl From<[u8; 60]> for TokenId {
    fn from(bytes: [u8; 60]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<Vec<u8>> for TokenId {
    type Error = String;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut token = [0u8; 60];
        //check that len is 32
        if bytes.len() != 60 {
            return Err("Invalid token length".to_string());
        }
        token.copy_from_slice(&bytes);
        Ok(Self(token))
    }
}

impl TryFrom<&str> for TokenId {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        BASE64_STANDARD
            .decode(s)
            .map_err(|e| e.to_string())
            .and_then(Self::try_from)
    }
}

impl TryFrom<String> for TokenId {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::try_from(s.as_str())
    }
}

impl From<TokenId> for String {
    fn from(token: TokenId) -> Self {
        token.to_string()
    }
}

impl TokenId {
    pub fn to_bytes(&self) -> [u8; 60] {
        self.0
    }

    pub fn new() -> Self {
        let mut bytes = [0u8; 60];
        let mut rng = rand_hc::Hc128Rng::from_entropy();
        rng.fill_bytes(&mut bytes);
        Self(bytes)
    }
}

impl Default for TokenId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize, Deserialize, Clone, JsonSchema, Debug)]
pub struct UserSession {
    pub user: User,
    expiry: u64,
}

pub struct AuthApp {
    tokens: HashMap<TokenId, UserSession>,
    timeouts: HashMap<String, (DateTime<Utc>, u16)>,
    client: reqwest::Client,
    hasher: Sha1,
    knet_username: String,
    knet_password: String,
    knet_api_base_url: String,
}

#[derive(Deserialize, JsonSchema)]
struct UserResponse {
    count: u64,
    results: Vec<UserResult>,
}

#[derive(Deserialize, JsonSchema, Clone)]
struct UserResult {
    username: String,
    password: Option<String>,
    vlan: String,
}

#[derive(Deserialize, JsonSchema, Clone, Debug)]
pub struct VlanResponse {
    #[serde(deserialize_with = "parse_room")]
    room: u16,
}

fn parse_room<'de, D>(deserializer: D) -> Result<u16, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    s.split_whitespace()
        .last()
        .map(|s| {
            s.chars()
                .as_str()
                .parse::<u16>()
                .map_err(serde::de::Error::custom)
        })
        .ok_or(serde::de::Error::custom("Failed to parse room"))?
}

impl AuthApp {
    pub fn new(base_url: String, username: String, password: String) -> Result<AuthApp> {
        let client = reqwest::Client::new();
        let hasher = Sha1::new();

        Ok(AuthApp {
            tokens: HashMap::new(),
            timeouts: HashMap::new(),
            client,
            hasher,
            knet_api_base_url: base_url,
            knet_username: username,
            knet_password: password,
        })
    }

    fn gen_cookie(token: &TokenId) -> Cookie<'static> {
        trace!("Generating cookie with token: {}", token.to_string());
        let mut builder = Cookie::build(("SESSION-COOKIE", token.to_string()))
            .expires(None)
            .same_site(cookie::SameSite::Strict)
            .path("/");

        if cfg!(debug_assertions) {
            builder = builder.secure(false);
        } else {
            builder = builder.secure(true);
        }

        builder.build().into_owned()
    }

    pub fn update_token(&mut self, token: &TokenId) -> Result<Cookie<'static>, String> {
        //assert that token is valid
        self.tokens
            .get(token)
            .ok_or("Token not found")?
            .expiry
            .checked_sub(chrono::Utc::now().timestamp() as u64)
            .ok_or("Token expired")?;

        //update token
        let session_token = self.tokens.remove(token).unwrap();

        let new_token = TokenId::new();
        self.tokens.insert(new_token, session_token);

        Ok(Self::gen_cookie(&new_token))
    }

    pub fn assert_login(&self, jar: CookieJar) -> Result<UserSession, String> {
        let cookie = jar
            .get("SESSION-COOKIE")
            .ok_or("No cookie found")
            .map_err(|_| "Not logged in")?
            .value();

        let token_id = TokenId::try_from(cookie)?;

        if let Some(token) = self.tokens.get(&token_id) {
            if token.expiry <= chrono::Utc::now().timestamp() as u64 {
                return Err("Token expired".to_string());
            }
            Ok(token.clone())
        } else {
            Err("Not logged in".to_string())
        }
    }

    pub async fn authenticate_user(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<(Cookie<'static>, UserSession), String> {
        //check that user isn't timed out

        let now = chrono::Utc::now();
        if let Some((timeout, _)) = self.timeouts.get(username) {
            if now < *timeout {
                return Err(format!(
                    "Try again in {} seconds",
                    (*timeout - now).num_seconds()
                ));
            }
        }

        let user_result = 'login_block: {
            let url = format!(
                "{}network/user/?username={}",
                self.knet_api_base_url, username
            );

            let response = self
                .client
                .get(&url)
                .basic_auth(&self.knet_username, Some(&self.knet_password))
                .send()
                .await
                .map_err(|_| "Failed to send request")?;

            if response.status() != StatusCode::OK {
                break 'login_block Err(format!(
                    "Login failed, auth backend returned status code: {}",
                    response.status()
                ));
            }
            info!("got response: {:?}", response);

            let response = response.json::<UserResponse>().await.map_err(|e| {
                warn!("{} for response ", e);
                format!("Failed to parse user response from k-net login server")
            })?;

            if response.count != 1 {
                break 'login_block Err("Login failed, user not found".to_string());
            }

            if response.results[0].password.is_none() {
                break 'login_block Err("Login failed, password not set".to_string());
            }

            let pwd_parts = response.results[0]
                .password
                .as_ref()
                .unwrap()
                .split('$')
                .collect::<Vec<_>>();

            if pwd_parts.len() != 3 || pwd_parts[0] != "sha1" {
                break 'login_block Err("Login failed, password hash not sha1".to_string());
            }

            let hash = format!("{}{}", pwd_parts[1], password);
            self.hasher.update(hash);

            let hash = format!("{:x}", self.hasher.finalize_reset());

            if hash != pwd_parts[2] {
                break 'login_block Err("Login failed, wrong password".to_string());
            }
            Ok(response.results[0].clone())
        }
        .map_err(|e| {
            warn!(e);
            //increase timeout
            let level = self
                .timeouts
                .get(username)
                .map(|(_, level)| *level + 1)
                .unwrap_or(0);
            let timeout_duration = std::cmp::min(
                Duration::from_millis(250) * (1u32 << level),
                Duration::from_secs(24 * 60 * 60),
            ); //max 24 hour timeout
            self.timeouts
                .insert(username.to_string(), (now + timeout_duration, level));

            format!(
                "login failed, try again in {} seconds",
                timeout_duration.as_secs()
            )
        })?;

        //Otherwise the login was successful so we clean the timeout
        self.timeouts.remove(username);

        // generate a token for the user
        let token = TokenId::new();

        let username = &user_result.username;
        let vlan_url = &user_result.vlan;

        let vlan_response = self
            .client
            .get(vlan_url)
            .basic_auth(&self.knet_username, Some(&self.knet_password))
            .send()
            .await
            .map_err(|_| "Failed to send request")?;

        match vlan_response.status() {
            StatusCode::OK => {}
            StatusCode::UNAUTHORIZED => {
                return Err("Failed to authenticate with K-Net".to_string());
            }
            StatusCode::NOT_FOUND => {
                return Err("Vlan not found".to_string());
            }
            _ => {
                return Err("Failed to get vlan".to_string());
            }
        }

        let vlan_response = vlan_response.json::<VlanResponse>().await.map_err(|e| {
            warn!("{}", e);
            "Failed to parse vlan response from k-net login server"
        })?;

        info!("got vlan: {:?}", vlan_response);

        info!("got vlan: {:?}", vlan_response);

        let session_token = UserSession {
            user: User::new(username.to_string(), vlan_response.room),
            expiry: chrono::Utc::now().timestamp() as u64 + 60 * 60 * 24, //24 hours
        };

        self.tokens.insert(token, session_token.clone());

        Ok((Self::gen_cookie(&token), session_token))
    }

    pub fn logout(&mut self, token: &TokenId) -> Result<(), String> {
        self.tokens.remove(token).ok_or("Token not found")?;
        debug!("Removed token: {}", token.to_string());
        Ok(())
    }

    pub fn clean_expired_tokens(&mut self) -> u32 {
        let now = chrono::Utc::now().timestamp() as u64;
        let expired_tokens = self
            .tokens
            .iter()
            .filter(|(_, token)| token.expiry <= now)
            .map(|(token_id, _)| *token_id)
            .collect::<Vec<_>>();

        let count = expired_tokens.len() as u32;

        for token_id in expired_tokens {
            self.tokens.remove(&token_id);
            debug!("Removed expired token: {}", token_id.to_string());
        }
        count
    }

    pub async fn start_token_cleanup(app: Arc<RwLock<Self>>) {
        loop {
            let duration = app
                .read()
                .await
                .tokens
                .values()
                .map(|token| token.expiry)
                .min()
                .map(|expiry| {
                    let now = chrono::Utc::now().timestamp() as u64;
                    expiry.saturating_sub(now)
                })
                .unwrap_or(24 * 60 * 60);
            info!("Cleaning expired tokens in {} seconds", duration);
            tokio::time::sleep(Duration::from_secs(duration + 1)).await;
            let count = app.write().await.clean_expired_tokens();
            info!("Cleaning expired tokens done, removed {} tokens", count);
        }
    }

    pub async fn start_timeout_cleanup(app: Arc<RwLock<Self>>) {
        //run every day
        loop {
            let duration = app
                .read()
                .await
                .timeouts
                .values()
                .map(|(timeout, _)| *timeout)
                .min()
                .map(|timeout| (timeout - chrono::Utc::now()).num_seconds())
                .unwrap_or(0)
                + 24 * 60 * 60
                + 1;
            info!("Cleaning timeouts in {} seconds", duration);
            tokio::time::sleep(Duration::from_secs(duration as u64)).await;
            //remove timeouts after a day. This means the max timeout is 48 hours
            let count = app.read().await.timeouts.len();
            app.write().await.timeouts.retain(|_, (timeout, _)| {
                *timeout + Duration::from_secs(24 * 60 * 60) > chrono::Utc::now()
            });
            let diff = count - app.read().await.timeouts.len();
            info!("Cleaning timeouts done, removed {} timeouts", diff);
        }
    }

    // pub async fn view_user(&self, username: &str) -> Result<UserResult, String> {
    //     let url = format!(
    //         "{}network/user/?username={}",
    //         self.knet_api_base_url, username
    //     );
    //     println!("url: {}", url);

    //     let response = self
    //         .client
    //         .get(&url)
    //         .basic_auth(&self.knet_username, Some(&self.knet_password))
    //         .send()
    //         .await
    //         .map_err(|_| format!("Failed to send request"))?;

    //     if response.status() != StatusCode::OK {
    //         return Err("Failed to get user".to_string());
    //     }
    //     let json_response = response.json::<serde_json::Value>().await.map_err(|e| {
    //         warn!("{} for response ", e);
    //         "Failed to parse user response from k-net login server"
    //     })?;
    //     println!("got response: {}", json_response);

    //     // print pretty printed json
    //     // println!("{}", response.text().await.unwrap());

    //     // let response = response.json::<UserResponse>().await.map_err(|e| {
    //     //     warn!("{} for response ", e);
    //     //     "Failed to parse user response from k-net login server"
    //     // })?;

    //     // if response.count != 1 {
    //     //     return Err("User not found".to_string());
    //     // }

    //     Err("User not found".to_string())
    // }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use std::collections::HashMap;
//     use std::env;
//     use std::time::Duration;
//     use tokio::sync::RwLock;
//     use tokio::time::sleep;

//     #[tokio::test]
//     async fn test_view_user() -> Result<()> {
//         dotenvy::dotenv().unwrap_or_default();
//         let auth_app = Arc::new(RwLock::new(AuthApp::new(
//             env::var("KNET_API_BASE_URL")?,
//             env::var("KNET_API_USERNAME")?,
//             env::var("KNET_API_PASSWORD")?,
//         )?));
//         let username = "thomas.s.conrad@gmail.com";
//         let user = auth_app.read().await.view_user(username).await.unwrap();
//         assert_eq!(user.username, username);
//         Ok(())
//     }

//     #[tokio::test]
//     async fn test_authenticate_user_fail() {
//         let mut app = AuthApp::new(
//             "http://localhost:8000/".to_string(),
//             "admin".to_string(),
//             "admin".to_string(),
//         )
//         .unwrap();
//         let result = app.authenticate_user("admin", "wrong").await;
//         assert!(result.is_err());
//         assert_eq!(app.tokens.len(), 0);
//     }

//     #[tokio::test]
//     async fn test_authenticate_user_timeout() {
//         let mut app = AuthApp::new(
//             "http://localhost:8000/".to_string(),
//             "admin".to_string(),
//             "admin".to_string(),
//         )
//         .unwrap();
//         let result = app.authenticate_user("admin", "wrong").await;
//         assert!(result.is_err());
//         assert_eq!(app.tokens.len(), 0);
//         assert_eq!(app.timeouts.len(), 1);
//         sleep(Duration::from_secs(1)).await;
//         let result = app.authenticate_user("admin", "wrong").await;
//         assert!(result.is_err());
//         assert_eq!(app.tokens.len(), 0);
//         assert_eq!(app.timeouts.len(), 1);
//         sleep(Duration::from_secs(1)).await;
//         let result = app.authenticate_user("admin", "wrong").await;
//         assert!(result.is_err());
//         assert_eq!(app.tokens.len(), 0);
//         assert_eq!(app.timeouts.len(), 1);
//         sleep(Duration::from_secs(1)).await;
//         let result = app.authenticate_user("admin", "admin").await;
//         assert!(result.is_ok());
//         assert_eq!(app.tokens.len(), 1);
//         assert_eq!(app.timeouts.len(), 0);
//         app.logout(&TokenId::try_from(result.unwrap().0.value()).unwrap())
//             .unwrap();
//     }

//     #[tokio::test]
//     async fn test_authenticate_user_timeout_cleanup() {
//         let app = Arc::new(RwLock::new(
//             AuthApp::new(
//                 "http://localhost:8000/".to_string(),
//                 "admin".to_string(),
//                 "admin".to_string(),
//             )
//             .unwrap(),
//         ));
//         let app1 = app.clone();
//         let app2 = app.clone();
//         let handle1 = tokio::spawn(async move { AuthApp::start_timeout_cleanup(app1).await });
//         let handle2 = tokio::spawn(async move { AuthApp::start_timeout_cleanup(app2).await });
//         sleep(Duration::from_secs(1)).await;
//         app.write()
//             .await
//             .timeouts
//             .insert("admin".to_string(), (chrono::Utc::now(), 0));
//         sleep(Duration::from_secs(1)).await;
//         assert_eq!(app.read().await.timeouts.len(), 1);
//         sleep(Duration::from_secs(1)).await;
//         assert_eq!(app.read().await.timeouts.len(), 0);
//         sleep(Duration::from_secs(1)).await;
//         assert_eq!(app.read().await.timeouts.len(), 0);
//         handle1.abort();
//         handle2.abort();
//     }
// }
