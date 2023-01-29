import { Args, Directive, ID, Query, Resolver } from '@nestjs/graphql';
import { TrainType } from 'src/models/traintype.model';
import TrainTypeDataLoader from './trainType.loader';

@Resolver(TrainType)
export class TrainTypeResolver {
  constructor(private readonly trainTypeDataLoader: TrainTypeDataLoader) {}

  @Directive(
    '@deprecated(reason: "This query will be removed in the next version")',
  )
  @Query(() => TrainType)
  async trainType(
    @Args('id', { type: () => ID }) id: number,
  ): Promise<TrainType> {
    return this.trainTypeDataLoader.load(id);
  }
}
