import { ApolloDriver, ApolloDriverConfig } from '@nestjs/apollo';
import { Module } from '@nestjs/common';
import { GraphQLModule } from '@nestjs/graphql';
import { join } from 'path';
import { LineModule } from './deprecated/line/line.module';
import { StationModule } from './deprecated/station/station.module';
import { TrainTypeModule } from './deprecated/trainType/trainType.module';

@Module({
  imports: [
    GraphQLModule.forRoot<ApolloDriverConfig>({
      driver: ApolloDriver,
      autoSchemaFile: join(process.cwd(), 'src/schema.gql'),
    }),
    StationModule,
    LineModule,
    TrainTypeModule,
  ],
  controllers: [],
  providers: [],
})
export class AppModule {}
