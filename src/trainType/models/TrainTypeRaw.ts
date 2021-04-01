import { LineRaw } from 'src/line/models/LineRaw';

export class TrainTypeRaw {
  type_cd: number;
  id: number;
  station_cd: number;
  type_name: string;
  type_name_k: string;
  type_name_r: string;
  line_group_cd: number;
  color: string;
  lines: LineRaw[];
}
