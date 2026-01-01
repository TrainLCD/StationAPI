use serde::{Deserialize, Serialize};

/// GTFS Agency (Bus operator)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GtfsAgency {
    pub agency_id: String,
    pub agency_name: String,
    pub agency_name_k: Option<String>,
    pub agency_name_r: Option<String>,
    pub agency_name_zh: Option<String>,
    pub agency_name_ko: Option<String>,
    pub agency_url: Option<String>,
    pub agency_timezone: String,
    pub agency_lang: Option<String>,
    pub agency_phone: Option<String>,
    pub agency_fare_url: Option<String>,
    pub company_cd: Option<i32>,
}

impl GtfsAgency {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        agency_id: String,
        agency_name: String,
        agency_name_k: Option<String>,
        agency_name_r: Option<String>,
        agency_name_zh: Option<String>,
        agency_name_ko: Option<String>,
        agency_url: Option<String>,
        agency_timezone: String,
        agency_lang: Option<String>,
        agency_phone: Option<String>,
        agency_fare_url: Option<String>,
        company_cd: Option<i32>,
    ) -> Self {
        Self {
            agency_id,
            agency_name,
            agency_name_k,
            agency_name_r,
            agency_name_zh,
            agency_name_ko,
            agency_url,
            agency_timezone,
            agency_lang,
            agency_phone,
            agency_fare_url,
            company_cd,
        }
    }
}

/// GTFS Route (Bus line)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GtfsRoute {
    pub route_id: String,
    pub agency_id: Option<String>,
    pub route_short_name: Option<String>,
    pub route_long_name: Option<String>,
    pub route_long_name_k: Option<String>,
    pub route_long_name_r: Option<String>,
    pub route_long_name_zh: Option<String>,
    pub route_long_name_ko: Option<String>,
    pub route_desc: Option<String>,
    pub route_type: i32,
    pub route_url: Option<String>,
    pub route_color: Option<String>,
    pub route_text_color: Option<String>,
    pub route_sort_order: Option<i32>,
    pub line_cd: Option<i32>,
}

impl GtfsRoute {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        route_id: String,
        agency_id: Option<String>,
        route_short_name: Option<String>,
        route_long_name: Option<String>,
        route_long_name_k: Option<String>,
        route_long_name_r: Option<String>,
        route_long_name_zh: Option<String>,
        route_long_name_ko: Option<String>,
        route_desc: Option<String>,
        route_type: i32,
        route_url: Option<String>,
        route_color: Option<String>,
        route_text_color: Option<String>,
        route_sort_order: Option<i32>,
        line_cd: Option<i32>,
    ) -> Self {
        Self {
            route_id,
            agency_id,
            route_short_name,
            route_long_name,
            route_long_name_k,
            route_long_name_r,
            route_long_name_zh,
            route_long_name_ko,
            route_desc,
            route_type,
            route_url,
            route_color,
            route_text_color,
            route_sort_order,
            line_cd,
        }
    }
}

/// GTFS Stop (Bus stop)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GtfsStop {
    pub stop_id: String,
    pub stop_code: Option<String>,
    pub stop_name: String,
    pub stop_name_k: Option<String>,
    pub stop_name_r: Option<String>,
    pub stop_name_zh: Option<String>,
    pub stop_name_ko: Option<String>,
    pub stop_desc: Option<String>,
    pub stop_lat: f64,
    pub stop_lon: f64,
    pub zone_id: Option<String>,
    pub stop_url: Option<String>,
    pub location_type: Option<i32>,
    pub parent_station: Option<String>,
    pub stop_timezone: Option<String>,
    pub wheelchair_boarding: Option<i32>,
    pub platform_code: Option<String>,
    pub station_cd: Option<i32>,
}

impl GtfsStop {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stop_id: String,
        stop_code: Option<String>,
        stop_name: String,
        stop_name_k: Option<String>,
        stop_name_r: Option<String>,
        stop_name_zh: Option<String>,
        stop_name_ko: Option<String>,
        stop_desc: Option<String>,
        stop_lat: f64,
        stop_lon: f64,
        zone_id: Option<String>,
        stop_url: Option<String>,
        location_type: Option<i32>,
        parent_station: Option<String>,
        stop_timezone: Option<String>,
        wheelchair_boarding: Option<i32>,
        platform_code: Option<String>,
        station_cd: Option<i32>,
    ) -> Self {
        Self {
            stop_id,
            stop_code,
            stop_name,
            stop_name_k,
            stop_name_r,
            stop_name_zh,
            stop_name_ko,
            stop_desc,
            stop_lat,
            stop_lon,
            zone_id,
            stop_url,
            location_type,
            parent_station,
            stop_timezone,
            wheelchair_boarding,
            platform_code,
            station_cd,
        }
    }
}

/// GTFS Calendar (Service schedule)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GtfsCalendar {
    pub service_id: String,
    pub monday: bool,
    pub tuesday: bool,
    pub wednesday: bool,
    pub thursday: bool,
    pub friday: bool,
    pub saturday: bool,
    pub sunday: bool,
    pub start_date: String,
    pub end_date: String,
}

