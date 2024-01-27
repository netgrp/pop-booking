#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HourMin {
    hour: u8,
    min: u8,
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
