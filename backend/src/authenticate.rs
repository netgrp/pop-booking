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
use std::{collections::HashMap, sync::Arc};
use std::{env, hash::Hash};
use tokio::sync::RwLock;
use tokio::time::Duration;
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
pub struct SessionToken {
    user: User,
    expiry: u64,
}

impl SessionToken {
    pub fn get_user(&self) -> &User {
        &self.user
    }
}

pub struct AuthApp {
    tokens: HashMap<TokenId, SessionToken>,
    timeouts: HashMap<String, (DateTime<Utc>, u16)>,
    client: reqwest::Client,
    hasher: Sha1,
    knet_username: String,
    knet_password: String,
    knet_api_base_url: String,
}
impl AuthApp {
    pub fn new() -> Result<AuthApp> {
        let knet_api_base_url = env::var("KNET_API_BASE_URL")?;
        let knet_api_username = env::var("KNET_API_USERNAME")?;
        let knet_api_password = env::var("KNET_API_PASSWORD")?;

        let client = reqwest::Client::new();
        let hasher = Sha1::new();

        Ok(AuthApp {
            tokens: HashMap::new(),
            timeouts: HashMap::new(),
            client,
            hasher,
            knet_api_base_url,
            knet_username: knet_api_username,
            knet_password: knet_api_password,
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

    pub fn assert_login(&self, jar: CookieJar) -> Result<SessionToken, String> {
        let cookie = jar
            .get("SESSION-COOKIE")
            .ok_or("No cookie found")
            .map_err(|e| {
                trace!("cookie not found: {}", e);
                "Not logged in"
            })?
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
    ) -> Result<(Cookie<'static>, SessionToken), String> {
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

        let response = 'login_block: {
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
                    "Login failed, no user found for username {}",
                    username
                ));
            }

            //parse response
            let response = response
                .json::<serde_json::Value>()
                .await
                .map_err(|_| "Failed to parse response")?;

            let user_count = response
                .get("count")
                .and_then(|count| count.as_u64())
                .ok_or("Failed to get count from K-Net")?;

            if user_count != 1 {
                break 'login_block Err(format!("Login failed, user count not 1: {}", user_count));
            }

            let passw_hash = response
                .get("results")
                .and_then(|results| results[0].get("password"))
                .and_then(|password| password.as_str())
                .ok_or("Failed to get password from K-Net")?;

            let pwd_parts = passw_hash.split('$').collect::<Vec<_>>();

            if pwd_parts.len() != 3 || pwd_parts[0] != "sha1" {
                break 'login_block Err(format!(
                    "Login failed, password hash not sha1: {}",
                    passw_hash
                ));
            }

            let hash = format!("{}{}", pwd_parts[1], password);
            self.hasher.update(hash);

            let hash = format!("{:x}", self.hasher.finalize_reset());

            if hash != pwd_parts[2] {
                break 'login_block Err("Login failed, wrong password".to_string());
            }
            Ok::<serde_json::Value, String>(response)
        }
        .map_err(|e| {
            debug!(e);
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

        let username = response
            .get("results")
            .and_then(|results| results[0].get("username"))
            .and_then(|username| username.as_str())
            .ok_or("Failed to get username from K-Net")?;

        let vlan_url = response
            .get("results")
            .and_then(|results| results[0].get("vlan"))
            .and_then(|vlan| vlan.as_str())
            .ok_or("Failed to get vlan from K-Net")?;

        let vlan_response = self
            .client
            .get(vlan_url)
            .basic_auth(&self.knet_username, Some(&self.knet_password))
            .send()
            .await
            .map_err(|_| "Failed to send request")?;

        if vlan_response.status() != StatusCode::OK {
            return Err(format!(
                "Login failed, no vlan found for username {}",
                username
            ));
        }

        //parse response

        let vlan_response = vlan_response
            .json::<serde_json::Value>()
            .await
            .map_err(|_| "Failed to parse response")?;

        let room = vlan_response
            .get("room")
            .map(|room| {
                room.to_string().split_whitespace().last().map(|s| {
                    let mut chars = s.chars();
                    chars.next_back();
                    chars.as_str().parse::<u16>()
                })
            })
            .ok_or("Failed to get room from K-Net")?
            .ok_or("Failed to parse room from K-Net")?
            .map_err(|_| "Failed to parse room from K-Net".to_string())?;

        let session_token = SessionToken {
            user: User::new(username.to_string(), room),
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
}
