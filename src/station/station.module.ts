import { Module } from '@nestjs/common';
import { LineRepository } from 'src/line/line.repository';
import { MysqlService } from 'src/mysql/mysql.service';
import { RawService } from 'src/raw/raw.service';
import { TrainTypeRepository } from 'src/trainType/trainType.repository';
import StationCoordsDataLoader from './station.coords.loader';
import StationGroupDataLoader from './station.group.loader';
import StationLineDataLoader from './station.line.loader';
import StationDataLoader from './station.loader';
import StationNameDataLoader from './station.name.loader';
import { StationRepository } from './station.repository';
import { StationResolver } from './station.resolver';
import { StationService } from './station.service';

@Module({
  providers: [
    StationService,
    StationResolver,
    StationDataLoader,
    StationGroupDataLoader,
    StationLineDataLoader,
    StationNameDataLoader,
    StationCoordsDataLoader,
    MysqlService,
    StationRepository,
    RawService,
    LineRepository,
    TrainTypeRepository,
  ],
})
export class StationModule {}
