import { Module } from '@nestjs/common';
import { MysqlService } from 'src/mysql/mysql.service';
import LineDataLoader from './line.loader';
import { LineRepository } from './line.repository';
import { LineResolver } from './line.resolver';
import { LineService } from './line.service';

@Module({
  providers: [
    LineResolver,
    LineService,
    LineDataLoader,
    LineRepository,
    MysqlService,
  ],
})
export class LineModule {}
