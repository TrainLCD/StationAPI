import { Args, Query, Resolver } from '@nestjs/graphql';
import { Line } from 'src/models/line.model';
import { LineService } from './line.service';

@Resolver(Line)
export class LineResolver {
  constructor(private readonly lineService: LineService) {}

  @Query(() => Line)
  async line(@Args('id') id: number): Promise<Line> {
    return this.lineService.findOne(id);
  }
}
