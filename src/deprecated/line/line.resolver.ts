import { Args, Directive, Query, Resolver } from '@nestjs/graphql';
import { Line } from 'src/models/line.model';
import LineDataLoader from './line.loader';

@Resolver(Line)
export class LineResolver {
  constructor(private readonly lineDataLoader: LineDataLoader) {}

  @Directive(
    '@deprecated(reason: "This query will be removed in the next version")',
  )
  @Query(() => Line)
  async line(@Args('id') id: number): Promise<Line> {
    return this.lineDataLoader.load(id);
  }
}
