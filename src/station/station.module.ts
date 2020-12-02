import { Module } from '@nestjs/common';
import { StationService } from './station.service';
import { StationResolver } from './station.resolver';
import { MysqlService } from 'src/mysql/mysql.service';
import { StationRepository } from './station.repository';
import { LineResolver } from './line.resolver';

@Module({
  providers: [
    StationService,
    StationResolver,
    MysqlService,
    StationRepository,
    LineResolver,
  ],
})
export class StationModule {}
