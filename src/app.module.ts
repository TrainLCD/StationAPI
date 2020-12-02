import { Module } from '@nestjs/common';
import { GraphQLModule } from '@nestjs/graphql';
import { join } from 'path';
import { StationService } from './station/station.service';
import { StationModule } from './station/station.module';
import { MysqlService } from './mysql/mysql.service';
import { StationRepository } from './station/station.repository';

@Module({
  imports: [
    GraphQLModule.forRoot({
      typePaths: ['./**/*.graphql'],
      definitions: {
        path: join(process.cwd(), 'src/graphql.ts'),
        outputAs: 'class',
      },
    }),
    StationModule,
  ],
  controllers: [],
  providers: [StationService, MysqlService, StationRepository],
})
export class AppModule {}
