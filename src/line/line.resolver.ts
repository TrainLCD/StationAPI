import { Args, Query, Resolver } from '@nestjs/graphql';
import { Line } from 'src/graphql';
import LineDataLoader from './line.loader';

@Resolver(Line)
export class LineResolver {
  constructor(private readonly lineDataLoader: LineDataLoader) {}

  @Query(() => Line)
  async line(@Args('id') id: number): Promise<Line> {
    return this.lineDataLoader.load(id);
  }
}
