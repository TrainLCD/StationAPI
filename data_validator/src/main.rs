use core::panic;
use std::path::Path;

use csv::{ReaderBuilder, StringRecord};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let data_path: &Path = Path::new("data");
    let mut rdr = ReaderBuilder::new().from_path(data_path.join("3!stations.csv"))?;
    let records: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();
    let station_ids: Vec<u32> = records
        .iter()
        .map(|row| row.get(0).unwrap().parse::<u32>().unwrap())
        .collect();

    let mut rdr = ReaderBuilder::new().from_path(data_path.join("4!types.csv"))?;
    let records: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();
    let type_ids: Vec<u32> = records
        .iter()
        .map(|row| row.get(1).unwrap().parse::<u32>().unwrap())
        .collect();

    let mut rdr = ReaderBuilder::new().from_path(data_path.join("5!station_station_types.csv"))?;
    let records: Vec<StringRecord> = rdr.records().filter_map(|row| row.ok()).collect();

    if let Some(invalid_record) = records
        .iter()
        .find(|row| !station_ids.contains(&row.get(1).unwrap().parse::<u32>().unwrap()))
    {
        panic!(
            "[INVALID] Unrecognized Station ID {:?} Found!",
            invalid_record.get(1).unwrap()
        );
    }

    if let Some(invalid_record) = records
        .iter()
        .find(|row| !type_ids.contains(&row.get(2).unwrap().parse::<u32>().unwrap()))
    {
        panic!(
            "[INVALID] Unrecognized Type ID {:?} Found!",
            invalid_record.get(2).unwrap()
        );
    }

    println!("[VALID] No errors reported.");
    Ok(())
}
