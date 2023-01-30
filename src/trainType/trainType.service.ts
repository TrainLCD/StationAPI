import { Injectable } from '@nestjs/common';
import { TrainType } from 'src/models/traintype.model';
import {
  convertLine,
  convertStation,
  convertTrainType,
} from 'src/utils/convert';
import { LineRepository } from '../line/line.repository';
import { TrainTypeRepository } from './trainType.repository';

@Injectable()
export class TrainTypeService {
  constructor(
    private readonly trainTypeRepo: TrainTypeRepository,
    private readonly lineRepo: LineRepository,
  ) {}

  async getByIds(lineGroupIds: number[]): Promise<TrainType[]> {
    const trainTypes = await this.trainTypeRepo.getByIds(lineGroupIds);
    const belongingStations = await this.trainTypeRepo.getBelongingStations(
      lineGroupIds,
    );
    const belongingLines = await this.trainTypeRepo.getBelongingLines(
      lineGroupIds,
    );

    const stationsByGroupIds = await this.trainTypeRepo.getStationsByGroupIds(
      belongingStations.map((bs) => bs.station_g_cd),
    );

    const belongingStationsCompanies =
      await this.lineRepo.getCompaniesByLineIds(
        stationsByGroupIds.map((s) => s.line_cd),
      );

    const trainTypeStations = belongingStations.map((bs) =>
      convertStation(bs, belongingStationsCompanies),
    );
    const trainTypeLines = belongingLines.map((bl) => convertLine(bl));

    return Promise.all(
      trainTypes.map((tt) => {
        return convertTrainType(tt, trainTypeStations, trainTypeLines);
      }),
    );
  }
}
