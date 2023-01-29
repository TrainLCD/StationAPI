import { Args, ID, Query, Resolver } from '@nestjs/graphql';
import { TrainType } from 'src/models/traintype.model';
import TrainTypeDataLoader from './trainType.loader';

@Resolver(TrainType)
export class TrainTypeResolver {
  constructor(private readonly trainTypeDataLoader: TrainTypeDataLoader) {}

  @Query(() => TrainType)
  async trainType(
    @Args('id', { type: () => ID }) id: number,
  ): Promise<TrainType> {
    return this.trainTypeDataLoader.load(id);
  }
}
