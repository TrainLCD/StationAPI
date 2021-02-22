import { Injectable } from '@nestjs/common';
import { TrainType } from 'src/graphql';
import { RawService } from 'src/raw/raw.service';
import { TrainTypeRepository } from './trainType.repository';

@Injectable()
export class TrainTypeService {
  constructor(
    private readonly trainTypeRepo: TrainTypeRepository,
    private readonly rawService: RawService,
  ) {}

  async findOne(lineGroupId: number, excludePass = false): Promise<TrainType> {
    const trainType = await this.trainTypeRepo.findOne(lineGroupId);
    const belongingStations = await this.trainTypeRepo.getBelongingStations(
      lineGroupId,
      excludePass,
    );
    const belongingLines = await this.trainTypeRepo.getBelongingLines(
      lineGroupId,
    );
    return this.rawService.convertTrainType(
      trainType,
      await Promise.all(
        belongingStations.map((bs) => this.rawService.convertStation(bs)),
      ),
      await Promise.all(
        belongingLines.map((bl) => this.rawService.convertLine(bl)),
      ),
    );
  }
}
