use anyhow::Result;
use axum_extra::extract::{
    cookie::{self, Cookie},
    CookieJar,
};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::env;
use tracing::{debug, info, trace};

use crate::booker::User;

#[derive(Deserialize)]
pub struct LoginPayload {
    pub username: String,
    pub password: String,
}

#[derive(Hash, PartialEq, Eq, Clone, Copy)]
pub struct TokenId([u8; 32]);

impl std::fmt::Display for TokenId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for b in self.0.iter() {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl From<[u8; 32]> for TokenId {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl TryFrom<Vec<u8>> for TokenId {
    type Error = String;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        let mut token = [0u8; 32];
        //check that len is 32
        if bytes.len() != 32 {
            return Err("Invalid token length".to_string());
        }
        token.copy_from_slice(&bytes);
        Ok(Self(token))
    }
}

impl TryFrom<&str> for TokenId {
    type Error = String;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect::<Result<Vec<u8>, String>>()
            .map(|v| Self::try_from(v))?
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
    fn to_string(&self) -> String {
        let mut s = String::new();
        for b in self.0.iter() {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.0
    }

    pub fn new() -> Self {
        Self(rand::random::<[u8; 32]>())
    }
}

#[derive(Serialize, Deserialize, Clone)]
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
            client,
            hasher,
            knet_api_base_url,
            knet_username: knet_api_username,
            knet_password: knet_api_password,
        })
    }

    fn gen_cookie(token: &TokenId) -> String {
        trace!("Generating cookie with token: {}", token.to_string());
        Cookie::build(("SESSION-COOKIE", token.to_string()))
            .expires(None)
            .same_site(cookie::SameSite::Strict)
            .path("/")
            .build()
            .to_string()
    }

    pub fn update_token(&mut self, token: &TokenId) -> Result<String, String> {
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
            .value()
            .to_string();

        let token_id = TokenId::try_from(cookie)?;

        trace!("Checking token: {}", token_id.to_string());
        self.tokens
            .get(&token_id)
            .ok_or("Not logged in")
            .map_err(|e| {
                trace!("token not found: {}", e);
                "Not logged in"
            })?
            .expiry
            .checked_sub(chrono::Utc::now().timestamp() as u64)
            .ok_or("Token expired")
            .map_err(|e| {
                trace!("token expired: {}", e);
                "Not logged in"
            })?;

        Ok(self.tokens.get(&token_id).unwrap().clone())
    }

    pub async fn authenticate_user(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<(String, SessionToken), String> {
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
            return Err(format!(
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
            return Err(format!("Login failed, user count not 1: {}", user_count));
        }

        let passw_hash = response
            .get("results")
            .and_then(|results| results[0].get("password"))
            .and_then(|password| password.as_str())
            .ok_or("Failed to get password from K-Net")?;

        let pwd_parts = passw_hash.split('$').collect::<Vec<_>>();

        if pwd_parts.len() != 3 || pwd_parts[0] != "sha1" {
            return Err(format!(
                "Login failed, password hash not sha1: {}",
                passw_hash
            ));
        }

        let hash = format!("{}{}", pwd_parts[1], password);
        self.hasher.update(hash);

        let hash = format!("{:x}", self.hasher.finalize_reset());

        debug!("Hash: {}", hash);
        debug!("K-Net hash: {}", pwd_parts[2]);
        if hash != pwd_parts[2] {
            return Err(format!("Login failed, wrong password"));
        }

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
                    chars.as_str().parse::<u8>()
                })
            })
            .ok_or("Failed to get room from K-Net")?
            .ok_or("Failed to parse room from K-Net")?
            .map_err(|e| format!("Failed to parse room from K-Net {}", e))?;

        let session_token = SessionToken {
            user: User::new(username.to_string(), room),
            expiry: chrono::Utc::now().timestamp() as u64 + 60 * 60 * 24, //24 hours
        };

        debug!("Inserting new session token: {}", token.to_string());
        self.tokens.insert(token, session_token.clone());

        Ok((Self::gen_cookie(&token), session_token))
    }

    pub fn logout(&mut self, token: &TokenId) -> Result<(), String> {
        self.tokens.remove(token).ok_or("Token not found")?;
        debug!("Removed token: {}", token.to_string());
        Ok(())
    }
}
