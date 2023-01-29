import { StationRaw } from 'src/models/stationRaw';

export class CompanyRaw {
  line_cd: number;
  company_cd: number;
  rr_cd: number;
  company_name: string;
  company_name_k: string;
  company_name_h: string;
  company_name_r: string;
  company_name_en: string;
  company_url: string;
  company_type: number;
  e_status: number;
  e_sort: number;
}

export class LineRaw {
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
  line_symbol_primary: string;
  line_symbol_secondary: string;
  line_symbol_extra: string;
  line_symbol_primary_color: string;
  line_symbol_secondary_color: string;
  line_symbol_extra_color: string;
  line_type: number;
  lon: number;
  lat: number;
  zoom: number;
  e_status: number;
  e_sort: number;
  transferStation?: StationRaw;
}
