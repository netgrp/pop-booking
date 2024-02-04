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
    pub resource_names: Vec<String>,
    #[serde(deserialize_with = "parse_rfc3339")]
    pub start_time: DateTime<Utc>,
    #[serde(deserialize_with = "parse_rfc3339")]
    pub end_time: DateTime<Utc>,
}

fn parse_rfc3339<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map_err(serde::de::Error::custom)
        .map(|dt| dt.with_timezone(&Utc))
}

#[derive(Debug, Deserialize)]
pub struct ChangeBooking {
    pub id: u32,
    #[serde(deserialize_with = "parse_rfc3339")]
    pub start_time: DateTime<Utc>,
    #[serde(deserialize_with = "parse_rfc3339")]
    pub end_time: DateTime<Utc>,
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
struct Booking {
    user: User,
    resource_name: String,
    start_time: DateTime<Utc>,
    end_time: DateTime<Utc>,
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

    pub fn get_resources(&self) -> Result<String> {
        #[derive(Serialize, Debug)]
        struct Resource {
            name: String,
            disallowed_periods: Option<Vec<ResourcePeriod>>,
        }
        let resources = &self
            .resources
            .iter()
            .map(|(name, resource)| {
                Ok((
                    name.clone(),
                    Resource {
                        name: resource.name.clone(),
                        disallowed_periods: resource
                            .disallowed_periods
                            .clone()
                            .map(|periods| {
                                periods
                                    .iter()
                                    .map(|period_name| {
                                        Ok(self
                                            .resource_periods
                                            .get(period_name)
                                            .ok_or(anyhow!("resource period not found"))?
                                            .clone())
                                    })
                                    .collect::<Result<Vec<ResourcePeriod>>>()
                            })
                            .transpose()?,
                    },
                ))
            })
            .collect::<Result<HashMap<String, Resource>>>()
            .map_err(|e| anyhow!("Failed to create resource json {e}"))?;
        serde_json::to_string_pretty(resources)
            .map_err(|e| anyhow!("Failed to serialize json: {}", e))
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
                start: booking.start_time.to_rfc3339(),
                end: booking.end_time.to_rfc3339(),
                owner: booking.user.room,
                color: self.resources[&booking.resource_name].color.clone(),
            })
            .collect::<Vec<Event>>();

        serde_json::to_string(&bookings_json)
    }

    fn check_available(&self, allow_id: Option<u32>, booking: &Booking) -> Result<(), String> {
        // Assert that the resources exist
        let duration = (booking.end_time - booking.start_time).num_minutes() as u32;
        let resource = self
            .resources
            .get(booking.resource_name.as_str())
            .ok_or(format!(
                "Resource {} does not exist",
                booking.resource_name.as_str()
            ))?;

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
                let disallowed_period =
                    self.resource_periods.get(disallowed_period_name).ok_or({
                        format!(
                            "Disallowed period {} does not exist",
                            disallowed_period_name
                        )
                    })?;

                debug!("Disallowed period: {}", disallowed_period);

                // just check the month and day of the start and end booking time. Max booking length ensures this is ok
                // at least in general

                let start_month_day = (booking.start_time.month(), booking.start_time.day());
                let end_month_day = (booking.start_time.month(), booking.start_time.day());

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
            let to_start = (allowed_times[1].to_minutes()
                - HourMin::from(booking.start_time).to_minutes())
            .rem_euclid(1440) as u32;
            let to_end = (allowed_times[0].to_minutes()
                - HourMin::from(booking.end_time).to_minutes())
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

        if self.bookings.iter().any(|(&id, existing_booking)| {
            booking.resource_name == existing_booking.resource_name
                && booking.start_time < existing_booking.end_time
                && booking.end_time > existing_booking.start_time
                && Some(id) != allow_id
        }) {
            return Err("Booking overlaps with another booking".to_string());
        }

        Ok(())
    }

    pub fn handle_new_booking(
        &mut self,
        booking: NewBooking,
        session: SessionToken,
    ) -> Result<String, String> {
        // Assert that the booking is within the allowed times

        if booking.start_time < Utc::now() {
            return Err("Start time is in the past".to_string());
        }

        if booking.start_time > booking.end_time {
            return Err("Start time is after end time".to_string());
        }

        let results = booking
            .resource_names
            .into_iter()
            .map(|resource_name| -> Result<u32, String> {
                let booking = Booking {
                    user: session.get_user().clone(),
                    resource_name,
                    start_time: booking.start_time,
                    end_time: booking.end_time,
                };

                self.check_available(None, &booking)?;

                let mut id = rand::random();

                //ensure id is unique. This is definitely not necessary, but just in case
                while self.bookings.contains_key(&id) {
                    id = rand::random();
                }

                self.add_booking(&id, booking)?;
                Ok(id)
            })
            .collect::<Vec<_>>();

        if results.iter().any(|result| result.is_err()) {
            //all bookings should be successful
            results
                .iter()
                .filter(|result| result.is_ok())
                .for_each(|result| {
                    let id = result.as_ref().unwrap();
                    self.delete_booking(id).unwrap();
                });
            return Err(format!("Error adding booking"));
        }

        Ok("Booking successful".to_string())
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

    fn delete_booking(&mut self, id: &u32) -> Result<(), String> {
        if !self.bookings.contains_key(id) {
            return Err("Booking does not exist".to_string());
        }

        self.bookings.remove(id);

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

    pub fn handle_change_booking(&mut self, change_booking: ChangeBooking) -> Result<(), String> {
        let id = change_booking.id;

        //check that booking exists
        debug!("Checking if booking exists");
        let mut booking = self
            .bookings
            .get(&id)
            .ok_or("Booking does not exist".to_string())?
            .clone();

        booking.start_time = change_booking.start_time;
        booking.end_time = change_booking.end_time;

        //check that booking is within allowed times
        debug!("Checking if booking is within allowed times");
        self.check_available(Some(id), &booking)?;

        //update booking
        debug!("Updating booking");
        self.add_booking(&id, booking)
    }
}
