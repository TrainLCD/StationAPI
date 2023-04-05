import { ApolloDriver, ApolloDriverConfig } from '@nestjs/apollo';
import { Module } from '@nestjs/common';
import { GraphQLModule } from '@nestjs/graphql';
import { join } from 'path';
import { LineModule } from './line/line.module';
import { StationModule } from './station/station.module';
import { TrainTypeModule } from './trainType/trainType.module';
import { DbModule } from './db/db.module';

@Module({
  imports: [
    GraphQLModule.forRoot<ApolloDriverConfig>({
      driver: ApolloDriver,
      autoSchemaFile: join(process.cwd(), 'src/schema.gql'),
    }),
    StationModule,
    LineModule,
    TrainTypeModule,
    DbModule,
  ],
  controllers: [],
  providers: [],
})
export class AppModule {}
