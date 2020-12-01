import { ParseIntPipe } from '@nestjs/common';
import { Args, Query, Resolver } from '@nestjs/graphql';
import { Line, Station } from 'src/graphql';
import { StationService } from './station.service';

@Resolver()
export class StationResolver {
  constructor(private readonly stationService: StationService) {}

  @Query('station')
  async findOneById(@Args('id', ParseIntPipe) id: number): Promise<Station> {
    return this.stationService.findOneById(id);
  }

  @Query('stationByGroupId')
  async findOneByGroupId(@Args('groupId') groupId: number): Promise<Station> {
    return this.stationService.findOneByGroupId(groupId);
  }

  @Query('stationsByCoords')
  async getByCoords(
    @Args('latitude') latitude: number,
    @Args('longitude') longitude: number,
  ): Promise<Station[]> {
    return this.stationService.getByCoords(latitude, longitude);
  }

  @Query('stationsByLineId')
  async findOneByLineId(@Args('id') id: number): Promise<Station[]> {
    return this.stationService.getByLineId(id);
  }

  @Query('line')
  async findOneLineById(@Args('id') id: number): Promise<Line> {
    return this.stationService.findOneLineById(id);
  }
}
