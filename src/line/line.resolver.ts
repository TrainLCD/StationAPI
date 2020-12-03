import { Args, Resolver, Query } from '@nestjs/graphql';
import { Line } from 'src/graphql';
import { LineService } from './line.service';

@Resolver((of) => Line)
export class LineResolver {
  constructor(private readonly lineService: LineService) {}

  @Query((returns) => Line)
  async line(@Args('id') id: number): Promise<Line> {
    return this.lineService.findOne(id);
  }
}
