import { Injectable } from '@nestjs/common';
import { Station, StationOnly } from 'src/graphql';
import { LineRaw } from 'src/line/models/LineRaw';
import { RawService } from 'src/raw/raw.service';
import { StationRepository } from './station.repository';

@Injectable()
export class StationService {
  constructor(
    private readonly stationRepo: StationRepository,
    private readonly rawService: RawService,
  ) {}

  async findOneById(id: number): Promise<Station> {
    return this.rawService.convertStation(
      await this.stationRepo.findOneById(id),
      await this.stationRepo.findTrainTypesById(id),
    );
  }

  async findOneByGroupId(groupId: number): Promise<Station> {
    const station = await this.stationRepo.findOneByGroupId(groupId);
    return this.rawService.convertStation(
      station,
      await this.stationRepo.findTrainTypesById(station?.station_cd),
    );
  }

  async findOneByCoords(latitude: number, longitude: number): Promise<Station> {
    const station = await this.stationRepo.findOneByCoords(latitude, longitude);

    return this.rawService.convertStation(
      station,
      await this.stationRepo.findTrainTypesById(station?.station_cd),
    );
  }

  async getByLineId(lineId: number): Promise<Station[]> {
    return await Promise.all(
      (await this.stationRepo.getByLineId(lineId)).map(async (s) => {
        const trainTypes = (
          await this.stationRepo.findTrainTypesById(s?.station_cd)
        ).map((tt) => {
          const lines = tt.lines.map((l) =>
            this.rawService.convertLine(l as LineRaw),
          );
          return {
            ...tt,
            lines,
          };
        });

        return this.rawService.convertStation(s, trainTypes);
      }),
    );
  }

  async getByName(name: string): Promise<Station[]> {
    return await Promise.all(
      (await this.stationRepo.getByName(name)).map(async (s) =>
        this.rawService.convertStation(
          s,
          await this.stationRepo.findTrainTypesById(s?.station_cd),
        ),
      ),
    );
  }

  async getAllStations(): Promise<StationOnly[]> {
    return await Promise.all(
      (await this.stationRepo.getAll()).map(async (s) =>
        this.rawService.convertStation(s),
      ),
    );
  }
}
