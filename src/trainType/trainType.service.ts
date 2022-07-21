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
    return Promise.all(
      trainTypes.map(async (tt) =>
        this.rawService.convertTrainType(
          tt,
          await Promise.all(
            belongingStations.map(async (bs) =>
              this.rawService.convertStation(
                bs,
                await this.lineRepo.findOneCompany(bs.line_cd),
              ),
            ),
          ),
          await Promise.all(
            belongingLines.map(async (bl) =>
              this.rawService.convertLine(
                bl,
                await this.lineRepo.findOneCompany(bl.line_cd),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
