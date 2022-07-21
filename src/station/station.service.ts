import { Injectable } from '@nestjs/common';
import { Station } from 'src/graphql';
import { LineRepository } from 'src/line/line.repository';
import { RawService } from 'src/raw/raw.service';
import { StationRepository } from './station.repository';

@Injectable()
export class StationService {
  constructor(
    private readonly stationRepo: StationRepository,
    private readonly lineRepo: LineRepository,
    private readonly rawService: RawService,
  ) {}

  async getStationsByIds(ids: number[]): Promise<Station[]> {
    const stations = await this.stationRepo.getByIds(ids);
    const lineIds = stations.map((s) => s.lines.map((l) => l.line_cd)).flat();
    const stationIds = stations.map((s) => s.station_cd);
    const companies = await this.lineRepo.getCompaniesByLineIds(lineIds);
    const trainTypes = await this.stationRepo.getTrainTypesByIds(stationIds);

    return stations.map((s, i) =>
      this.rawService.convertStation(s, companies, trainTypes[i]),
    );
  }

  async getStationsByGroupIds(ids: number[]): Promise<Station[]> {
    const stations = await this.stationRepo.getByIds(ids);
    const lineIds = stations.map((s) => s.lines.map((l) => l.line_cd)).flat();
    const stationIds = stations.map((s) => s.station_cd);
    const companies = await this.lineRepo.getCompaniesByLineIds(lineIds);
    const trainTypes = await this.stationRepo.getTrainTypesByIds(stationIds);

    return Promise.all(
      stations.map((s, i) =>
        this.rawService.convertStation(s, companies, trainTypes[i]),
      ),
    );
  }

  async getByCoords(
    latitude: number,
    longitude: number,
    limit?: number,
  ): Promise<Station[]> {
    const stations = await this.stationRepo.getByCoords(
      latitude,
      longitude,
      limit,
    );
    const lineIds = stations.map((s) => s.lines.map((l) => l.line_cd)).flat();
    const stationIds = stations.map((s) => s.station_cd);
    const companies = await this.lineRepo.getCompaniesByLineIds(lineIds);
    const trainTypes = await this.stationRepo.getTrainTypesByIds(stationIds);

    return Promise.all(
      stations.map((s, i) =>
        this.rawService.convertStation(s, companies, trainTypes[i]),
      ),
    );
  }

  async getByLineIds(lineIds: number[]): Promise<Station[]> {
    const stations = await this.stationRepo.getByLineIds(lineIds);
    const stationIds = stations.map((s) => s.station_cd);
    const stationLineIds = stations
      .map((s) => s.lines.map((l) => l.line_cd))
      .flat();
    const companies = await this.lineRepo.getCompaniesByLineIds(stationLineIds);
    const trainTypes = await this.stationRepo.getTrainTypesByIds(stationIds);
    return Promise.all(
      stations.map((s, i) =>
        this.rawService.convertStation(s, companies, trainTypes[i]),
      ),
    );
  }

  async getByNames(names: string[]): Promise<Station[][]> {
    return Promise.all(
      names.map(async (name) => {
        const stations = await this.stationRepo.getByName(name);

        return stations.map((s) => this.rawService.convertStation(s, [], []));
      }),
    );
  }

  async getRandomStation(): Promise<Station> {
    const station = await this.stationRepo.getRandomly();
    const companies = await this.lineRepo.getCompaniesByLineIds(
      station.lines.map((l) => l.line_cd),
    );
    return this.rawService.convertStation(station, companies, []);
  }
}
