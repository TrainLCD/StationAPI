import { Injectable } from '@nestjs/common';
import { TrainType } from 'src/graphql';
import { LineRepository } from 'src/line/line.repository';
import { RawService } from 'src/raw/raw.service';
import { TrainTypeRepository } from './trainType.repository';

@Injectable()
export class TrainTypeService {
  constructor(
    private readonly trainTypeRepo: TrainTypeRepository,
    private readonly lineRepo: LineRepository,
    private readonly rawService: RawService,
  ) {}

  async getByIds(lineGroupIds: number[]): Promise<TrainType[]> {
    const trainTypes = await this.trainTypeRepo.getByIds(lineGroupIds);
    const belongingStations = await this.trainTypeRepo.getBelongingStations(
      lineGroupIds,
    );
    const belongingLines = await this.trainTypeRepo.getBelongingLines(
      lineGroupIds,
    );

    const belongingStationsLineIds = belongingStations.map((s) =>
      s.lines.map((l) => l.line_cd),
    );
    const belongingStationsCompanies = await Promise.all(
      belongingStationsLineIds.map(
        async (lids) => await this.lineRepo.getCompaniesByLineIds(lids),
      ),
    );

    const trainTypeStations = belongingStations.map((bs, i) =>
      this.rawService.convertStation(bs, belongingStationsCompanies[i]),
    );
    const trainTypeLines = belongingLines.map((bl) =>
      this.rawService.convertLine(bl),
    );

    return Promise.all(
      trainTypes.map((tt) => {
        return this.rawService.convertTrainType(
          tt,
          trainTypeStations,
          trainTypeLines,
        );
      }),
    );
  }
}
