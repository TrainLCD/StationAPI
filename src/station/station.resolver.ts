import { ParseIntPipe } from '@nestjs/common';
import { Args, Query, Resolver } from '@nestjs/graphql';
import { FoundPath, Station } from 'src/graphql';
import StationCoordsDataLoader from './station.coords.loader';
import StatioGroupDataLoader from './station.group.loader';
import StationLineDataLoader from './station.line.loader';
import StationDataLoader from './station.loader';
import StationNameDataLoader from './station.name.loader';
import StatioPathfinderDataLoader from './station.pathfinder.loader';
import { StationService } from './station.service';

@Resolver(Station)
export class StationResolver {
  constructor(
    private readonly stationService: StationService,
    private readonly stationDataLoader: StationDataLoader,
    private readonly stationGroupDataLoader: StatioGroupDataLoader,
    private readonly stationLineDataLoader: StationLineDataLoader,
    private readonly stationNameDataLoader: StationNameDataLoader,
    private readonly stationCoordsDataLoader: StationCoordsDataLoader,
    private readonly pathfinderDataLoader: StatioPathfinderDataLoader,
  ) {}

  @Query(() => Station)
  async station(@Args('id', ParseIntPipe) id: number): Promise<Station> {
    return this.stationDataLoader.load(id);
  }

  @Query(() => Station)
  async stationByGroupId(@Args('groupId') groupId: number): Promise<Station> {
    return this.stationGroupDataLoader.load(groupId);
  }

  @Query(() => [Station])
  async nearbyStations(
    @Args('latitude') latitude: number,
    @Args('longitude') longitude: number,
    @Args('limit') limit = 1,
  ): Promise<Station[]> {
    return this.stationCoordsDataLoader.load([latitude, longitude, limit]);
  }

  @Query(() => [Station])
  async stationsByLineId(@Args('lineId') lineId: number): Promise<Station[]> {
    return this.stationLineDataLoader.load(lineId);
  }

  @Query(() => [Station])
  async stationsByName(@Args('name') name: string): Promise<Station[]> {
    return this.stationNameDataLoader.load(name);
  }

  @Query(() => Station)
  async random(): Promise<Station> {
    return this.stationService.getRandomStation();
  }

  @Query(() => [FoundPath])
  async pathfinder(
    @Args('srcGroupId') srcGroupId: number,
    @Args('dstGroupId') dstGroupId: number,
  ): Promise<FoundPath[]> {
    return this.pathfinderDataLoader.load([srcGroupId, dstGroupId]);
  }
}
