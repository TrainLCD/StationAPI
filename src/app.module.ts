import { Module } from '@nestjs/common';
import { GraphQLModule } from '@nestjs/graphql';
import { join } from 'path';
import { StationModule } from './station/station.module';
import { LineModule } from './line/line.module';
import { TrainTypeModule } from './trainType/trainType.module';

@Module({
  imports: [
    GraphQLModule.forRoot({
      typePaths: ['./**/*.graphql'],
      playground: true,
      definitions: {
        path: join(process.cwd(), 'src/graphql.ts'),
        outputAs: 'class',
      },
    }),
    StationModule,
    LineModule,
    TrainTypeModule,
  ],
  controllers: [],
  providers: [],
})
export class AppModule {}
