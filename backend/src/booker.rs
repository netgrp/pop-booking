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
    #[serde(deserialize_with = "minutes_from_str")]
    max_duration: u32, //in minutes
    color: String,
}

fn minutes_from_str<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let parts = s.split(':').collect::<Vec<&str>>();
    if parts.len() != 2 {
        return Err(serde::de::Error::custom(format!(
            "Expected format: HH:MM, got: {}",
            s
        )));
    }
    let hours = parts[0].parse::<u32>().map_err(serde::de::Error::custom)?;
    let minutes = parts[1].parse::<u32>().map_err(serde::de::Error::custom)?;
    Ok(hours * 60 + minutes)
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
        let resource = self
            .resources
            .get(&booking.resource_name)
            .ok_or_else(|| format!("Resource {} does not exist", booking.resource_name.clone()))?;

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

        let inrange = |time: DateTime<Utc>, range: [HourMin; 2]| {
            HourMin::from(time) >= std::cmp::min(range[0], range[1])
                && HourMin::from(time) <= std::cmp::max(range[0], range[1])
        };

        let order = allowed_times[0] > allowed_times[1];
        let duration = (times[1] - times[0]).num_minutes() as u32;

        if duration > resource.max_duration {
            return Err(format!(
                "Booking duration is longer than allowed: {} > {}",
                duration, resource.max_duration
            ));
        }

        if order == inrange(times[0], allowed_times) && order == inrange(times[1], allowed_times) {
            return Err(format!(
                "Booking outside allowed times: {} - {}",
                allowed_times[0], allowed_times[1]
            ));
        }

        //Check that the booking is within the allowed times again, but different, hard to explain
        let legal_duration = std::cmp::min(
            resource.max_duration,
            (allowed_times[1].to_minutes() - HourMin::from(times[0]).to_minutes()).rem_euclid(1440)
                as u32,
        );

        //last check to see whether the booking crosses an illegal time
        if legal_duration < duration {
            return Err(format!(
                "Booking outside allowed times {} - {}",
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