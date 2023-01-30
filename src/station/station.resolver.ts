import { ParseIntPipe } from '@nestjs/common';
import { Args, Float, ID, Int, Query, Resolver } from '@nestjs/graphql';
import { Station } from 'src/models/station.model';
import StationCoordsDataLoader from './station.coords.loader';
import StatioGroupDataLoader from './station.group.loader';
import StationLineDataLoader from './station.line.loader';
import StationDataLoader from './station.loader';
import StationNameDataLoader from './station.name.loader';
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
    @Args('latitude', { type: () => Float }) latitude: number,
    @Args('longitude', { type: () => Float }) longitude: number,
    @Args('limit', { type: () => Int, nullable: true }) limit = 1,
  ): Promise<Station[]> {
    return this.stationCoordsDataLoader.load([latitude, longitude, limit]);
  }

  @Query(() => [Station])
  async stationsByLineId(
    @Args('lineId', { type: () => ID }) lineId: number,
  ): Promise<Station[]> {
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
}
