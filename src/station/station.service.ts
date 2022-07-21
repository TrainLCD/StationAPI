import { Injectable } from '@nestjs/common';
import { Station, StationOnly } from 'src/graphql';
import { LineRepository } from 'src/line/line.repository';
import { LineRaw } from 'src/line/models/LineRaw';
import { RawService } from 'src/raw/raw.service';
import { StationRepository } from './station.repository';

@Injectable()
export class StationService {
  constructor(
    private readonly stationRepo: StationRepository,
    private readonly lineRepo: LineRepository,
    private readonly rawService: RawService,
  ) {}

  async findOneById(id: number): Promise<Station> {
    const station = await this.stationRepo.findOneById(id);
    return this.rawService.convertStation(
      station,
      await this.lineRepo.findOneCompany(station.line_cd),
      await this.stationRepo.findTrainTypesById(id),
    );
  }

  async getStationsByIds(ids: number[]): Promise<Station[]> {
    const stations = await this.stationRepo.getByIds(ids);
    return Promise.all(
      stations.map(async (s) =>
        this.rawService.convertStation(
          s,
          await this.lineRepo.findOneCompany(s.line_cd),
          await this.stationRepo.findTrainTypesById(s.station_cd),
        ),
      ),
    );
  }

  async getStationsByGroupIds(ids: number[]): Promise<Station[]> {
    const stations = await this.stationRepo.getByIds(ids);
    return Promise.all(
      stations.map(async (s) =>
        this.rawService.convertStation(
          s,
          await this.lineRepo.findOneCompany(s.line_cd),
          await this.stationRepo.findTrainTypesById(s.station_cd),
        ),
      ),
    );
  }

  async findOneByGroupId(groupId: number): Promise<Station> {
    const station = await this.stationRepo.findOneByGroupId(groupId);
    return this.rawService.convertStation(
      {
        ...station,
        primary_station_number: null,
        secondary_station_number: null,
        extra_station_number: null,
      },
      await this.lineRepo.findOneCompany(station?.line_cd),
      await this.stationRepo.findTrainTypesById(station?.station_cd),
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

    return Promise.all(
      stations.map(async (s) =>
        this.rawService.convertStation(
          s,
          await this.lineRepo.findOneCompany(s.line_cd),
          await this.stationRepo.findTrainTypesById(s.station_cd),
        ),
      ),
    );
  }

  async getByLineId(lineId: number): Promise<Station[]> {
    return await Promise.all(
      (await this.stationRepo.getByLineId(lineId)).map(async (s) => {
        const trainTypes = await Promise.all(
          (await this.stationRepo.findTrainTypesById(s?.station_cd)).map(
            async (tt) => {
              const lines = await Promise.all(
                tt.lines.map(async (l) =>
                  this.rawService.convertLine(
                    l as LineRaw,
                    await this.lineRepo.findOneCompany((l as LineRaw).line_cd),
                  ),
                ),
              );
              return {
                ...tt,
                lines,
              };
            },
          ),
        );

        return this.rawService.convertStation(
          s,
          await this.lineRepo.findOneCompany(s.line_cd),
          trainTypes,
        );
      }),
    );
  }

  async getByLineIds(lineIds: number[]): Promise<Station[]> {
    const stations = await this.stationRepo.getByLineIds(lineIds);
    const stationIds = stations.map((s) => s.station_cd);
    const trainTypesGroupedByStations = await this.stationRepo.getTrainTypesByIds(
      stationIds,
    );

    return Promise.all(
      stations.map(async (s, i) => {
        const trainTypes = trainTypesGroupedByStations.length
          ? await Promise.all(
              trainTypesGroupedByStations[i].map(async (tt) => ({
                ...tt,
                lines: await Promise.all(
                  tt.lines.map(async (l) =>
                    this.rawService.convertLine(
                      l as LineRaw,
                      await this.lineRepo.findOneCompany(
                        (l as LineRaw).line_cd,
                      ),
                    ),
                  ),
                ),
              })),
            )
          : [];
        return this.rawService.convertStation(
          s,
          await this.lineRepo.findOneCompany(s.line_cd),
          trainTypes,
        );
      }),
    );
  }

  async getByNames(names: string[]): Promise<Station[][]> {
    return Promise.all(
      names.map(async (name) => {
        const stations = await this.stationRepo.getByName(name);

        return stations.map((s) => this.rawService.convertStation(s, null, []));
      }),
    );
  }

  async getAllStations(): Promise<StationOnly[]> {
    return await Promise.all(
      (await this.stationRepo.getAll()).map(async (s) =>
        this.rawService.convertStation(
          s,
          await this.lineRepo.findOneCompany(s?.line_cd),
        ),
      ),
    );
  }

  async getRandomStation(): Promise<Station> {
    const station = await this.stationRepo.getRandomly();
    return this.rawService.convertStation(
      station,
      await this.lineRepo.findOneCompany(station?.line_cd),
      await this.stationRepo.findTrainTypesById(station?.station_cd),
    );
  }
}
