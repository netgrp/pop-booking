use super::hourmin::HourMin;
use super::json_db::JsonDb;
use anyhow::{anyhow, Result};
use chrono::{DateTime, Datelike, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{self};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use tracing::{debug, info};

#[derive(Debug, Deserialize, JsonSchema)]
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

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChangeBooking {
    pub ids: Vec<u32>,
    #[serde(deserialize_with = "parse_rfc3339")]
    pub start_time: DateTime<Utc>,
    #[serde(deserialize_with = "parse_rfc3339")]
    pub end_time: DateTime<Utc>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeletePayload {
    pub ids: Vec<u32>,
}



#[derive(Debug, PartialEq, Eq, Hash, Clone, Serialize, Deserialize, JsonSchema)]
pub struct User {
    username: String,
    room: u16,
}

impl User {
    pub fn new(username: String, room: u16) -> Self {
        Self { username, room }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize, JsonSchema)]
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
    depends_on: Option<String>,
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
    group_id: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct BookingsDb {
    version: u32,
    bookings: HashMap<u32, Booking>,
}

impl Default for BookingsDb {
    fn default() -> Self {
        Self {
            version: 2,
            bookings: HashMap::new(),
        }
    }
}

pub struct BookingApp {
    db: JsonDb<BookingsDb>,
    resources: HashMap<String, Resource>,
    resource_periods: HashMap<String, ResourcePeriod>,
}

// Rebuild cache
#[derive(Serialize, JsonSchema)]
pub struct Event {
    title: String,
    id: u32,
    group_id: u32,
    start: String,
    end: String,
    owner: u16,
    color: String,
}

#[derive(Serialize, Debug, JsonSchema)]
pub struct BookableResource {
    name: String,
    disallowed_periods: Option<Vec<ResourcePeriod>>,
    depends_on: Option<String>,
}

impl BookingApp {
    pub async fn from_config(config_dir: &str, bookings_dir: &str) -> Result<Self> {
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

        let bookings_path = format!("{bookings_dir}/bookings.json");

        // Try to migrate from old format (raw HashMap<u32, Booking without group_id>)
        if let Ok(content) = std::fs::read_to_string(&bookings_path) {
            if !content.trim().is_empty() {
                // Try parsing as new format first
                if serde_json::from_str::<BookingsDb>(&content).is_err() {
                    // Try parsing as old format (HashMap with bookings missing group_id)
                    #[derive(Deserialize)]
                    struct OldBooking {
                        user: User,
                        resource_name: String,
                        start_time: DateTime<Utc>,
                        end_time: DateTime<Utc>,
                    }

                    if let Ok(old_bookings) =
                        serde_json::from_str::<HashMap<u32, OldBooking>>(&content)
                    {
                        info!("Migrating bookings database from v1 to v2 (adding group_id)");

                        // Backup before migrating
                        let backup_path = format!("{bookings_dir}/bookings.json.v1.bak");
                        std::fs::write(&backup_path, &content)
                            .map_err(|e| anyhow!("Failed to create backup before migration: {}", e))?;
                        info!("Backup saved to {}", backup_path);

                        // Group bookings by (user, start_time, end_time)
                        let mut groups: HashMap<(User, DateTime<Utc>, DateTime<Utc>), u32> =
                            HashMap::new();

                        let mut new_bookings: HashMap<u32, Booking> = HashMap::new();
                        for (id, old) in old_bookings {
                            let key = (old.user.clone(), old.start_time, old.end_time);
                            let group_id =
                                *groups.entry(key).or_insert_with(rand::random::<u32>);
                            new_bookings.insert(
                                id,
                                Booking {
                                    user: old.user,
                                    resource_name: old.resource_name,
                                    start_time: old.start_time,
                                    end_time: old.end_time,
                                    group_id,
                                },
                            );
                        }

                        let migrated = BookingsDb {
                            version: 2,
                            bookings: new_bookings,
                        };

                        let migrated_json = serde_json::to_string_pretty(&migrated)?;
                        std::fs::write(&bookings_path, migrated_json)?;
                        info!("Migration complete");
                    }
                }
            }
        }

        let db = JsonDb::open(&bookings_path)
            .await
            .map_err(|e| anyhow!("Failed to open bookings database: {}", e))?;
        Ok(Self {
            db,
            resources,
            resource_periods,
        })
    }

    pub fn get_resources(&self) -> Result<HashMap<String, BookableResource>> {
        self.resources
            .iter()
            .map(|(name, resource)| {
                Ok((
                    name.clone(),
                    BookableResource {
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
                        depends_on: resource.depends_on.clone(),
                    },
                ))
            })
            .collect::<Result<HashMap<String, BookableResource>>>()
            .map_err(|e| anyhow!("Failed to create resource hashmap: {e}"))
    }

    // https://fullcalendar.io/docs/event-parsing
    pub fn get_bookings(
        &self,
        range_start: Option<DateTime<Utc>>,
        range_end: Option<DateTime<Utc>>,
    ) -> Result<Vec<Event>, serde_json::Error> {
        let events = self.db.read(|db| {
            db.bookings
                .iter()
                .filter(|(_, booking)| {
                    if let Some(start) = range_start {
                        if booking.end_time <= start {
                            return false;
                        }
                    }
                    if let Some(end) = range_end {
                        if booking.start_time >= end {
                            return false;
                        }
                    }
                    true
                })
                .map(|(&id, booking)| Event {
                    title: format!(
                        "Room {}, {}",
                        booking.user.room, self.resources[&booking.resource_name].name
                    ),
                    id,
                    group_id: booking.group_id,
                    start: booking.start_time.to_rfc3339(),
                    end: booking.end_time.to_rfc3339(),
                    owner: booking.user.room,
                    color: self.resources[&booking.resource_name].color.clone(),
                })
                .collect::<Vec<Event>>()
        });

        Ok(events)
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
                let end_month_day = (booking.end_time.month(), booking.end_time.day());

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
                    false
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

        if self.db.read(|db| {
            db.bookings.iter().any(|(&id, existing_booking)| {
                booking.resource_name == existing_booking.resource_name
                    && booking.start_time < existing_booking.end_time
                    && booking.end_time > existing_booking.start_time
                    && Some(id) != allow_id
            })
        }) {
            return Err("Booking overlaps with another booking".to_string());
        }

        // Hack to insert meetingroom rule. A real solution is an interpretable scripting language
        // for defining rules :/ Not really feasible though. Otherwise configs should be read at compile time,
        // but that's not really feasible either.
        if booking.resource_name == "meetingroom" {
            // 1: Check booking is within 3 weeks
            let now = Utc::now();
            let three_weeks = now + chrono::Duration::weeks(3);
            if booking.start_time > three_weeks {
                return Err(
                    "Booking is more than 3 weeks in the future. Not allowed for the meeting room."
                        .to_string(),
                );
            }
            // 2: check that no more than 2 bookings are made in the future by the same user
            else {
                let user_bookings = self.db.read(|db| {
                    db.bookings
                        .values()
                        .filter(|prior_booking| {
                            prior_booking.user == booking.user
                                && prior_booking.resource_name == "meetingroom"
                                && prior_booking.end_time > now
                        })
                        .count()
                });

                if user_bookings >= 2 {
                    return Err("Attempt to create more than 2 bookings in the future. Not allowed for the meeting room.".to_string());
                }
            }
        }

        Ok(())
    }

    pub async fn handle_new_booking(
        &mut self,
        booking: NewBooking,
        user: &User,
    ) -> Result<String, String> {
        // Assert that the booking is within the allowed times

        if booking.start_time < Utc::now() {
            return Err("Start time is in the past".to_string());
        }

        if booking.start_time > booking.end_time {
            return Err("Start time is after end time".to_string());
        }

        //handle empty case
        if booking.resource_names.is_empty() {
            return Err("No resources selected".to_string());
        }

        let start_time = booking.start_time;
        let end_time = booking.end_time;
        let mut created_ids: Vec<u32> = Vec::new();
        let group_id: u32 = rand::random();

        for resource_name in &booking.resource_names {
            // Check depends_on constraint
            if let Some(resource) = self.resources.get(resource_name.as_str()) {
                if let Some(ref parent_id) = resource.depends_on {
                    // Parent must be in the same booking request OR already booked by same user at same time
                    let parent_in_request = booking.resource_names.contains(parent_id);
                    if !parent_in_request {
                        let parent_booked = self.db.read(|db| {
                            db.bookings.values().any(|b| {
                                b.resource_name == *parent_id
                                    && b.user == *user
                                    && b.start_time == start_time
                                    && b.end_time == end_time
                            })
                        });
                        if !parent_booked {
                            let parent_name = self
                                .resources
                                .get(parent_id.as_str())
                                .map(|r| r.name.as_str())
                                .unwrap_or(parent_id.as_str());
                            return Err(format!(
                                "{} can only be booked together with {}",
                                resource.name, parent_name
                            ));
                        }
                    }
                }
            }
        }

        for resource_name in booking.resource_names {
            let new_booking = Booking {
                user: user.clone(),
                resource_name,
                start_time,
                end_time,
                group_id,
            };

            if let Err(e) = self.check_available(None, &new_booking) {
                for id in &created_ids {
                    let _ = self.delete_booking(id).await;
                }
                return Err(e);
            }

            let mut id: u32 = rand::random();
            while self.db.read(|db| db.bookings.contains_key(&id)) {
                id = rand::random();
            }

            if let Err(e) = self.add_booking(&id, new_booking).await {
                for prev_id in &created_ids {
                    let _ = self.delete_booking(prev_id).await;
                }
                return Err(e);
            }

            created_ids.push(id);
        }

        Ok("Booking successful".to_string())
    }
    pub async fn handle_delete(&mut self, payload: DeletePayload) -> Result<(), String> {
        let mut all_ids_to_delete: Vec<u32> = Vec::new();

        for id in &payload.ids {
            // Single read to check existence, past-booking, and extract info
            let booking_info = self.db.read(|db| {
                db.bookings.get(id).map(|b| {
                    (b.start_time < Utc::now(), b.resource_name.clone(), b.user.clone(), b.start_time, b.end_time)
                })
            });

            let Some((is_past, resource_name, user, start_time, end_time)) = booking_info else {
                continue; // Skip already-deleted (e.g. cascaded by a previous item)
            };

            if is_past {
                return Err("Cannot delete a booking in the past".to_string());
            }

            let dependent_resource_ids: Vec<String> = self
                .resources
                .iter()
                .filter(|(_, r)| r.depends_on.as_deref() == Some(resource_name.as_str()))
                .map(|(rid, _)| rid.to_string())
                .collect();

            let dependent_booking_ids: Vec<u32> = self.db.read(|db| {
                db.bookings
                    .iter()
                    .filter(|(_, b)| {
                        dependent_resource_ids.contains(&b.resource_name)
                            && b.user == user
                            && b.start_time == start_time
                            && b.end_time == end_time
                    })
                    .map(|(bid, _)| *bid)
                    .collect()
            });

            if !all_ids_to_delete.contains(id) {
                all_ids_to_delete.push(*id);
            }
            for dep_id in dependent_booking_ids {
                if !all_ids_to_delete.contains(&dep_id) {
                    all_ids_to_delete.push(dep_id);
                }
            }
        }

        self.db
            .update(|db| {
                for id in &all_ids_to_delete {
                    db.bookings.remove(id);
                }
            })
            .await
            .map_err(|e| format!("Error deleting bookings: {}", e))?;

        Ok(())
    }

    pub fn assert_ids(&self, ids: &[u32], user: &User) -> bool {
        self.db.read(|db| {
            ids.iter().all(|id| {
                db.bookings.get(id).is_some_and(|b| b.user == *user)
            })
        })
    }

    async fn add_booking(&mut self, id: &u32, booking: Booking) -> Result<(), String> {
        debug!("Adding booking: {:?}", booking);
        self.db
            .update(|db| {
                db.bookings.insert(*id, booking);
            })
            .await
            .map_err(|e| format!("Error adding booking: {}", e))?;
        Ok(())
    }

    async fn delete_booking(&mut self, id: &u32) -> Result<(), String> {
        if !self.db.read(|db| db.bookings.contains_key(id)) {
            return Err("Booking does not exist".to_string());
        }

        self.db
            .update(|db| {
                db.bookings.remove(id);
            })
            .await
            .map_err(|e| format!("Error deleting booking: {}", e))
    }

    pub async fn handle_change_booking(
        &mut self,
        change_booking: ChangeBooking,
    ) -> Result<(), String> {
        for id in &change_booking.ids {
            //check that booking exists
            debug!("Checking if booking exists");
            let mut booking = self.db.read(|db| {
                db.bookings
                    .get(id)
                    .cloned()
                    .ok_or("Booking does not exist".to_string())
            })?;

            booking.start_time = change_booking.start_time;
            booking.end_time = change_booking.end_time;

            //check that booking is within allowed times
            debug!("Checking if booking is within allowed times");
            self.check_available(Some(*id), &booking)?;

            //update booking
            debug!("Updating booking");
            self.add_booking(id, booking).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup_app() -> (BookingApp, TempDir) {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();

        let resources = r##"{
            "sauna": { "name": "Sauna", "description": "", "max_duration": "10:00", "color": "#838800" },
            "hottub": { "name": "Hot tub", "description": "", "max_duration": "10:00", "color": "#003ad9", "depends_on": "sauna" }
        }"##;
        std::fs::write(config_dir.join("resources.json"), resources).unwrap();
        std::fs::write(config_dir.join("resource_periods.json"), "{}").unwrap();

        let bookings_dir = dir.path().join("bookings");
        std::fs::create_dir_all(&bookings_dir).unwrap();

        let app = BookingApp::from_config(
            config_dir.to_str().unwrap(),
            bookings_dir.to_str().unwrap(),
        )
        .await
        .unwrap();

        (app, dir)
    }

    fn test_user() -> User {
        User {
            username: "test".to_string(),
            room: 101,
        }
    }

    fn future_time(hours_from_now: i64) -> DateTime<Utc> {
        Utc::now() + chrono::Duration::hours(hours_from_now)
    }

    #[tokio::test]
    async fn test_new_booking_assigns_group_id() {
        let (mut app, _dir) = setup_app().await;
        let user = test_user();

        let booking = NewBooking {
            resource_names: vec!["sauna".to_string(), "hottub".to_string()],
            start_time: future_time(1),
            end_time: future_time(3),
        };

        app.handle_new_booking(booking, &user).await.unwrap();

        let bookings: Vec<Booking> = app.db.read(|db| db.bookings.values().cloned().collect());
        assert_eq!(bookings.len(), 2);
        assert_eq!(bookings[0].group_id, bookings[1].group_id);
    }

    #[tokio::test]
    async fn test_separate_bookings_get_different_group_ids() {
        let (mut app, _dir) = setup_app().await;
        let user = test_user();

        let booking1 = NewBooking {
            resource_names: vec!["sauna".to_string()],
            start_time: future_time(1),
            end_time: future_time(3),
        };
        let booking2 = NewBooking {
            resource_names: vec!["sauna".to_string()],
            start_time: future_time(5),
            end_time: future_time(7),
        };

        app.handle_new_booking(booking1, &user).await.unwrap();
        app.handle_new_booking(booking2, &user).await.unwrap();

        let bookings: Vec<Booking> = app.db.read(|db| db.bookings.values().cloned().collect());
        assert_eq!(bookings.len(), 2);
        assert_ne!(bookings[0].group_id, bookings[1].group_id);
    }

    #[tokio::test]
    async fn test_bulk_delete() {
        let (mut app, _dir) = setup_app().await;
        let user = test_user();

        let booking = NewBooking {
            resource_names: vec!["sauna".to_string(), "hottub".to_string()],
            start_time: future_time(1),
            end_time: future_time(3),
        };
        app.handle_new_booking(booking, &user).await.unwrap();

        let ids: Vec<u32> = app.db.read(|db| db.bookings.keys().copied().collect());
        assert_eq!(ids.len(), 2);

        app.handle_delete(DeletePayload { ids }).await.unwrap();

        let remaining = app.db.read(|db| db.bookings.len());
        assert_eq!(remaining, 0);
    }

    #[tokio::test]
    async fn test_bulk_delete_cascades_dependents() {
        let (mut app, _dir) = setup_app().await;
        let user = test_user();

        let booking = NewBooking {
            resource_names: vec!["sauna".to_string(), "hottub".to_string()],
            start_time: future_time(1),
            end_time: future_time(3),
        };
        app.handle_new_booking(booking, &user).await.unwrap();

        // Only delete sauna — hottub should cascade
        let sauna_id = app.db.read(|db| {
            db.bookings
                .iter()
                .find(|(_, b)| b.resource_name == "sauna")
                .map(|(id, _)| *id)
                .unwrap()
        });

        app.handle_delete(DeletePayload {
            ids: vec![sauna_id],
        })
        .await
        .unwrap();

        let remaining = app.db.read(|db| db.bookings.len());
        assert_eq!(remaining, 0);
    }

    #[tokio::test]
    async fn test_bulk_change() {
        let (mut app, _dir) = setup_app().await;
        let user = test_user();

        let booking = NewBooking {
            resource_names: vec!["sauna".to_string(), "hottub".to_string()],
            start_time: future_time(1),
            end_time: future_time(3),
        };
        app.handle_new_booking(booking, &user).await.unwrap();

        let ids: Vec<u32> = app.db.read(|db| db.bookings.keys().copied().collect());
        let new_start = future_time(10);
        let new_end = future_time(12);

        app.handle_change_booking(ChangeBooking {
            ids,
            start_time: new_start,
            end_time: new_end,
        })
        .await
        .unwrap();

        let bookings: Vec<Booking> = app.db.read(|db| db.bookings.values().cloned().collect());
        for b in &bookings {
            assert_eq!(b.start_time, new_start);
            assert_eq!(b.end_time, new_end);
        }
    }

    #[tokio::test]
    async fn test_migration_from_v1() {
        let dir = TempDir::new().unwrap();
        let config_dir = dir.path().join("config");
        std::fs::create_dir_all(&config_dir).unwrap();

        let resources = r##"{
            "sauna": { "name": "Sauna", "description": "", "max_duration": "10:00", "color": "#838800" }
        }"##;
        std::fs::write(config_dir.join("resources.json"), resources).unwrap();
        std::fs::write(config_dir.join("resource_periods.json"), "{}").unwrap();

        let bookings_dir = dir.path().join("bookings");
        std::fs::create_dir_all(&bookings_dir).unwrap();

        // Write old format (no group_id, raw HashMap)
        let old_data = r#"{
            "1": { "user": { "username": "alice", "room": 101 }, "resource_name": "sauna", "start_time": "2026-06-01T10:00:00Z", "end_time": "2026-06-01T12:00:00Z" },
            "2": { "user": { "username": "alice", "room": 101 }, "resource_name": "sauna", "start_time": "2026-06-01T10:00:00Z", "end_time": "2026-06-01T12:00:00Z" },
            "3": { "user": { "username": "bob", "room": 202 }, "resource_name": "sauna", "start_time": "2026-06-01T10:00:00Z", "end_time": "2026-06-01T12:00:00Z" }
        }"#;
        std::fs::write(bookings_dir.join("bookings.json"), old_data).unwrap();

        let app = BookingApp::from_config(
            config_dir.to_str().unwrap(),
            bookings_dir.to_str().unwrap(),
        )
        .await
        .unwrap();

        let bookings: Vec<(u32, Booking)> =
            app.db.read(|db| db.bookings.iter().map(|(k, v)| (*k, v.clone())).collect());

        assert_eq!(bookings.len(), 3);

        let b1 = bookings.iter().find(|(id, _)| *id == 1).unwrap();
        let b2 = bookings.iter().find(|(id, _)| *id == 2).unwrap();
        let b3 = bookings.iter().find(|(id, _)| *id == 3).unwrap();

        // Same user + same time = same group_id
        assert_eq!(b1.1.group_id, b2.1.group_id);
        // Different user = different group_id
        assert_ne!(b1.1.group_id, b3.1.group_id);

        let version = app.db.read(|db| db.version);
        assert_eq!(version, 2);
    }

    #[tokio::test]
    async fn test_assert_ids_rejects_non_owner() {
        let (mut app, _dir) = setup_app().await;
        let user = test_user();
        let other_user = User {
            username: "other".to_string(),
            room: 202,
        };

        let booking = NewBooking {
            resource_names: vec!["sauna".to_string()],
            start_time: future_time(1),
            end_time: future_time(3),
        };
        app.handle_new_booking(booking, &user).await.unwrap();

        let ids: Vec<u32> = app.db.read(|db| db.bookings.keys().copied().collect());
        assert!(app.assert_ids(&ids, &user));
        assert!(!app.assert_ids(&ids, &other_user));
    }
}
