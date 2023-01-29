import { Injectable, Scope } from '@nestjs/common';
import { BaseDataloader } from 'src/BaseDataloader';
import { Station } from 'src/models/station.model';
import { StationService } from './station.service';

@Injectable({ scope: Scope.REQUEST })
export default class StationDataLoader extends BaseDataloader<number, Station> {
  constructor(private readonly stationService: StationService) {
    super();
  }

  protected batchLoad(keys: number[]) {
    return this.stationService.getStationsByIds(keys);
  }
}
