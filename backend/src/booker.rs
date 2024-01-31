use crate::authenticate::SessionToken;
use crate::hourmin::HourMin;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::env;
use std::hash::{Hash, Hasher};
use tracing::{debug, info};

#[derive(Debug, Deserialize)]
pub struct NewBooking {
    pub resource_name: String,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Deserialize)]
pub struct DeletePayload {
    pub id: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize)]
pub struct User {
    username: String,
    room: u16,
}

impl User {
    pub fn new(username: String, room: u16) -> Self {
        Self { username, room }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct ResourcePeriod {
    //month and date
    #[serde(deserialize_with = "month_day_from_str")]
    start: (u32, u32),
    #[serde(deserialize_with = "month_day_from_str")]
    end: (u32, u32),
}

impl std::fmt::Display for ResourcePeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02}-{:02} - {:02}-{:02}",
            self.start.0, self.start.1, self.end.0, self.end.1
        )
    }
}

fn month_day_from_str<'de, D>(deserializer: D) -> Result<(u32, u32), D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let parts = s.split('-').collect::<Vec<&str>>();
    if parts.len() != 2 {
        return Err(serde::de::Error::custom(format!(
            "Expected format: MM-DD, got: {}",
            s
        )));
    }
    let month = parts[0].parse::<u32>().map_err(serde::de::Error::custom)?;
    let day = parts[1].parse::<u32>().map_err(serde::de::Error::custom)?;
    Ok((month, day))
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
struct Resource {
    name: String,
    description: String,
    allowed_times: Option<[HourMin; 2]>,
    #[serde(deserialize_with = "minutes_from_str")]
    max_duration: u32, //in minutes
    color: String,
    disallowed_periods: Option<Vec<String>>,
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
struct Booking {
    user: User,
    resource_name: String,
    times: [DateTime<Utc>; 2],
}

pub struct BookingApp {
    bookings: HashMap<u32, Booking>,
    resources: HashMap<String, Resource>,
    resource_periods: HashMap<String, ResourcePeriod>,
    cached_resource_json: Option<String>,
}

impl BookingApp {
    pub fn from_config(config_dir: &str) -> Result<Self> {
        //load using serde_json
        let resources_path = format!("{config_dir}/resources.json");
        info!("Loading resources from: {}", resources_path);

        let resources_content = std::fs::read_to_string(resources_path)?;
        let resources: HashMap<String, Resource> = serde_json::from_str(&resources_content)?;

        let resource_periods_path = format!("{config_dir}/resource_periods.json");
        info!("Loading resource periods from: {}", resource_periods_path);

        let resource_periods_content = std::fs::read_to_string(resource_periods_path)?;
        let resource_periods: HashMap<String, ResourcePeriod> =
            serde_json::from_str(&resource_periods_content)?;

        Ok(Self {
            bookings: HashMap::new(),
            resources,
            resource_periods,
            cached_resource_json: None,
        })
    }

    pub fn load_bookings(&mut self, bookings_dir: &str) -> Result<()> {
        //load bookings from file
        let bookings_path = format!("{bookings_dir}/bookings.json");
        info!("Loading bookings from: {}", bookings_path);

        //check if file exists
        if !std::path::Path::new(&bookings_path).exists() {
            info!("Bookings file does not exist, creating empty file");
            std::fs::write(&bookings_path, "{}")?;
        }

        let bookings_content = std::fs::read_to_string(bookings_path)?;
        self.bookings = serde_json::from_str(&bookings_content)
            .map_err(|e| anyhow!("Loading of bookings failed: {}", e))?;

        //build bookings json
        self.cached_resource_json = Some(self.get_event_json().unwrap());

        Ok(())
    }

    pub fn get_resources(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.resources)
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
            id: String,
            start: String,
            end: String,
            owner: u16,
            color: String,
        }

        let bookings_json = self
            .bookings
            .iter()
            .map(|(id, booking)| Event {
                title: format!(
                    "Room {}, {}",
                    booking.user.room, self.resources[&booking.resource_name].name
                ),
                id: id.to_string(),
                start: booking.times[0].to_rfc3339(),
                end: booking.times[1].to_rfc3339(),
                owner: booking.user.room,
                color: self.resources[&booking.resource_name].color.clone(),
            })
            .collect::<Vec<Event>>();

