import { Module } from '@nestjs/common';
import { DbModule } from 'src/db/db.module';
import { LineRepository } from '../line/line.repository';
import { TrainTypeRepository } from '../trainType/trainType.repository';
import StationCoordsDataLoader from './station.coords.loader';
import StationGroupDataLoader from './station.group.loader';
import StationLineDataLoader from './station.line.loader';
import StationDataLoader from './station.loader';
import StationNameDataLoader from './station.name.loader';
import { StationRepository } from './station.repository';
import { StationResolver } from './station.resolver';
import { StationService } from './station.service';

@Module({
  imports: [DbModule],
  providers: [
    StationService,
    StationResolver,
    StationDataLoader,
    StationGroupDataLoader,
    StationLineDataLoader,
    StationNameDataLoader,
    StationCoordsDataLoader,
    StationRepository,
    LineRepository,
    TrainTypeRepository,
  ],
})
export class StationModule {}
