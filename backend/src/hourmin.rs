use core::fmt;

use chrono::{DateTime, Local, Timelike, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct HourMin {
    hour: u8,
    min: u8,
}

impl<'de> Deserialize<'de> for HourMin {
    fn deserialize<D>(deserializer: D) -> Result<HourMin, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        HourMin::try_from(s).map_err(serde::de::Error::custom)
    }
}

impl TryFrom<&str> for HourMin {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        //ensure only 5 chars
        if value.len() != 5 {
            return Err(format!("Invalid length: {}", value.len()));
        }
        let mut parts = value.split(':');
        let hour = parts
            .next()
            .ok_or_else(|| "Missing hour".to_string())?
            .parse::<u8>()
            .map_err(|e| format!("Invalid hour: {}", e))?;
        if hour > 23 {
            return Err(format!("Invalid hour, value too high: {}", hour));
        }

        let min = parts
            .next()
            .ok_or_else(|| "Missing min".to_string())?
            .parse::<u8>()
            .map_err(|e| format!("Invalid min: {}", e))?;
        if min > 59 {
            return Err(format!("Invalid min, value too high: {}", min));
        }
        Ok(Self { hour, min })
    }
}

impl TryFrom<String> for HourMin {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl HourMin {
    pub fn now() -> Self {
        Self::from(Utc::now())
    }

    pub fn hour(&self) -> u8 {
        self.hour
    }

    pub fn min(&self) -> u8 {
        self.min
    }

    pub fn to_minutes(&self) -> i32 {
        self.hour as i32 * 60 + self.min as i32
    }

    pub fn from_minutes(minutes: u32) -> Self {
        //roll over if minutes > 24h
        let minutes = minutes % (24 * 60);

        Self {
            hour: (minutes / 60) as u8,
            min: (minutes % 60) as u8,
        }
    }
}

//Sadly I have to assume local timezone here
impl From<DateTime<Utc>> for HourMin {
    fn from(dt: DateTime<Utc>) -> Self {
        //convert to local time
        let dt = dt.with_timezone(&Local);
        Self {
            hour: dt.hour() as u8,
            min: dt.minute() as u8,
        }
    }
}

impl fmt::Display for HourMin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}:{:02}", self.hour, self.min)
    }
}
