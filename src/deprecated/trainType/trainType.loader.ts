import { Injectable, Scope } from '@nestjs/common';
import { BaseDataloader } from 'src/BaseDataloader';
import { TrainType } from 'src/models/traintype.model';
import { TrainTypeService } from './trainType.service';

@Injectable({ scope: Scope.REQUEST })
export default class TrainTypeDataLoader extends BaseDataloader<
  number,
  TrainType
> {
  constructor(private readonly trainTypeService: TrainTypeService) {
    super();
  }

  protected batchLoad(keys: number[]) {
    return this.trainTypeService.getByIds(keys);
  }
}