impl GtfsCalendar {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        service_id: String,
        monday: bool,
        tuesday: bool,
        wednesday: bool,
        thursday: bool,
        friday: bool,
        saturday: bool,
        sunday: bool,
        start_date: String,
        end_date: String,
    ) -> Self {
        Self {
            service_id,
            monday,
            tuesday,
            wednesday,
            thursday,
            friday,
            saturday,
            sunday,
            start_date,
            end_date,
        }
    }

    /// Check if the service runs on a given weekday (0 = Monday, 6 = Sunday)
    pub fn runs_on_weekday(&self, weekday: u32) -> bool {
        match weekday {
            0 => self.monday,
            1 => self.tuesday,
            2 => self.wednesday,
            3 => self.thursday,
            4 => self.friday,
            5 => self.saturday,
            6 => self.sunday,
            _ => false,
        }
    }
}

/// GTFS Calendar Date (Service exception)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GtfsCalendarDate {
    pub id: i32,
    pub service_id: String,
    pub date: String,
    pub exception_type: i32, // 1: added, 2: removed
}

impl GtfsCalendarDate {
    pub fn new(id: i32, service_id: String, date: String, exception_type: i32) -> Self {
        Self {
            id,
            service_id,
            date,
            exception_type,
        }
    }

    /// Check if this exception adds the service on this date
    pub fn is_added(&self) -> bool {
        self.exception_type == 1
    }

    /// Check if this exception removes the service on this date
    pub fn is_removed(&self) -> bool {
        self.exception_type == 2
    }
}

/// GTFS Trip (Single bus trip/journey)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GtfsTrip {
    pub trip_id: String,
    pub route_id: String,
    pub service_id: String,
    pub trip_headsign: Option<String>,
    pub trip_headsign_k: Option<String>,
    pub trip_headsign_r: Option<String>,
    pub trip_short_name: Option<String>,
    pub direction_id: Option<i32>,
    pub block_id: Option<String>,
    pub shape_id: Option<String>,
    pub wheelchair_accessible: Option<i32>,
    pub bikes_allowed: Option<i32>,
}

impl GtfsTrip {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        trip_id: String,
        route_id: String,
        service_id: String,
        trip_headsign: Option<String>,
        trip_headsign_k: Option<String>,
        trip_headsign_r: Option<String>,
        trip_short_name: Option<String>,
        direction_id: Option<i32>,
        block_id: Option<String>,
        shape_id: Option<String>,
        wheelchair_accessible: Option<i32>,
        bikes_allowed: Option<i32>,
    ) -> Self {
        Self {
            trip_id,
            route_id,
            service_id,
            trip_headsign,
            trip_headsign_k,
            trip_headsign_r,
            trip_short_name,
            direction_id,
            block_id,
            shape_id,
            wheelchair_accessible,
            bikes_allowed,
        }
    }
}

/// GTFS Stop Time (Timetable entry)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GtfsStopTime {
    pub id: i32,
    pub trip_id: String,
    pub arrival_time: Option<String>,
    pub departure_time: Option<String>,
    pub stop_id: String,
    pub stop_sequence: i32,
    pub stop_headsign: Option<String>,
    pub pickup_type: Option<i32>,
    pub drop_off_type: Option<i32>,
    pub shape_dist_traveled: Option<f64>,
    pub timepoint: Option<i32>,
}

impl GtfsStopTime {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        trip_id: String,
        arrival_time: Option<String>,
        departure_time: Option<String>,
        stop_id: String,
        stop_sequence: i32,
        stop_headsign: Option<String>,
        pickup_type: Option<i32>,
        drop_off_type: Option<i32>,
        shape_dist_traveled: Option<f64>,
        timepoint: Option<i32>,
    ) -> Self {
        Self {
            id,
            trip_id,
            arrival_time,
            departure_time,
            stop_id,
            stop_sequence,
            stop_headsign,
            pickup_type,
            drop_off_type,
            shape_dist_traveled,
            timepoint,
        }
    }
}

/// GTFS Shape Point (Route geometry point)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GtfsShapePoint {
    pub id: i32,
    pub shape_id: String,
    pub shape_pt_lat: f64,
    pub shape_pt_lon: f64,
    pub shape_pt_sequence: i32,
    pub shape_dist_traveled: Option<f64>,
}

impl GtfsShapePoint {
    pub fn new(
        id: i32,
        shape_id: String,
        shape_pt_lat: f64,
        shape_pt_lon: f64,
        shape_pt_sequence: i32,
        shape_dist_traveled: Option<f64>,
    ) -> Self {
        Self {
            id,
            shape_id,
            shape_pt_lat,
            shape_pt_lon,
            shape_pt_sequence,
            shape_dist_traveled,
        }
    }
}

