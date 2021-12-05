import { Args, Query, Resolver } from '@nestjs/graphql';
import { TrainType } from 'src/graphql';
import { TrainTypeService } from './trainType.service';

@Resolver((of) => TrainType)
export class TrainTypeResolver {
  constructor(private readonly trainTypeService: TrainTypeService) {}

  @Query((returns) => TrainType)
  async trainType(
    @Args('id') id: number,
    @Args('excludePass') excludePass: boolean,
  ): Promise<TrainType> {
    return this.trainTypeService.findOne(id, excludePass);
  }
}
