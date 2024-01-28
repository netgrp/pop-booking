use std::collections::HashMap;

struct SessionToken {
    token: String,
    expiry: u64,
}

pub struct AuthApp {
    tokens: HashMap<String, SessionToken>,
}

impl Default for AuthApp {
    fn default() -> Self {
        Self::new()
    }
}

impl AuthApp {
    pub fn new() -> AuthApp {
        AuthApp {
            tokens: HashMap::new(),
        }
    }
}
