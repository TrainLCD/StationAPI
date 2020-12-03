import { Injectable } from '@nestjs/common';
import { Station } from 'src/graphql';
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
    );
  }

  async findOneByGroupId(groupId: number): Promise<Station> {
    return this.rawService.convertStation(
      await this.stationRepo.findOneByGroupId(groupId),
    );
  }

  async findOneByCoords(latitude: number, longitude: number): Promise<Station> {
    return this.rawService.convertStation(
      await this.stationRepo.findOneByCoords(latitude, longitude),
    );
  }

  async getByLineId(lineId: number): Promise<Station[]> {
    return (await this.stationRepo.getByLineId(lineId)).map((s) =>
      this.rawService.convertStation(s),
    );
  }

  async getByName(name: string): Promise<Station[]> {
    return (await this.stationRepo.getByName(name)).map((s) =>
      this.rawService.convertStation(s),
    );
  }
}
