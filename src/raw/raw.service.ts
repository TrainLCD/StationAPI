import { Injectable } from '@nestjs/common';
import { Line, Station } from 'src/graphql';
import { LineRaw } from 'src/line/models/LineRaw';
import { StationRaw } from 'src/station/models/StationRaw';

@Injectable()
export class RawService {
  convertStation(raw: StationRaw): Station {
    if (!raw) {
      return;
    }
    return {
      id: raw.station_cd,
      address: raw.address,
      distance: raw.distance,
      latitude: raw.lat,
      longitude: raw.lon,
      lines: raw.lines.map((l) => this.convertLine(l)),
      openYmd: raw.open_ymd,
      postalCode: raw.post,
      prefId: raw.pref_cd,
      groupId: raw.station_g_cd,
      name: raw.station_name,
      nameK: raw.station_name_k,
      nameR: raw.station_name_r,
    };
  }

  convertLine(raw: LineRaw): Line {
    if (!raw) {
      return;
    }

    return {
      id: raw.line_cd,
      companyId: raw.company_cd,
      latitude: raw.lat,
      longitude: raw.lon,
      lineColorC: raw.line_color_c,
      lineColorT: raw.line_color_t,
      name: raw.line_name,
      nameH: raw.line_name_h,
      nameK: raw.line_name_k,
      nameR: raw.line_name_r,
      lineType: raw.line_type,
      zoom: raw.zoom,
    };
  }
}