        serde_json::to_string(&bookings_json)
    }

    pub fn handle_new_booking(
        &mut self,
        booking: NewBooking,
        session: SessionToken,
    ) -> Result<String, String> {
        // Assert that the resource exists
        let resource = self
            .resources
            .get(&booking.resource_name)
            .ok_or_else(|| format!("Resource {} does not exist", booking.resource_name.clone()))?;

        // Assert that the booking is within the allowed times
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

        let duration = (times[1] - times[0]).num_minutes() as u32;

        debug!("Duration: {}", duration);
        debug!("Max duration: {}", resource.max_duration);
        if duration > resource.max_duration {
            return Err(format!(
                "Booking duration is longer than allowed: {} > {}",
                HourMin::from_minutes(duration),
                HourMin::from_minutes(resource.max_duration)
            ));
        }

        if let Some(disallowed_periods) = &resource.disallowed_periods {
            for disallowed_period_name in disallowed_periods {
                let disallowed_period = self
                    .resource_periods
                    .get(disallowed_period_name)
                    .ok_or_else(|| {
                        format!(
                            "Disallowed period {} does not exist",
                            disallowed_period_name
                        )
                    })?;

                debug!("Disallowed period: {}", disallowed_period);

                // just check the month and day of the start and end booking time. Max booking length ensures this is ok
                // at least in general

                let start_month_day = (times[0].month(), times[0].day());
                let end_month_day = (times[1].month(), times[1].day());

                debug!("Start month day: {:?}", start_month_day);
                debug!("End month day: {:?}", end_month_day);

                let is_in_range = |start: (u32, u32), end: (u32, u32), target: (u32, u32)| {
                    if start > end {
                        // wrap around the year
                        if target >= start || target <= end {
                            return true;
                        }
                    } else if target >= start && target <= end {
                        return true;
                    }
                    return false;
                };

                if is_in_range(
                    disallowed_period.start,
                    disallowed_period.end,
                    start_month_day,
                ) || is_in_range(
                    disallowed_period.start,
                    disallowed_period.end,
                    end_month_day,
                ) {
                    return Err(format!(
                        "Booking is in {}: {}",
                        disallowed_period_name, disallowed_period
                    ));
                }
            }
        }

        if let Some(allowed_times) = resource.allowed_times {
            let to_start = (allowed_times[1].to_minutes() - HourMin::from(times[0]).to_minutes())
                .rem_euclid(1440) as u32;
            let to_end = (allowed_times[0].to_minutes() - HourMin::from(times[1]).to_minutes())
                .rem_euclid(1440) as u32;

            debug!("To start: {}", to_start);
            debug!("To end: {}", to_end);

            if to_end < to_start {
                return Err(format!(
                    "Booking outside allowed times: {} - {}",
                    allowed_times[0], allowed_times[1]
                ));
            }

            let legal_duration = std::cmp::min(resource.max_duration, to_start);

            debug!("Legal duration: {}", legal_duration);
            if legal_duration < duration {
                return Err(format!(
                    "Booking outside allowed times {} - {}",
                    allowed_times[0], allowed_times[1]
                ));
            }
        }

        if self.bookings.iter().any(|(_, existing_booking)| {
            booking.resource_name == existing_booking.resource_name
                && times[0] < existing_booking.times[1]
                && times[1] > existing_booking.times[0]
        }) {
            return Err("Booking overlaps with another booking".to_string());
        }

        let mut id = rand::random();

        //ensure id is unique. This is definitely not necessary, but just in case
        while self.bookings.contains_key(&id) {
            id = rand::random();
        }

        self.add_booking(
            &id,
            Booking {
                user: session.get_user().to_owned(),
                resource_name: booking.resource_name,
                times,
            },
        )?;
        Ok(id.to_string())
    }

    pub fn handle_delete(&mut self, payload: DeletePayload) -> Result<(), String> {
        let id = payload
            .id
            .parse::<u32>()
            .map_err(|e| format!("Error parsing id: {}", e))?;

        if !self.bookings.contains_key(&id) {
            return Err("Booking does not exist".to_string());
        }

        self.bookings.remove(&id);

        //build bookings json
        self.cached_resource_json = Some(self.get_event_json().unwrap());

        //save to file
        let bookings_path = format!(
            "{}/bookings.json",
            env::var("BOOKINGS_DIR").map_err(|e| format!("Error getting bookings dir: {}", e))?
        );
        info!("Saving bookings to: {}", bookings_path);

        let bookings_content = serde_json::to_string_pretty(&self.bookings).unwrap();
        std::fs::write(bookings_path, bookings_content).unwrap();

        Ok(())
    }

    fn add_booking(&mut self, id: &u32, booking: Booking) -> Result<(), String> {
        // if self.bookings.contains(&booking) {
        //     return Err("Booking already exists".to_string());
        // }
        debug!("Adding booking: {:?}", booking);
        self.bookings.insert(*id, booking);

        //build bookings json
        self.cached_resource_json = Some(self.get_event_json().unwrap());

        //save to file
        let bookings_path = format!(
            "{}/bookings.json",
            env::var("BOOKINGS_DIR").map_err(|e| format!("Error getting bookings dir: {}", e))?
        );
        info!("Saving bookings to: {}", bookings_path);

        let bookings_content = serde_json::to_string_pretty(&self.bookings).unwrap();
        std::fs::write(bookings_path, bookings_content).unwrap();

        Ok(())
    }
}
