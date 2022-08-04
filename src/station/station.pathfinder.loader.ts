import { Injectable, Scope } from '@nestjs/common';
import { BaseDataloader } from 'src/BaseDataloader';
import { FoundPath } from 'src/graphql';

import { StationService } from './station.service';

@Injectable({ scope: Scope.REQUEST })
export default class StatioPathfinderDataLoader extends BaseDataloader<
  [number, number],
  FoundPath[]
> {
  constructor(private readonly stationService: StationService) {
    super();
  }

  protected async batchLoad(keys: [number, number][]) {
    return this.stationService.findPath(keys);
  }
}
