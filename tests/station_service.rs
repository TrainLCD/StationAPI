use std::vec;

use anyhow::Result;
use futures::future;
use stationapi::application::station_service::StationService;
use stationapi::domain::models::station::{
    station_model::Station, station_repository::MockStationRepository,
};

#[tokio::test]
async fn test_find_by_id() -> Result<()> {
    let mut station_repo = MockStationRepository::new();
    let actual_station = get_station_fixture();
    station_repo
        .expect_find_by_id()
        .return_once(move |_| Box::pin(future::ready(Ok(actual_station))));
    let service: StationService<MockStationRepository> = StationService::new(station_repo);
    let res = service.find_by_id(1111553).await?;

    let expected_station = get_station_fixture();
    assert_eq!(res.station_cd, expected_station.station_cd);

    Ok(())
}

#[tokio::test]
async fn test_find_by_group_id() -> Result<()> {
    let mut station_repo = MockStationRepository::new();
    let actual_station = get_station_fixture();
    station_repo
        .expect_find_by_group_id()
        .return_once(move |_| Box::pin(future::ready(Ok(vec![actual_station]))));
    let service: StationService<MockStationRepository> = StationService::new(station_repo);
    let actual = service.get_by_group_id(1111553).await?;

    let expected_station = get_station_fixture();
    actual.iter().for_each(|station| {
        assert_eq!(station.station_g_cd, expected_station.station_g_cd);
    });

    Ok(())
}

#[tokio::test]
async fn test_find_by_line_id() -> Result<()> {
    let mut station_repo = MockStationRepository::new();
    let actual_station = get_station_fixture();
    station_repo
        .expect_find_by_line_id()
        .return_once(move |_| Box::pin(future::ready(Ok(vec![actual_station]))));
    let service: StationService<MockStationRepository> = StationService::new(station_repo);
    let actual = service.get_stations_by_line_id(11115).await?;

    let expected_station = get_station_fixture();
    actual.iter().for_each(|station| {
        assert_eq!(station.station_cd, expected_station.station_cd);
    });

    Ok(())
}

#[tokio::test]
async fn test_find_by_name() -> Result<()> {
    let mut station_repo = MockStationRepository::new();
    let actual_station = get_station_fixture();
    station_repo
        .expect_find_by_name()
        .return_once(move |_| Box::pin(future::ready(Ok(vec![actual_station]))));
    let service: StationService<MockStationRepository> = StationService::new(station_repo);
    let actual = service.get_stations_by_name("稚内", Some(1)).await?;

    let expected_station = get_station_fixture();
    actual.iter().for_each(|station| {
        assert_eq!(station.station_cd, expected_station.station_cd);
    });

    Ok(())
}

#[tokio::test]
async fn test_find_by_coordinates() -> Result<()> {
    let mut station_repo = MockStationRepository::new();
    let actual_station = get_station_fixture();
    station_repo
        .expect_find_by_coordinates()
        .return_once(move |_, _, _| Box::pin(future::ready(Ok(vec![actual_station]))));
    let service: StationService<MockStationRepository> = StationService::new(station_repo);
    let actual = service
        .get_station_by_coordinates(0.0, 0.0, Some(1))
        .await?;

    let expected_station = get_station_fixture();
    actual.iter().for_each(|station| {
        assert_eq!(station.station_cd, expected_station.station_cd);
    });

    Ok(())
}

pub fn get_station_fixture() -> Station {
    Station {
        station_cd: 1111553,
        station_g_cd: 1111553,
        station_name: "稚内".to_string(),
        station_name_k: "ワッカナイ".to_string(),
        station_name_r: "Wakkanai".to_string(),
        station_name_zh: "稚内".to_string(),
        station_name_ko: "왓카나이".to_string(),
        primary_station_number: Some(80.to_string()),
        secondary_station_number: Some(String::new()),
        extra_station_number: Some(String::new()),
        three_letter_code: Some(String::new()),
        line_cd: 11115,
        pref_cd: 1,
        post: "097-0022".to_string(),
        address: "北海道稚内市中央３丁目".to_string(),
        lon: 141.6769990,
        lat: 45.41699500,
        open_ymd: "0000-00-00".to_string(),
        close_ymd: "0000-00-00".to_string(),
        e_status: 0,
        e_sort: 1111553,
        distance: Some(0.0),
    }
}
