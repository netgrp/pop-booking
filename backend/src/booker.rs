use crate::hourmin::HourMin;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tracing::info;

#[derive(Deserialize)]
pub struct NewBooking {
    pub name: String,
    pub email: String,
    pub room: u8,
    pub resource_name: String,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
struct User {
    name: String,
    email: String,
    room: u8,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Resource {
    name: String,
    description: String,
    allowed_times: [HourMin; 2],
}

// Hashing only by name, must be unique
impl Hash for Resource {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Booking {
    user: User,
    resource_name: String,
    times: [DateTime<Utc>; 2],
}

pub struct BookingApp {
    bookings: Vec<Booking>,
    resources: HashMap<String, Resource>,
    cached_resource_json: Option<String>,
}

impl BookingApp {
    pub fn from_config(config_dir: &str) -> Result<Self, Box<dyn std::error::Error>> {
        //load using serde_json
        let resources_path = format!("{config_dir}/resources.json");
        info!("Loading resources from: {}", resources_path);

        let resources_content = std::fs::read_to_string(resources_path)?;
        let resources: HashMap<String, Resource> = serde_json::from_str(&resources_content)?;

        Ok(Self {
            bookings: Vec::new(),
            resources: resources,
            cached_resource_json: None,
        })
    }

    pub fn load_bookings(&mut self, bookings_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        //load bookings from file
        let bookings_path = format!("{bookings_dir}/bookings.json");
        info!("Loading bookings from: {}", bookings_path);

        let bookings_content = std::fs::read_to_string(bookings_path)?;
        self.bookings = serde_json::from_str(&bookings_content)?;

        Ok(())
    }

    pub fn get_resources(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.resources)
    }

    // https://fullcalendar.io/docs/event-parsing
    pub fn get_bookings(&self) -> String {
        if let Some(cached_string) = &self.cached_resource_json {
            return cached_string.clone();
        }

        //rebuild cache
        struct Event {
            title: String,
            start: String,
            end: String,
        }

        todo!()
    }

    pub fn handle_new_booking(&mut self, booking: NewBooking) -> Result<(), String> {
        let user = User {
            name: booking.name.clone(),
            email: booking.email.clone(),
            room: booking.room.clone(),
        };

        //Assert that the resource exists
        if !self.resources.contains_key(&booking.resource_name) {
            return Err(format!(
                "Resource does not exist: {}",
                booking.resource_name
            ));
        }

        //https://xkcd.com/1179/
        let times = [
            DateTime::parse_from_rfc3339(&booking.start_time)
                .map_err(|e| format!("Error parsing start time: {}", e))?
                .with_timezone(&Utc),
            DateTime::parse_from_rfc3339(&booking.end_time)
                .map_err(|e| format!("Error parsing end time: {}", e))?
                .with_timezone(&Utc),
        ];

        self.add_booking(Booking {
            user,
            resource_name: booking.resource_name,
            times,
        })
    }

    fn add_booking(&mut self, booking: Booking) -> Result<(), String> {
        // if self.bookings.contains(&booking) {
        //     return Err("Booking already exists".to_string());
        // }
        info!("Adding booking: {:?}", booking);
        self.bookings.push(booking);

        //invalidate cache
        self.cached_resource_json = None;
        Ok(())
    }
}
