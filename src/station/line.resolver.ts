import { Args, Resolver, Query } from '@nestjs/graphql';
import { Line } from 'src/graphql';
import { StationService } from './station.service';

@Resolver((of) => Line)
export class LineResolver {
  constructor(private readonly stationService: StationService) {}

  @Query((returns) => Line)
  async line(@Args('id') id: number): Promise<Line> {
    return this.stationService.findOneLineById(id);
  }
}
