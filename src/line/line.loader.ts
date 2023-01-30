import { Injectable, Scope } from '@nestjs/common';
import { BaseDataloader } from 'src/BaseDataloader';
import { Line } from 'src/models/line.model';
import { LineService } from './line.service';

@Injectable({ scope: Scope.REQUEST })
export default class LineDataLoader extends BaseDataloader<number, Line> {
  constructor(private readonly lineService: LineService) {
    super();
  }

  protected batchLoad(keys: number[]) {
    return this.lineService.getByIds(keys);
  }
}
