import { Injectable } from '@nestjs/common';
import { Line, Station } from 'src/graphql';
import { LineRaw } from './models/LineRaw';
import { StationRaw } from './models/StationRaw';
import { StationRepository } from './station.repository';

@Injectable()
export class StationService {
  constructor(private readonly stationRepo: StationRepository) {}

  async findOneById(id: number): Promise<Station> {
    return this.convertStation(await this.stationRepo.findOneById(id));
  }

  async findOneByGroupId(groupId: number): Promise<Station> {
    return this.convertStation(
      await this.stationRepo.findOneByGroupId(groupId),
    );
  }

  async findOneByCoords(latitude: number, longitude: number): Promise<Station> {
    return this.convertStation(
      await this.stationRepo.findOneByCoords(latitude, longitude),
    );
  }

  async getByLineId(lineId: number): Promise<Station[]> {
    return (await this.stationRepo.getByLineId(lineId)).map((s) =>
      this.convertStation(s),
    );
  }

  async getByName(name: string): Promise<Station[]> {
    return (await this.stationRepo.getByName(name)).map((s) =>
      this.convertStation(s),
    );
  }

  async findOneLineById(id: number): Promise<Line> {
    return this.convertLine(await this.stationRepo.findOneLine(id));
  }

  async getLinesByGroupId(groupId: number): Promise<Line[]> {
    return (await this.stationRepo.getLinesByGroupId(groupId)).map((l) =>
      this.convertLine(l),
    );
  }

  convertLine(raw: LineRaw): Line {
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

  convertStation(raw: StationRaw): Station {
    return {
      id: raw.station_cd,
      address: raw.address,
      distance: raw.distance,
      latitude: raw.lat,
      longitude: raw.lon,
      lines: raw.lines.map((l) => this.convertLine(l)),
      openYmd: raw.open_ymd,
      postal_code: raw.post,
      prefId: raw.pref_cd,
      groupId: raw.station_g_cd,
      name: raw.station_name,
      nameK: raw.station_name_k,
      nameR: raw.station_name_r,
    };
  }
}
