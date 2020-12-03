import { Module } from '@nestjs/common';
import { StationService } from './station.service';
import { StationResolver } from './station.resolver';
import { MysqlService } from 'src/mysql/mysql.service';
import { StationRepository } from './station.repository';
import { RawService } from 'src/raw/raw.service';
import { LineRepository } from 'src/line/line.repository';

@Module({
  providers: [
    StationService,
    StationResolver,
    MysqlService,
    StationRepository,
    RawService,
    LineRepository,
  ],
})
export class StationModule {}
