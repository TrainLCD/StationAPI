import { Injectable } from '@nestjs/common';
import { Line, Station } from 'src/graphql';

@Injectable()
export class StationService {
  private readonly stations: Station[] = [];
  private readonly lines: Line[] = [];

  findOneById(id: number): Station {
    return this.stations.find((s) => s.id === id);
  }

  findOneByGroupId(groupId: number): Station {
    return this.stations.find((s) => s.groupId === groupId);
  }

  getByCoords(latitude: number, longitude: number): Station[] {
    return this.stations;
  }

  getByLineId(lineId: number): Station[] {
    return this.stations;
  }

  findOneLineById(id: number): Line {
    return this.lines.find((l) => l.id === id);
  }
}
