import { Injectable, Scope } from '@nestjs/common';
import { BaseDataloader } from 'src/BaseDataloader';
import { Station } from 'src/models/station.model';
import { StationService } from './station.service';

@Injectable({ scope: Scope.REQUEST })
export default class StationLineDataLoader extends BaseDataloader<
  number,
  Station[]
> {
  constructor(private readonly stationService: StationService) {
    super();
  }

  protected async batchLoad(keys: number[]) {
    const stations = await this.stationService.getByLineIds(keys);
    return [stations];
  }
}
