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

  async findOneByLineGroupId(lineGroupId: number): Promise<TrainType> {
    const trainType = await this.trainTypeRepo.findOne(lineGroupId);
    const belongingStations = await this.trainTypeRepo.getBelongingStations([
      lineGroupId,
    ]);
    const belongingLines = await this.trainTypeRepo.getBelongingLines([
      lineGroupId,
    ]);

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

    return convertTrainType(trainType, trainTypeStations, trainTypeLines);
  }
}
