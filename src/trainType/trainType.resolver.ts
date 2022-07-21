import { Args, Query, Resolver } from '@nestjs/graphql';
import { TrainType } from 'src/graphql';
import TrainTypeDataLoader from './trainType.loader';

@Resolver(TrainType)
export class TrainTypeResolver {
  constructor(private readonly trainTypeDataLoader: TrainTypeDataLoader) {}

  @Query(() => TrainType)
  async trainType(@Args('id') id: number): Promise<TrainType> {
    return this.trainTypeDataLoader.load(id);
  }
}
