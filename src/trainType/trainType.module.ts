import { Module } from '@nestjs/common';
import { LineRepository } from 'src/line/line.repository';
import { MysqlService } from 'src/mysql/mysql.service';
import { RawService } from 'src/raw/raw.service';
import TrainTypeDataLoader from './trainType.loader';
import { TrainTypeRepository } from './trainType.repository';
import { TrainTypeResolver } from './trainType.resolver';
import { TrainTypeService } from './trainType.service';

@Module({
  providers: [
    TrainTypeResolver,
    TrainTypeService,
    TrainTypeDataLoader,
    TrainTypeRepository,
    RawService,
    MysqlService,
    LineRepository,
  ],
})
export class TrainTypeModule {}
