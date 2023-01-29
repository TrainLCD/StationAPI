import { Injectable } from '@nestjs/common';
import { Line } from 'src/models/line.model';
import { convertLine } from 'src/utils/convert';
import { LineRepository } from './line.repository';

@Injectable()
export class LineService {
  constructor(private readonly lineRepo: LineRepository) {}

  async getByIds(ids: number[]): Promise<Line[]> {
    const lines = await this.lineRepo.getByIds(ids);
    const companies = await this.lineRepo.getCompaniesByLineIds(ids);
    return lines.map((l, i) => convertLine(l, companies[i]));
  }

  async getLinesByGroupId(groupId: number): Promise<Line[]> {
    return await Promise.all(
      (
        await this.lineRepo.getByGroupId(groupId)
      ).map(async (l) =>
        convertLine(l, await this.lineRepo.findOneCompany(l.line_cd)),
      ),
    );
  }
}
