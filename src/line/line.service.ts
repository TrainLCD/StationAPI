import { Injectable } from '@nestjs/common';
import { Line } from 'src/graphql';
import { RawService } from 'src/raw/raw.service';
import { LineRepository } from './line.repository';

@Injectable()
export class LineService {
  constructor(
    private readonly lineRepo: LineRepository,
    private readonly rawService: RawService,
  ) {}

  async findOne(id: number): Promise<Line> {
    return this.rawService.convertLine(
      await this.lineRepo.findOne(id),
      await this.lineRepo.findOneCompany(id),
    );
  }

  async getByIds(ids: number[]): Promise<Line[]> {
    return Promise.all(
      ids.map(async (id) =>
        this.rawService.convertLine(
          await this.lineRepo.findOne(id),
          await this.lineRepo.findOneCompany(id),
        ),
      ),
    );
  }

  async getLinesByGroupId(groupId: number): Promise<Line[]> {
    return await Promise.all(
      (await this.lineRepo.getByGroupId(groupId)).map(async (l) =>
        this.rawService.convertLine(
          l,
          await this.lineRepo.findOneCompany(l.line_cd),
        ),
      ),
    );
  }
}
