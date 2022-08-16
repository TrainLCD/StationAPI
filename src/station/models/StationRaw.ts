import { LineRaw } from 'src/line/models/LineRaw';

enum StopCondition {
  ALL,
  NOT,
  PARTIAL,
  WEEKDAY,
  HOLIDAY,
  PARTIAL_STOP,
}

enum TrainDirection {
  BOTH,
  INBOUND,
  OUTBOUND,
}

export class StationRaw {
  station_cd: number;
  station_g_cd: number;
  station_name: string;
  station_name_k: string;
  station_name_r: string;
  station_name_zh: string;
  station_name_ko: string;
  primary_station_number: string;
  secondary_station_number: string;
  extra_station_number: string;
  three_letter_code: string;
  line_cd: number;
  pref_cd: number;
  post: string;
  address: string;
  lon: number;
  lat: number;
  open_ymd: string;
  close_ymd: string;
  e_status: number;
  e_sort: number;
  distance?: number;
  lines: LineRaw[];
  currentLine: LineRaw;
  pass?: StopCondition;
  direction: TrainDirection;
}
