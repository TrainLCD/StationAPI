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
    return this.rawService.convertStation(
      await this.stationRepo.findOneById(id),
      await this.lineRepo.findOneCompany(id),
      await this.stationRepo.findTrainTypesById(id),
    );
  }

  async findOneByGroupId(groupId: number): Promise<Station> {
    const station = await this.stationRepo.findOneByGroupId(groupId);
    return this.rawService.convertStation(
      station,
      await this.lineRepo.findOneCompany(station?.line_cd),
      await this.stationRepo.findTrainTypesById(station?.station_cd),
    );
  }

  /**
   * @deprecated New API is using `getByCoords` instead.
   */
  async findOneByCoords(latitude: number, longitude: number): Promise<Station> {
    const station = await this.stationRepo.findOneByCoords(latitude, longitude);

    return this.rawService.convertStation(
      station,
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

  async getByName(name: string): Promise<Station[]> {
    return await Promise.all(
      (await this.stationRepo.getByName(name)).map(async (s) =>
        this.rawService.convertStation(
          s,
          await this.lineRepo.findOneCompany(s?.line_cd),
          await this.stationRepo.findTrainTypesById(s?.station_cd),
        ),
      ),
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
