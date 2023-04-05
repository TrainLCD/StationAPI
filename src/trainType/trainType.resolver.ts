import { Args, ID, Query, Resolver } from '@nestjs/graphql';
import { TrainType } from 'src/models/traintype.model';
import { TrainTypeService } from './trainType.service';

@Resolver(TrainType)
export class TrainTypeResolver {
  constructor(private readonly trainTypeService: TrainTypeService) {}

  @Query(() => TrainType)
  async trainType(
    @Args('id', { type: () => ID }) id: number,
  ): Promise<TrainType> {
    return this.trainTypeService.findOneByLineGroupId(id);
  }
}
