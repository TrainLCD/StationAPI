import { Module } from '@nestjs/common';
import { DbModule } from 'src/db/db.module';
import { LineRepository } from './line.repository';
import { LineResolver } from './line.resolver';
import { LineService } from './line.service';

@Module({
  imports: [DbModule],
  providers: [LineResolver, LineService, LineRepository],
})
export class LineModule {}
