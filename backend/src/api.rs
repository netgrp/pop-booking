use serde::Deserialize;

#[derive(Deserialize)]
pub struct NewBooking {
    pub name: String,
    pub email: String,
    pub room: u8,
    pub resource_name: String,
    pub start_time: String,
    pub end_time: String,
}
