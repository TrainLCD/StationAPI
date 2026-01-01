use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

use crate::domain::{
    entity::gtfs::{
        GtfsAgency, GtfsCalendar, GtfsCalendarDate, GtfsFeedInfo, GtfsRoute, GtfsShapePoint,
        GtfsStop, GtfsStopTime, GtfsTrip,
    },
    error::DomainError,
    repository::gtfs_repository::{
        GtfsAgencyRepository, GtfsCalendarDateRepository, GtfsCalendarRepository,
        GtfsFeedInfoRepository, GtfsRouteRepository, GtfsShapeRepository, GtfsStopRepository,
        GtfsStopTimeRepository, GtfsTripRepository,
    },
};

// ============================================================
// Row types for SQLx
// ============================================================

#[derive(sqlx::FromRow)]
struct GtfsAgencyRow {
    agency_id: String,
    agency_name: String,
    agency_name_k: Option<String>,
    agency_name_r: Option<String>,
    agency_name_zh: Option<String>,
    agency_name_ko: Option<String>,
    agency_url: Option<String>,
    agency_timezone: Option<String>,
    agency_lang: Option<String>,
    agency_phone: Option<String>,
    agency_fare_url: Option<String>,
    company_cd: Option<i32>,
}

impl From<GtfsAgencyRow> for GtfsAgency {
    fn from(row: GtfsAgencyRow) -> Self {
        Self {
            agency_id: row.agency_id,
            agency_name: row.agency_name,
            agency_name_k: row.agency_name_k,
            agency_name_r: row.agency_name_r,
            agency_name_zh: row.agency_name_zh,
            agency_name_ko: row.agency_name_ko,
            agency_url: row.agency_url,
            agency_timezone: row.agency_timezone.unwrap_or_else(|| "Asia/Tokyo".to_string()),
            agency_lang: row.agency_lang,
            agency_phone: row.agency_phone,
            agency_fare_url: row.agency_fare_url,
            company_cd: row.company_cd,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GtfsRouteRow {
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
}

impl From<GtfsRouteRow> for GtfsRoute {
    fn from(row: GtfsRouteRow) -> Self {
        Self {
            route_id: row.route_id,
            agency_id: row.agency_id,
            route_short_name: row.route_short_name,
            route_long_name: row.route_long_name,
            route_long_name_k: row.route_long_name_k,
            route_long_name_r: row.route_long_name_r,
            route_long_name_zh: row.route_long_name_zh,
            route_long_name_ko: row.route_long_name_ko,
            route_desc: row.route_desc,
            route_type: row.route_type,
            route_url: row.route_url,
            route_color: row.route_color,
            route_text_color: row.route_text_color,
            route_sort_order: row.route_sort_order,
            line_cd: row.line_cd,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GtfsStopRow {
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
}

impl From<GtfsStopRow> for GtfsStop {
    fn from(row: GtfsStopRow) -> Self {
        Self {
            stop_id: row.stop_id,
            stop_code: row.stop_code,
            stop_name: row.stop_name,
            stop_name_k: row.stop_name_k,
            stop_name_r: row.stop_name_r,
            stop_name_zh: row.stop_name_zh,
            stop_name_ko: row.stop_name_ko,
            stop_desc: row.stop_desc,
            stop_lat: row.stop_lat,
            stop_lon: row.stop_lon,
            zone_id: row.zone_id,
            stop_url: row.stop_url,
            location_type: row.location_type,
            parent_station: row.parent_station,
            stop_timezone: row.stop_timezone,
            wheelchair_boarding: row.wheelchair_boarding,
            platform_code: row.platform_code,
            station_cd: row.station_cd,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GtfsCalendarRow {
    service_id: String,
    monday: bool,
    tuesday: bool,
    wednesday: bool,
    thursday: bool,
    friday: bool,
    saturday: bool,
    sunday: bool,
    start_date: chrono::NaiveDate,
    end_date: chrono::NaiveDate,
}

impl From<GtfsCalendarRow> for GtfsCalendar {
    fn from(row: GtfsCalendarRow) -> Self {
        Self {
            service_id: row.service_id,
            monday: row.monday,
            tuesday: row.tuesday,
            wednesday: row.wednesday,
            thursday: row.thursday,
            friday: row.friday,
            saturday: row.saturday,
            sunday: row.sunday,
            start_date: row.start_date.format("%Y%m%d").to_string(),
            end_date: row.end_date.format("%Y%m%d").to_string(),
        }
    }
}

#[derive(sqlx::FromRow)]
struct GtfsCalendarDateRow {
    id: i32,
    service_id: String,
    date: chrono::NaiveDate,
    exception_type: i32,
}

impl From<GtfsCalendarDateRow> for GtfsCalendarDate {
    fn from(row: GtfsCalendarDateRow) -> Self {
        Self {
            id: row.id,
            service_id: row.service_id,
            date: row.date.format("%Y%m%d").to_string(),
            exception_type: row.exception_type,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GtfsTripRow {
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
}

impl From<GtfsTripRow> for GtfsTrip {
    fn from(row: GtfsTripRow) -> Self {
        Self {
            trip_id: row.trip_id,
            route_id: row.route_id,
            service_id: row.service_id,
            trip_headsign: row.trip_headsign,
            trip_headsign_k: row.trip_headsign_k,
            trip_headsign_r: row.trip_headsign_r,
            trip_short_name: row.trip_short_name,
            direction_id: row.direction_id,
            block_id: row.block_id,
            shape_id: row.shape_id,
            wheelchair_accessible: row.wheelchair_accessible,
            bikes_allowed: row.bikes_allowed,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GtfsStopTimeRow {
    id: i32,
    trip_id: String,
    arrival_time: Option<chrono::NaiveTime>,
    departure_time: Option<chrono::NaiveTime>,
    stop_id: String,
    stop_sequence: i32,
    stop_headsign: Option<String>,
    pickup_type: Option<i32>,
    drop_off_type: Option<i32>,
    shape_dist_traveled: Option<f64>,
    timepoint: Option<i32>,
}

impl From<GtfsStopTimeRow> for GtfsStopTime {
    fn from(row: GtfsStopTimeRow) -> Self {
        Self {
            id: row.id,
            trip_id: row.trip_id,
            arrival_time: row.arrival_time.map(|t| t.format("%H:%M:%S").to_string()),
            departure_time: row.departure_time.map(|t| t.format("%H:%M:%S").to_string()),
            stop_id: row.stop_id,
            stop_sequence: row.stop_sequence,
            stop_headsign: row.stop_headsign,
            pickup_type: row.pickup_type,
            drop_off_type: row.drop_off_type,
            shape_dist_traveled: row.shape_dist_traveled,
            timepoint: row.timepoint,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GtfsShapePointRow {
    id: i32,
    shape_id: String,
    shape_pt_lat: f64,
    shape_pt_lon: f64,
    shape_pt_sequence: i32,
    shape_dist_traveled: Option<f64>,
}

impl From<GtfsShapePointRow> for GtfsShapePoint {
    fn from(row: GtfsShapePointRow) -> Self {
        Self {
            id: row.id,
            shape_id: row.shape_id,
            shape_pt_lat: row.shape_pt_lat,
            shape_pt_lon: row.shape_pt_lon,
            shape_pt_sequence: row.shape_pt_sequence,
            shape_dist_traveled: row.shape_dist_traveled,
        }
    }
}

#[derive(sqlx::FromRow)]
struct GtfsFeedInfoRow {
    id: i32,
    feed_publisher_name: String,
    feed_publisher_url: Option<String>,
    feed_lang: Option<String>,
    feed_start_date: Option<chrono::NaiveDate>,
    feed_end_date: Option<chrono::NaiveDate>,
    feed_version: Option<String>,
    feed_contact_email: Option<String>,
    feed_contact_url: Option<String>,
    imported_at: Option<chrono::NaiveDateTime>,
}

impl From<GtfsFeedInfoRow> for GtfsFeedInfo {
    fn from(row: GtfsFeedInfoRow) -> Self {
        Self {
            id: row.id,
            feed_publisher_name: row.feed_publisher_name,
            feed_publisher_url: row.feed_publisher_url,
            feed_lang: row.feed_lang,
            feed_start_date: row.feed_start_date.map(|d| d.format("%Y%m%d").to_string()),
            feed_end_date: row.feed_end_date.map(|d| d.format("%Y%m%d").to_string()),
            feed_version: row.feed_version,
            feed_contact_email: row.feed_contact_email,
            feed_contact_url: row.feed_contact_url,
            imported_at: row
                .imported_at
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
        }
    }
}

// ============================================================
// Repository implementations (using runtime query_as)
// ============================================================

pub struct MyGtfsAgencyRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyGtfsAgencyRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GtfsAgencyRepository for MyGtfsAgencyRepository {
    async fn find_by_id(&self, agency_id: &str) -> Result<Option<GtfsAgency>, DomainError> {
        let row = sqlx::query_as::<_, GtfsAgencyRow>(
            r#"SELECT * FROM gtfs_agencies WHERE agency_id = $1"#,
        )
        .bind(agency_id)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_all(&self) -> Result<Vec<GtfsAgency>, DomainError> {
        let rows = sqlx::query_as::<_, GtfsAgencyRow>(r#"SELECT * FROM gtfs_agencies"#)
            .fetch_all(&*self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_by_company_cd(&self, company_cd: i32) -> Result<Vec<GtfsAgency>, DomainError> {
        let rows = sqlx::query_as::<_, GtfsAgencyRow>(
            r#"SELECT * FROM gtfs_agencies WHERE company_cd = $1"#,
        )
        .bind(company_cd)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

pub struct MyGtfsRouteRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyGtfsRouteRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GtfsRouteRepository for MyGtfsRouteRepository {
    async fn find_by_id(&self, route_id: &str) -> Result<Option<GtfsRoute>, DomainError> {
        let row =
            sqlx::query_as::<_, GtfsRouteRow>(r#"SELECT * FROM gtfs_routes WHERE route_id = $1"#)
                .bind(route_id)
                .fetch_optional(&*self.pool)
                .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_by_agency_id(&self, agency_id: &str) -> Result<Vec<GtfsRoute>, DomainError> {
        let rows = sqlx::query_as::<_, GtfsRouteRow>(
            r#"SELECT * FROM gtfs_routes WHERE agency_id = $1 ORDER BY route_sort_order"#,
        )
        .bind(agency_id)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_by_line_cd(&self, line_cd: i32) -> Result<Vec<GtfsRoute>, DomainError> {
        let rows =
            sqlx::query_as::<_, GtfsRouteRow>(r#"SELECT * FROM gtfs_routes WHERE line_cd = $1"#)
                .bind(line_cd)
                .fetch_all(&*self.pool)
                .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn search_by_name(
        &self,
        name: &str,
        limit: Option<u32>,
    ) -> Result<Vec<GtfsRoute>, DomainError> {
        let search_pattern = format!("%{name}%");
        let limit = limit.unwrap_or(10) as i64;

        let rows = sqlx::query_as::<_, GtfsRouteRow>(
            r#"SELECT * FROM gtfs_routes
               WHERE route_short_name LIKE $1
                  OR route_long_name LIKE $1
                  OR route_long_name_k LIKE $1
               ORDER BY route_sort_order
               LIMIT $2"#,
        )
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

pub struct MyGtfsStopRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyGtfsStopRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GtfsStopRepository for MyGtfsStopRepository {
    async fn find_by_id(&self, stop_id: &str) -> Result<Option<GtfsStop>, DomainError> {
        let row =
            sqlx::query_as::<_, GtfsStopRow>(r#"SELECT * FROM gtfs_stops WHERE stop_id = $1"#)
                .bind(stop_id)
                .fetch_optional(&*self.pool)
                .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_by_station_cd(&self, station_cd: i32) -> Result<Vec<GtfsStop>, DomainError> {
        let rows =
            sqlx::query_as::<_, GtfsStopRow>(r#"SELECT * FROM gtfs_stops WHERE station_cd = $1"#)
                .bind(station_cd)
                .fetch_all(&*self.pool)
                .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<GtfsStop>, DomainError> {
        let limit = limit.unwrap_or(10) as i32;

        let rows = sqlx::query_as::<_, GtfsStopRow>(
            r#"SELECT * FROM gtfs_stops
               ORDER BY point(stop_lat, stop_lon) <-> point($1, $2)
               LIMIT $3"#,
        )
        .bind(latitude)
        .bind(longitude)
        .bind(limit)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn search_by_name(
        &self,
        name: &str,
        limit: Option<u32>,
    ) -> Result<Vec<GtfsStop>, DomainError> {
        let search_pattern = format!("%{name}%");
        let limit = limit.unwrap_or(10) as i64;

        let rows = sqlx::query_as::<_, GtfsStopRow>(
            r#"SELECT * FROM gtfs_stops
               WHERE stop_name LIKE $1
                  OR stop_name_k LIKE $1
                  OR stop_name_r LIKE $1
               LIMIT $2"#,
        )
        .bind(&search_pattern)
        .bind(limit)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_by_route_id(&self, route_id: &str) -> Result<Vec<GtfsStop>, DomainError> {
        let rows = sqlx::query_as::<_, GtfsStopRow>(
            r#"SELECT DISTINCT gs.*
               FROM gtfs_stops gs
               JOIN gtfs_stop_times gst ON gs.stop_id = gst.stop_id
               JOIN gtfs_trips gt ON gst.trip_id = gt.trip_id
               WHERE gt.route_id = $1
               ORDER BY gs.stop_name"#,
        )
        .bind(route_id)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

pub struct MyGtfsCalendarRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyGtfsCalendarRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GtfsCalendarRepository for MyGtfsCalendarRepository {
    async fn find_by_id(&self, service_id: &str) -> Result<Option<GtfsCalendar>, DomainError> {
        let row = sqlx::query_as::<_, GtfsCalendarRow>(
            r#"SELECT * FROM gtfs_calendar WHERE service_id = $1"#,
        )
        .bind(service_id)
        .fetch_optional(&*self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_active_on_date(&self, date: &str) -> Result<Vec<GtfsCalendar>, DomainError> {
        let date = chrono::NaiveDate::parse_from_str(date, "%Y%m%d")
            .map_err(|e| DomainError::Unexpected(e.to_string()))?;

        let rows = sqlx::query_as::<_, GtfsCalendarRow>(
            r#"SELECT * FROM gtfs_calendar
               WHERE start_date <= $1 AND end_date >= $1"#,
        )
        .bind(date)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

pub struct MyGtfsCalendarDateRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyGtfsCalendarDateRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GtfsCalendarDateRepository for MyGtfsCalendarDateRepository {
    async fn get_by_service_id(
        &self,
        service_id: &str,
    ) -> Result<Vec<GtfsCalendarDate>, DomainError> {
        let rows = sqlx::query_as::<_, GtfsCalendarDateRow>(
            r#"SELECT * FROM gtfs_calendar_dates WHERE service_id = $1 ORDER BY date"#,
        )
        .bind(service_id)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_by_date(&self, date: &str) -> Result<Vec<GtfsCalendarDate>, DomainError> {
        let date = chrono::NaiveDate::parse_from_str(date, "%Y%m%d")
            .map_err(|e| DomainError::Unexpected(e.to_string()))?;

        let rows = sqlx::query_as::<_, GtfsCalendarDateRow>(
            r#"SELECT * FROM gtfs_calendar_dates WHERE date = $1"#,
        )
        .bind(date)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

pub struct MyGtfsTripRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyGtfsTripRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GtfsTripRepository for MyGtfsTripRepository {
    async fn find_by_id(&self, trip_id: &str) -> Result<Option<GtfsTrip>, DomainError> {
        let row =
            sqlx::query_as::<_, GtfsTripRow>(r#"SELECT * FROM gtfs_trips WHERE trip_id = $1"#)
                .bind(trip_id)
                .fetch_optional(&*self.pool)
                .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_by_route_id(&self, route_id: &str) -> Result<Vec<GtfsTrip>, DomainError> {
        let rows =
            sqlx::query_as::<_, GtfsTripRow>(r#"SELECT * FROM gtfs_trips WHERE route_id = $1"#)
                .bind(route_id)
                .fetch_all(&*self.pool)
                .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_by_service_id(&self, service_id: &str) -> Result<Vec<GtfsTrip>, DomainError> {
        let rows =
            sqlx::query_as::<_, GtfsTripRow>(r#"SELECT * FROM gtfs_trips WHERE service_id = $1"#)
                .bind(service_id)
                .fetch_all(&*self.pool)
                .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

pub struct MyGtfsStopTimeRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyGtfsStopTimeRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GtfsStopTimeRepository for MyGtfsStopTimeRepository {
    async fn get_by_trip_id(&self, trip_id: &str) -> Result<Vec<GtfsStopTime>, DomainError> {
        let rows = sqlx::query_as::<_, GtfsStopTimeRow>(
            r#"SELECT * FROM gtfs_stop_times WHERE trip_id = $1 ORDER BY stop_sequence"#,
        )
        .bind(trip_id)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_by_stop_id(&self, stop_id: &str) -> Result<Vec<GtfsStopTime>, DomainError> {
        let rows = sqlx::query_as::<_, GtfsStopTimeRow>(
            r#"SELECT * FROM gtfs_stop_times WHERE stop_id = $1 ORDER BY arrival_time"#,
        )
        .bind(stop_id)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }

    async fn get_departures_at_stop(
        &self,
        stop_id: &str,
        from_time: &str,
        limit: Option<u32>,
    ) -> Result<Vec<GtfsStopTime>, DomainError> {
        let from_time = chrono::NaiveTime::parse_from_str(from_time, "%H:%M:%S")
            .map_err(|e| DomainError::Unexpected(e.to_string()))?;
        let limit = limit.unwrap_or(10) as i64;

        let rows = sqlx::query_as::<_, GtfsStopTimeRow>(
            r#"SELECT * FROM gtfs_stop_times
               WHERE stop_id = $1 AND departure_time >= $2
               ORDER BY departure_time
               LIMIT $3"#,
        )
        .bind(stop_id)
        .bind(from_time)
        .bind(limit)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

pub struct MyGtfsShapeRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyGtfsShapeRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GtfsShapeRepository for MyGtfsShapeRepository {
    async fn get_by_shape_id(&self, shape_id: &str) -> Result<Vec<GtfsShapePoint>, DomainError> {
        let rows = sqlx::query_as::<_, GtfsShapePointRow>(
            r#"SELECT * FROM gtfs_shapes WHERE shape_id = $1 ORDER BY shape_pt_sequence"#,
        )
        .bind(shape_id)
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}

pub struct MyGtfsFeedInfoRepository {
    pool: Arc<Pool<Postgres>>,
}

impl MyGtfsFeedInfoRepository {
    pub fn new(pool: Arc<Pool<Postgres>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl GtfsFeedInfoRepository for MyGtfsFeedInfoRepository {
    async fn get_latest(&self) -> Result<Option<GtfsFeedInfo>, DomainError> {
        let row = sqlx::query_as::<_, GtfsFeedInfoRow>(
            r#"SELECT * FROM gtfs_feed_info ORDER BY imported_at DESC LIMIT 1"#,
        )
        .fetch_optional(&*self.pool)
        .await?;

        Ok(row.map(|r| r.into()))
    }

    async fn get_all(&self) -> Result<Vec<GtfsFeedInfo>, DomainError> {
        let rows = sqlx::query_as::<_, GtfsFeedInfoRow>(
            r#"SELECT * FROM gtfs_feed_info ORDER BY imported_at DESC"#,
        )
        .fetch_all(&*self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| r.into()).collect())
    }
}
