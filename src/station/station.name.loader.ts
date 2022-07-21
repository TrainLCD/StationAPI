import { Injectable, Scope } from '@nestjs/common';
import { BaseDataloader } from 'src/BaseDataloader';
import { Station } from 'src/graphql';

import { StationService } from './station.service';

@Injectable({ scope: Scope.REQUEST })
export default class StationNameDataLoader extends BaseDataloader<
  string,
  Station[]
> {
  constructor(private readonly stationService: StationService) {
    super();
  }

  protected async batchLoad(keys: string[]) {
    const stations = await this.stationService.getByNames(keys);
    return stations;
  }
}
