import { Module } from '@nestjs/common';
import { MysqlService } from 'src/mysql/mysql.service';
import { LineRepository } from '../line/line.repository';
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
    MysqlService,
    LineRepository,
  ],
})
export class TrainTypeModule {}