/// GTFS Feed Info (Feed metadata)
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct GtfsFeedInfo {
    pub id: i32,
    pub feed_publisher_name: String,
    pub feed_publisher_url: Option<String>,
    pub feed_lang: Option<String>,
    pub feed_start_date: Option<String>,
    pub feed_end_date: Option<String>,
    pub feed_version: Option<String>,
    pub feed_contact_email: Option<String>,
    pub feed_contact_url: Option<String>,
    pub imported_at: Option<String>,
}

impl GtfsFeedInfo {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i32,
        feed_publisher_name: String,
        feed_publisher_url: Option<String>,
        feed_lang: Option<String>,
        feed_start_date: Option<String>,
        feed_end_date: Option<String>,
        feed_version: Option<String>,
        feed_contact_email: Option<String>,
        feed_contact_url: Option<String>,
        imported_at: Option<String>,
    ) -> Self {
        Self {
            id,
            feed_publisher_name,
            feed_publisher_url,
            feed_lang,
            feed_start_date,
            feed_end_date,
            feed_version,
            feed_contact_email,
            feed_contact_url,
            imported_at,
        }
    }
}

/// Transport type enum for distinguishing rail and bus
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
#[repr(i32)]
pub enum TransportType {
    #[default]
    Rail = 0,
    Bus = 1,
}

impl From<i32> for TransportType {
    fn from(value: i32) -> Self {
        match value {
            1 => TransportType::Bus,
            _ => TransportType::Rail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gtfs_agency_new() {
        let agency = GtfsAgency::new(
            "toei".to_string(),
            "東京都交通局".to_string(),
            Some("トウキョウトコウツウキョク".to_string()),
            Some("Tokyo Metropolitan Bureau of Transportation".to_string()),
            Some("东京都交通局".to_string()),
            Some("도쿄도 교통국".to_string()),
            Some("https://www.kotsu.metro.tokyo.jp/".to_string()),
            "Asia/Tokyo".to_string(),
            Some("ja".to_string()),
            Some("03-3816-5700".to_string()),
            None,
            Some(1001),
        );

        assert_eq!(agency.agency_id, "toei");
        assert_eq!(agency.agency_name, "東京都交通局");
        assert_eq!(agency.company_cd, Some(1001));
    }

    #[test]
    fn test_gtfs_route_new() {
        let route = GtfsRoute::new(
            "toei_bus_01".to_string(),
            Some("toei".to_string()),
            Some("都01".to_string()),
            Some("渋谷駅～新橋駅".to_string()),
            None,
            None,
            None,
            None,
            None,
            3,
            None,
            Some("FF0000".to_string()),
            Some("FFFFFF".to_string()),
            Some(1),
            None,
        );

        assert_eq!(route.route_id, "toei_bus_01");
        assert_eq!(route.route_short_name, Some("都01".to_string()));
        assert_eq!(route.route_type, 3);
    }

    #[test]
    fn test_gtfs_stop_new() {
        let stop = GtfsStop::new(
            "stop_001".to_string(),
            Some("001".to_string()),
            "渋谷駅前".to_string(),
            Some("シブヤエキマエ".to_string()),
            Some("Shibuya Station".to_string()),
            None,
            None,
            None,
            35.658034,
            139.701636,
            None,
            None,
            Some(0),
            None,
            None,
            Some(1),
            None,
            None,
        );

        assert_eq!(stop.stop_id, "stop_001");
        assert_eq!(stop.stop_name, "渋谷駅前");
        assert!((stop.stop_lat - 35.658034).abs() < 0.0001);
    }

    #[test]
    fn test_gtfs_calendar_runs_on_weekday() {
        let calendar = GtfsCalendar::new(
            "weekday".to_string(),
            true,
            true,
            true,
            true,
            true,
            false,
            false,
            "20240101".to_string(),
            "20241231".to_string(),
        );

        assert!(calendar.runs_on_weekday(0)); // Monday
        assert!(calendar.runs_on_weekday(4)); // Friday
        assert!(!calendar.runs_on_weekday(5)); // Saturday
        assert!(!calendar.runs_on_weekday(6)); // Sunday
    }

    #[test]
    fn test_gtfs_calendar_date_exception_type() {
        let added = GtfsCalendarDate::new(1, "service1".to_string(), "20240101".to_string(), 1);
        let removed = GtfsCalendarDate::new(2, "service1".to_string(), "20240102".to_string(), 2);

        assert!(added.is_added());
        assert!(!added.is_removed());
        assert!(!removed.is_added());
        assert!(removed.is_removed());
    }

    #[test]
    fn test_transport_type_conversion() {
        assert_eq!(TransportType::from(0), TransportType::Rail);
        assert_eq!(TransportType::from(1), TransportType::Bus);
        assert_eq!(TransportType::from(99), TransportType::Rail); // Default to Rail

        assert_eq!(TransportType::Rail as i32, 0);
        assert_eq!(TransportType::Bus as i32, 1);
    }

    #[test]
    fn test_transport_type_default() {
        let default_type: TransportType = Default::default();
        assert_eq!(default_type, TransportType::Rail);
    }
}
