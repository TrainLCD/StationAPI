import { Module } from '@nestjs/common';
import { StationService } from './station.service';
import { StationResolver } from './station.resolver';

@Module({
  providers: [StationService, StationResolver],
})
export class StationModule {}
