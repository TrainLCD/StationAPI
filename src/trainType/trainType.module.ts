import { Module } from '@nestjs/common';
import { DbModule } from 'src/db/db.module';
import { LineRepository } from '../line/line.repository';
import { TrainTypeRepository } from './trainType.repository';
import { TrainTypeResolver } from './trainType.resolver';
import { TrainTypeService } from './trainType.service';

@Module({
  imports: [DbModule],
  providers: [
    TrainTypeResolver,
    TrainTypeService,
    TrainTypeRepository,
    LineRepository,
  ],
})
export class TrainTypeModule {}
