import { LineRaw } from 'src/line/models/LineRaw';

export class TrainTypeRaw {
  type_cd: number;
  id: number;
  station_cd: number;
  type_name: string;
  type_name_k: string;
  type_name_r: string;
  type_name_zh: string;
  type_name_ko: string;
  line_group_cd: number;
  line_cd: number;
  color: string;
  lines: LineRaw[];
  direction: number;
}

export class TrainTypeWithLineRaw {
  // TrainType
  type_cd: number;
  id: number;
  station_cd: number;
  type_name: string;
  type_name_k: string;
  type_name_r: string;
  type_name_zh: string;
  type_name_ko: string;
  line_group_cd: number;
  color: string;
  // Line
  line_cd: number;
  company_cd: number;
  line_name: string;
  line_name_k: string;
  line_name_h: string;
  line_name_r: string;
  line_name_zh: string;
  line_name_ko: string;
  line_color_c: string;
  line_color_t: string;
  line_type: number;
  lon: number;
  lat: number;
  zoom: number;
  e_status: number;
  e_sort: number;
  // Company
  company_name: string;
  company_name_en: string;
}
