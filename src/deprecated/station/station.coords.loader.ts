import { Injectable, Scope } from '@nestjs/common';
import { BaseDataloader } from 'src/BaseDataloader';
import { Station } from 'src/models/station.model';
import { StationService } from './station.service';

@Injectable({ scope: Scope.REQUEST })
export default class StationCoordsDataLoader extends BaseDataloader<
  [number, number, number | undefined],
  Station[]
> {
  constructor(private readonly stationService: StationService) {
    super();
  }

  protected async batchLoad(keys: [number, number, number | undefined][]) {
    return Promise.all(
      keys.map((k) => this.stationService.getByCoords(k[0], k[1], k[2])),
    );
  }
}
