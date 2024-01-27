use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
mod hourmin;
use hourmin::HourMin;

struct User {
    name: String,
    email: String,
}

struct SessionToken {
    token: String,
    expiry: u64,
}

#[derive(Debug, PartialEq, Eq)]
struct Resource {
    name: String,
    description: String,
    allowed_times: Vec<HourMin>,
}

impl Resource {
    fn new(name: &str, description: &str, allowed_times: Vec<HourMin>) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            allowed_times,
        }
    }

    fn empty(name: &str) -> Self {
        Self {
            name: name.to_string(),
            description: "".to_string(),
            allowed_times: Vec::new(),
        }
    }
}

// Hashing only by name, must be unique
impl Hash for Resource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

struct Booking {
    user: User,
    resource: Resource,
}

pub struct BookingApp {
    sessions: HashMap<User, SessionToken>,
    bookings: Vec<Booking>,
    resources: HashSet<Resource>,
}

impl<'a> BookingApp {
    pub fn from_config() -> Self {
        Self {
            sessions: HashMap::new(),
            bookings: Vec::new(),
            resources: HashSet::new(),
        }
    }

    pub fn new_booking(&mut self, user: User, resource_name: &str, times: [HourMin; 2]) {
        let resource = self.resources.get(&Resource::empty(resource_name));
        todo!()
    }
}
