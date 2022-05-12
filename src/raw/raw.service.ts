import { Injectable } from '@nestjs/common';
import {
  Line,
  Station,
  StopCondition,
  TrainDirection,
  TrainType,
} from 'src/graphql';
import { CompanyRaw, LineRaw } from 'src/line/models/LineRaw';
import { StationRaw } from 'src/station/models/StationRaw';
import { TrainTypeRaw } from 'src/trainType/models/TrainTypeRaw';

@Injectable()
export class RawService {
  convertStation(
    raw: StationRaw,
    companyRaw: CompanyRaw,
    trainTypes?: TrainType[],
  ): Station {
    if (!raw) {
      return;
    }

    const enumStopCondition = (() => {
      switch (raw.pass) {
        case 0:
          return StopCondition.ALL;
        case 1:
          return StopCondition.NOT;
        case 2:
          return StopCondition.PARTIAL;
        case 3:
          return StopCondition.WEEKDAY;
        case 4:
          return StopCondition.HOLIDAY;
        default:
          return StopCondition.ALL;
      }
    })() as StopCondition;

    const rawCurrentLine = raw.lines.find((l) => l.line_cd === raw.line_cd);

    const stationNumber = (() => {
      if (raw.primary_station_number.length) {
        return raw.primary_station_number;
      }
      if (raw.secondary_station_number.length) {
        return raw.secondary_station_number;
      }
      return null;
    })();
    const fullStationNumber = (() => {
      if (raw.primary_station_number) {
        return `${rawCurrentLine.line_symbol_primary}-${raw.primary_station_number}`;
      }
      if (raw.secondary_station_number) {
        return `${rawCurrentLine.line_symbol_secondary}-${raw.secondary_station_number}`;
      }
      return null;
    })();

    return {
      id: raw.station_cd,
      address: raw.address,
      distance: raw.distance,
      latitude: raw.lat,
      longitude: raw.lon,
      currentLine: this.convertLine(raw.currentLine, companyRaw),
      lines: raw.lines?.map((l) => this.convertLine(l, companyRaw)),
      openYmd: raw.open_ymd,
      postalCode: raw.post,
      prefId: raw.pref_cd,
      groupId: raw.station_g_cd,
      name: raw.station_name,
      nameK: raw.station_name_k,
      nameR: raw.station_name_r,
      nameZh: raw.station_name_zh,
      nameKo: raw.station_name_ko,
      pass: raw.pass === 1 ? true : false,
      stopCondition: enumStopCondition,
      trainTypes: trainTypes,
      stationNumber,
      fullStationNumber,
      secondaryFullStationNumber: raw.secondary_station_number.length
        ? raw.secondary_station_number
        : null,
      secondaryStationNumber: raw.secondary_station_number
        ? `${rawCurrentLine.line_symbol_secondary}-${raw.secondary_station_number}`
        : null,
    };
  }

  convertLine(lineRaw: LineRaw, companyRaw: CompanyRaw): Line {
    if (!lineRaw || !companyRaw) {
      return;
    }

    return {
      id: lineRaw.line_cd,
      companyId: lineRaw.company_cd,
      latitude: lineRaw.lat,
      longitude: lineRaw.lon,
      lineColorC: lineRaw.line_color_c,
      lineColorT: lineRaw.line_color_t,
      lineSymbolPrimary: lineRaw.line_symbol_primary.length
        ? lineRaw.line_symbol_primary
        : null,
      lineSymbolSecondary: lineRaw.line_symbol_secondary.length
        ? lineRaw.line_symbol_secondary
        : null,
      name: lineRaw.line_name,
      nameH: lineRaw.line_name_h,
      nameK: lineRaw.line_name_k,
      nameR: lineRaw.line_name_r,
      nameZh: lineRaw.line_name_zh,
      nameKo: lineRaw.line_name_ko,
      lineType: lineRaw.line_type,
      zoom: lineRaw.zoom,
      company: {
        id: companyRaw.company_cd,
        railroadId: companyRaw.rr_cd,
        name: companyRaw.company_name,
        nameK: companyRaw.company_name_k,
        nameH: companyRaw.company_name_h,
        nameR: companyRaw.company_name_r,
        nameEn: companyRaw.company_name_en,
        url: companyRaw.company_url,
        companyType: companyRaw.company_type,
      },
    };
  }

  convertTrainType(
    raw: TrainTypeRaw,
    stations: Station[],
    lines: Line[],
  ): TrainType {
    if (!raw) {
      return;
    }

    const enumDirection = (() => {
      switch (raw.direction) {
        case 0:
          return TrainDirection.BOTH;
        case 1:
          return TrainDirection.INBOUND;
        case 2:
          return TrainDirection.OUTBOUND;
        default:
          return TrainDirection.BOTH;
      }
    })() as TrainDirection;

    return {
      id: raw.type_cd,
      groupId: raw.line_group_cd,
      name: raw.type_name,
      nameK: raw.type_name_k,
      nameR: raw.type_name_r,
      nameZh: raw.type_name_zh,
      nameKo: raw.type_name_ko,
      color: raw.color,
      direction: enumDirection,
      stations,
      lines,
    };
  }
}
