import { ParseIntPipe } from '@nestjs/common';
import { Args, Float, ID, Int, Query, Resolver } from '@nestjs/graphql';
import { Station } from 'src/models/station.model';
import { StationService } from './station.service';

@Resolver(Station)
export class StationResolver {
  constructor(private readonly stationService: StationService) {}

  @Query(() => Station)
  async station(@Args('id', ParseIntPipe) id: number): Promise<Station> {
    return this.stationService.findOne(id);
  }

  @Query(() => Station)
  async stationByGroupId(@Args('groupId') groupId: number): Promise<Station> {
    return this.stationService.findStationByGroupId(groupId);
  }

  @Query(() => [Station])
  async nearbyStations(
    @Args('latitude', { type: () => Float }) latitude: number,
    @Args('longitude', { type: () => Float }) longitude: number,
    @Args('limit', { type: () => Int, nullable: true }) limit = 1,
  ): Promise<Station[]> {
    return this.stationService.getByCoords(latitude, longitude, limit);
  }

  @Query(() => [Station])
  async stationsByLineId(
    @Args('lineId', { type: () => ID }) lineId: number,
  ): Promise<Station[]> {
    return this.stationService.getByLineId(lineId);
  }

  @Query(() => [Station])
  async stationsByName(@Args('name') name: string): Promise<Station[]> {
    return (await this.stationService.getByNames([name]))[0];
  }

  @Query(() => Station)
  async random(): Promise<Station> {
    return this.stationService.getRandomStation();
  }
}
