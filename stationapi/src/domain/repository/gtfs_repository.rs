use async_trait::async_trait;

use crate::domain::{
    entity::gtfs::{
        GtfsAgency, GtfsCalendar, GtfsCalendarDate, GtfsFeedInfo, GtfsRoute, GtfsShapePoint,
        GtfsStop, GtfsStopTime, GtfsTrip,
    },
    error::DomainError,
};

/// Repository trait for GTFS Agency operations
#[async_trait]
pub trait GtfsAgencyRepository: Send + Sync {
    async fn find_by_id(&self, agency_id: &str) -> Result<Option<GtfsAgency>, DomainError>;
    async fn get_all(&self) -> Result<Vec<GtfsAgency>, DomainError>;
    async fn get_by_company_cd(&self, company_cd: i32) -> Result<Vec<GtfsAgency>, DomainError>;
}

/// Repository trait for GTFS Route operations
#[async_trait]
pub trait GtfsRouteRepository: Send + Sync {
    async fn find_by_id(&self, route_id: &str) -> Result<Option<GtfsRoute>, DomainError>;
    async fn get_by_agency_id(&self, agency_id: &str) -> Result<Vec<GtfsRoute>, DomainError>;
    async fn get_by_line_cd(&self, line_cd: i32) -> Result<Vec<GtfsRoute>, DomainError>;
    async fn search_by_name(
        &self,
        name: &str,
        limit: Option<u32>,
    ) -> Result<Vec<GtfsRoute>, DomainError>;
}

/// Repository trait for GTFS Stop operations
#[async_trait]
pub trait GtfsStopRepository: Send + Sync {
    async fn find_by_id(&self, stop_id: &str) -> Result<Option<GtfsStop>, DomainError>;
    async fn get_by_station_cd(&self, station_cd: i32) -> Result<Vec<GtfsStop>, DomainError>;
    async fn get_by_coordinates(
        &self,
        latitude: f64,
        longitude: f64,
        limit: Option<u32>,
    ) -> Result<Vec<GtfsStop>, DomainError>;
    async fn search_by_name(
        &self,
        name: &str,
        limit: Option<u32>,
    ) -> Result<Vec<GtfsStop>, DomainError>;
    async fn get_by_route_id(&self, route_id: &str) -> Result<Vec<GtfsStop>, DomainError>;
}

/// Repository trait for GTFS Calendar operations
#[async_trait]
pub trait GtfsCalendarRepository: Send + Sync {
    async fn find_by_id(&self, service_id: &str) -> Result<Option<GtfsCalendar>, DomainError>;
    async fn get_active_on_date(&self, date: &str) -> Result<Vec<GtfsCalendar>, DomainError>;
}

/// Repository trait for GTFS Calendar Date operations
#[async_trait]
pub trait GtfsCalendarDateRepository: Send + Sync {
    async fn get_by_service_id(
        &self,
        service_id: &str,
    ) -> Result<Vec<GtfsCalendarDate>, DomainError>;
    async fn get_by_date(&self, date: &str) -> Result<Vec<GtfsCalendarDate>, DomainError>;
}

/// Repository trait for GTFS Trip operations
#[async_trait]
pub trait GtfsTripRepository: Send + Sync {
    async fn find_by_id(&self, trip_id: &str) -> Result<Option<GtfsTrip>, DomainError>;
    async fn get_by_route_id(&self, route_id: &str) -> Result<Vec<GtfsTrip>, DomainError>;
    async fn get_by_service_id(&self, service_id: &str) -> Result<Vec<GtfsTrip>, DomainError>;
}

/// Repository trait for GTFS Stop Time operations
#[async_trait]
pub trait GtfsStopTimeRepository: Send + Sync {
    async fn get_by_trip_id(&self, trip_id: &str) -> Result<Vec<GtfsStopTime>, DomainError>;
    async fn get_by_stop_id(&self, stop_id: &str) -> Result<Vec<GtfsStopTime>, DomainError>;
    async fn get_departures_at_stop(
        &self,
        stop_id: &str,
        from_time: &str,
        limit: Option<u32>,
    ) -> Result<Vec<GtfsStopTime>, DomainError>;
}

/// Repository trait for GTFS Shape operations
#[async_trait]
pub trait GtfsShapeRepository: Send + Sync {
    async fn get_by_shape_id(&self, shape_id: &str) -> Result<Vec<GtfsShapePoint>, DomainError>;
}

/// Repository trait for GTFS Feed Info operations
#[async_trait]
pub trait GtfsFeedInfoRepository: Send + Sync {
    async fn get_latest(&self) -> Result<Option<GtfsFeedInfo>, DomainError>;
    async fn get_all(&self) -> Result<Vec<GtfsFeedInfo>, DomainError>;
}
