use std::vec;

use anyhow::Result;
use bigdecimal::Zero;

use crate::{
    domain::models::line::{line_model::Line, line_repository::LineRepository},
    pb::{LineResponse, LineSymbol},
};

#[derive(Debug)]
pub struct LineService<T>
where
    T: LineRepository,
{
    line_repository: T,
}

impl From<Line> for LineResponse {
    fn from(value: Line) -> Self {
        Self {
            id: value.line_cd,
            name_short: value.line_name,
            name_katakana: value.line_name_k,
            name_full: value.line_name_h,
            name_roman: value.line_name_r,
            name_chinese: value.line_name_zh.unwrap_or("".to_string()),
            name_korean: value.line_name_ko.unwrap_or("".to_string()),
            color: value.line_color_c,
            line_type: value.line_type as i32,
            company_id: value.company_cd,
            line_symbols: vec![],
            status: value.e_status as i32,
        }
    }
}

impl<T: LineRepository> LineService<T> {
    pub fn new(line_repository: T) -> Self {
        Self { line_repository }
    }
    pub async fn find_by_id(&self, id: u32) -> Result<Line> {
        match self.line_repository.find_by_id(id).await {
            Ok(value) => Ok(value.into()),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the line. Provided ID: {:?}",
                id
            )),
        }
    }
    pub async fn get_by_station_group_id(&self, station_group_id: u32) -> Result<Vec<Line>> {
        match self
            .line_repository
            .get_by_station_group_id(station_group_id)
            .await
        {
            Ok(value) => Ok(value.into()),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the line. Provided Group ID: {:?}",
                station_group_id
            )),
        }
    }

    pub async fn find_by_station_id(&self, station_id: u32) -> Result<Line> {
        match self.line_repository.find_by_station_id(station_id).await {
            Ok(value) => Ok(value.into()),
            Err(_) => Err(anyhow::anyhow!(
                "Could not find the line. Provided Station ID: {:?}",
                station_id
            )),
        }
    }

    pub fn get_line_symbols(&self, line: &mut Line) -> Vec<LineSymbol> {
        let mut line_symbols = vec![];

        if !line
            .line_symbol_primary
            .clone()
            .unwrap_or("".to_string())
            .len()
            .is_zero()
        {
            let line_symbol = LineSymbol {
                symbol: line.clone().line_symbol_primary.unwrap_or("".to_string()),
                color: line
                    .clone()
                    .line_symbol_primary_color
                    .unwrap_or("".to_string()),
                shape: line
                    .clone()
                    .line_symbol_primary_shape
                    .unwrap_or("".to_string()),
            };
            line_symbols.push(line_symbol);
        }

        if !line
            .line_symbol_secondary
            .clone()
            .unwrap_or("".to_string())
            .len()
            .is_zero()
        {
            let line_symbol = LineSymbol {
                symbol: line.clone().line_symbol_secondary.unwrap_or("".to_string()),
                color: line
                    .clone()
                    .line_symbol_secondary_color
                    .unwrap_or("".to_string()),
                shape: line
                    .clone()
                    .line_symbol_secondary_shape
                    .unwrap_or("".to_string()),
            };
            line_symbols.push(line_symbol);
        }

        if !line
            .line_symbol_extra
            .clone()
            .unwrap_or("".to_string())
            .len()
            .is_zero()
        {
            let line_symbol = LineSymbol {
                symbol: line.clone().line_symbol_extra.unwrap_or("".to_string()),
                color: line
                    .clone()
                    .line_symbol_extra_color
                    .unwrap_or("".to_string()),
                shape: line
                    .clone()
                    .line_symbol_extra_shape
                    .unwrap_or("".to_string()),
            };
            line_symbols.push(line_symbol);
        }
        line_symbols
    }
}
