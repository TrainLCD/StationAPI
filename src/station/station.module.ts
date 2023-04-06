import { Module } from '@nestjs/common';
import { DbModule } from 'src/db/db.module';
import { LineRepository } from '../line/line.repository';
import { TrainTypeRepository } from '../trainType/trainType.repository';
import { StationRepository } from './station.repository';
import { StationResolver } from './station.resolver';
import { StationService } from './station.service';

@Module({
  imports: [DbModule],
  providers: [
    StationService,
    StationResolver,
    StationRepository,
    LineRepository,
    TrainTypeRepository,
  ],
})
export class StationModule {}
