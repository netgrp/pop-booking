use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
pub mod api;
use api::NewBooking;
mod hourmin;
use hourmin::HourMin;
use tracing::info;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct User {
    name: String,
    email: String,
    room: String,
}

struct SessionToken {
    token: String,
    expiry: u64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Debug)]
struct Booking {
    user: User,
    resource: Resource,
    times: [HourMin; 2],
}

pub struct BookingApp {
    sessions: HashMap<User, SessionToken>,
    bookings: Vec<Booking>,
    resources: HashSet<Resource>,
}

impl BookingApp {
    pub fn from_config(config_dir: String) -> Self {
        Self {
            sessions: HashMap::new(),
            bookings: Vec::new(),
            resources: HashSet::new(),
        }
    }

    pub fn handle_new_booking(&mut self, booking: NewBooking) -> Result<(), String> {
        let user = User {
            name: booking.name.clone(),
            email: booking.email.clone(),
            room: booking.room.clone(),
        };

        let resource = self
            .resources
            .get(&Resource::empty(&booking.resource_name))
            .ok_or("Resource not found")?
            .clone();

        let times = [booking.start_time.try_into()?, booking.end_time.try_into()?];
        self.add_booking(Booking {
            user,
            resource,
            times,
        })
    }

    fn add_booking(&mut self, booking: Booking) -> Result<(), String> {
        // if self.bookings.contains(&booking) {
        //     return Err("Booking already exists".to_string());
        // }
        info!("Adding booking: {:?}", booking);
        self.bookings.push(booking);
        Ok(())
    }
}
