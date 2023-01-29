import { CompanyRaw, LineRaw } from 'src/line/models/LineRaw';
import { Line } from 'src/models/line.model';
import { LineSymbol } from 'src/models/lineSymbol.model';
import { Station } from 'src/models/station.model';
import { StationNumber } from 'src/models/stationNumber.model';
import { StationRaw } from 'src/models/stationRaw';
import { StopCondition } from 'src/models/stopCondition.model';
import { TrainDirection } from 'src/models/trainDirection.model';
import { TrainType } from 'src/models/traintype.model';
import { TrainTypeRaw } from 'src/trainType/models/TrainTypeRaw';

export const convertStation = (
  raw: StationRaw,
  companies?: CompanyRaw[],
  trainTypes?: TrainType[],
): Station => {
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
      case 5:
        return StopCondition.PARTIAL_STOP;
      default:
        return StopCondition.ALL;
    }
  })() as StopCondition;

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

  const rawCurrentLine = raw.lines?.find((l) => l.line_cd === raw.line_cd);

  const lineSymbolsRaw = [
    rawCurrentLine?.line_symbol_primary,
    rawCurrentLine?.line_symbol_secondary,
    rawCurrentLine?.line_symbol_extra,
  ];
  const lineSymbolColorsRaw = [
    rawCurrentLine?.line_symbol_primary_color.length
      ? rawCurrentLine?.line_symbol_primary_color
      : rawCurrentLine?.line_color_c,
    rawCurrentLine?.line_symbol_secondary_color.length
      ? rawCurrentLine?.line_symbol_secondary_color
      : rawCurrentLine?.line_color_c,
    rawCurrentLine?.line_symbol_extra_color.length
      ? rawCurrentLine?.line_symbol_extra_color
      : rawCurrentLine?.line_color_c,
  ];
  const stationNumbersRaw = [
    raw.primary_station_number,
    raw.secondary_station_number,
    raw.extra_station_number,
  ];

  const fullStationNumbers: StationNumber[] = stationNumbersRaw
    .map(
      (num, idx) =>
        num && {
          lineSymbol: lineSymbolsRaw[idx] ?? null,
          lineSymbolColor: lineSymbolColorsRaw[idx] ?? null,
          stationNumber: `${lineSymbolsRaw[idx] ?? ''}-${
            stationNumbersRaw[idx]
          }`,
        },
    )
    .filter((num) => num)
    .map((num) => ({
      ...num,
      // 01: 札幌駅
      stationNumber: num.stationNumber === '0-1' ? '01' : num.stationNumber,
    }));

  return {
    id: raw.station_cd,
    address: raw.address,
    distance: raw.distance,
    latitude: raw.lat,
    longitude: raw.lon,
    currentLine: convertLine(
      raw.currentLine,
      companies?.find((c) => c.company_cd === raw.currentLine.company_cd),
    ),
    lines: raw.lines?.map((l) =>
      convertLine(
        l,
        companies?.find((c) => c.company_cd === l.company_cd),
      ),
    ),
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
    trainTypes:
      trainTypes?.map((tt) => ({ ...tt, direction: enumDirection })) ?? [],
    stationNumbers: fullStationNumbers,
    threeLetterCode: raw.three_letter_code,
  };
};

export const convertLine = (
  lineRaw: LineRaw,
  companyRaw?: CompanyRaw,
): Line => {
  if (!lineRaw) {
    return;
  }

  const lineSymbols: LineSymbol[] = [
    lineRaw.line_symbol_primary && {
      lineSymbol: lineRaw.line_symbol_primary || null,
      lineSymbolColor: lineRaw.line_symbol_primary_color.length
        ? lineRaw.line_symbol_primary_color
        : lineRaw.line_color_c,
    },
    lineRaw.line_symbol_secondary && {
      lineSymbol: lineRaw.line_symbol_secondary || null,
      lineSymbolColor: lineRaw.line_symbol_secondary_color.length
        ? lineRaw.line_symbol_secondary_color
        : lineRaw.line_color_c,
    },
    lineRaw.line_symbol_extra && {
      lineSymbol: lineRaw.line_symbol_extra || null,
      lineSymbolColor: lineRaw.line_symbol_extra_color.length
        ? lineRaw.line_symbol_extra_color
        : lineRaw.line_color_c,
    },
  ].filter((sym) => sym);

  return {
    id: lineRaw.line_cd,
    companyId: lineRaw.company_cd,
    latitude: lineRaw.lat,
    longitude: lineRaw.lon,
    lineColorC: lineRaw.line_color_c,
    lineColorT: lineRaw.line_color_t,
    lineSymbols,
    name: lineRaw.line_name,
    nameH: lineRaw.line_name_h,
    nameK: lineRaw.line_name_k,
    nameR: lineRaw.line_name_r,
    nameZh: lineRaw.line_name_zh,
    nameKo: lineRaw.line_name_ko,
    lineType: lineRaw.line_type,
    zoom: lineRaw.zoom,
    company: companyRaw && {
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
    transferStation: convertStation(lineRaw.transferStation),
  };
};

export const convertTrainType = (
  raw: TrainTypeRaw,
  stations: Station[],
  lines: Line[],
): TrainType => {
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
    typeId: raw.type_cd,
    groupId: raw.line_group_cd,
    name: raw.type_name,
    nameK: raw.type_name_k,
    nameR: raw.type_name_r,
    nameZh: raw.type_name_zh,
    nameKo: raw.type_name_ko,
    color: raw.color,
    direction: enumDirection,
    allTrainTypes: [],
    stations,
    lines,
  };
};
