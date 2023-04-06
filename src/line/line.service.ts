import { Injectable } from '@nestjs/common';
import { Line } from 'src/models/line.model';
import { convertLine } from 'src/utils/convert';
import { LineRepository } from './line.repository';

@Injectable()
export class LineService {
  constructor(private readonly lineRepo: LineRepository) {}

  async findOne(id: number): Promise<Line> {
    const line = await this.lineRepo.findOne(id);
    const company = (await this.lineRepo.findOneCompany(id))[0];
    return convertLine(line, company);
  }
}
