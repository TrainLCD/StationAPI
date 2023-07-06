use crate::{
    domain::entity::station_number::StationNumber, pb::StationNumber as GrpcStationNumber,
};

impl From<StationNumber> for GrpcStationNumber {
    fn from(station_number: StationNumber) -> Self {
        Self {
            line_symbol: station_number.line_symbol,
            line_symbol_color: station_number.line_symbol_color,
            line_symbol_shape: station_number.line_symbol_shape,
            station_number: station_number.station_number,
        }
    }
}
