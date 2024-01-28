use crate::hourmin::HourMin;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::env;
use std::hash::{Hash, Hasher};
use tracing::info;

#[derive(Debug, Deserialize)]
pub struct NewBooking {
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
    color: String,
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
            resources,
            cached_resource_json: None,
        })
    }

    pub fn load_bookings(&mut self, bookings_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
        //load bookings from file
        let bookings_path = format!("{bookings_dir}/bookings.json");
        info!("Loading bookings from: {}", bookings_path);

        let bookings_content = std::fs::read_to_string(bookings_path)?;
        self.bookings = serde_json::from_str(&bookings_content)?;

        //build bookings json
        self.cached_resource_json = Some(self.get_event_json().unwrap());

        Ok(())
    }

    pub fn get_resources(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.resources)
    }

    // https://fullcalendar.io/docs/event-parsing
    pub fn get_bookings(&self) -> Result<String, serde_json::Error> {
        if let Some(cached_string) = &self.cached_resource_json {
            return Ok(cached_string.clone());
        }

        self.get_event_json()
    }

    fn get_event_json(&self) -> Result<String, serde_json::Error> {
        // Rebuild cache
        #[derive(Serialize)]
        struct Event {
            title: String,
            start: String,
            end: String,
            color: String,
        }

        let bookings_json = self
            .bookings
            .iter()
            .map(|booking| Event {
                title: format!(
                    "Room {}, {}",
                    booking.user.room, self.resources[&booking.resource_name].name
                ),
                start: booking.times[0].to_rfc3339(),
                end: booking.times[1].to_rfc3339(),
                color: self.resources[&booking.resource_name].color.clone(),
            })
            .collect::<Vec<Event>>();

        serde_json::to_string(&bookings_json)
    }

    pub fn handle_new_booking(&mut self, booking: NewBooking) -> Result<(), String> {
        let user = User {
            name: "John Doe".to_string(), //TODO: get from auth
            email: "John@doe.com".to_string(),
            room: 42,
        };

        //Assert that the resource exists
        if !self.resources.contains_key(&booking.resource_name) {
            return Err(format!(
                "Resource does not exist: {}",
                booking.resource_name
            ));
        }

        //Assert that the booking is within the allowed times
        let allowed_times = self.resources[&booking.resource_name].allowed_times;

        //https://xkcd.com/1179/
        let times = [
            DateTime::parse_from_rfc3339(&booking.start_time)
                .map_err(|e| format!("Error parsing start time: {}", e))?
                .with_timezone(&Utc),
            DateTime::parse_from_rfc3339(&booking.end_time)
                .map_err(|e| format!("Error parsing end time: {}", e))?
                .with_timezone(&Utc),
        ];

        if times[0] < Utc::now() {
            return Err("Start time is in the past".to_string());
        }

        if times[0] > times[1] {
            return Err("Start time is after end time".to_string());
        }

        let inrange = |time: DateTime<Utc>| {
            HourMin::from(time) >= std::cmp::min(allowed_times[0], allowed_times[1])
                && HourMin::from(time) <= std::cmp::max(allowed_times[0], allowed_times[1])
        };

        let order = allowed_times[0] > allowed_times[1];
        if order == inrange(times[0]) && order == inrange(times[1]) {
            return Err(format!(
                "Booking outside allowed times: {} - {}",
                allowed_times[0], allowed_times[1]
            ));
        }

        //check if it overlaps with any other bookings
        // TODO: implement data structure that allows for fast lookup of overlapping bookings
        // I want to use a BTreeMap for this, that would be O(nlogn)
        if self
            .bookings
            .iter()
            .filter(|existing_booking| {
                (booking.resource_name == existing_booking.resource_name)
                    && (times[0] < existing_booking.times[1]
                        && times[1] > existing_booking.times[0])
            })
            .count()
            > 0
        {
            return Err("Booking overlaps with another booking".to_string());
        }

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

        //build bookings json
        self.cached_resource_json = Some(self.get_event_json().unwrap());

        //save to file
        let bookings_path = format!(
            "{}/bookings.json",
            env::var("BOOKINGS_DIR").map_err(|e| format!("Error getting bookings dir: {}", e))?
        );
        info!("Saving bookings to: {}", bookings_path);

        let bookings_content = serde_json::to_string(&self.bookings).unwrap();
        std::fs::write(bookings_path, bookings_content).unwrap();

        Ok(())
    }
}
