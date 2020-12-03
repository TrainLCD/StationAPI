import { Injectable } from '@nestjs/common';
import { NEX_ID } from 'src/constants/ignore';
import { MysqlService } from 'src/mysql/mysql.service';
import { LineRaw } from './models/LineRaw';

@Injectable()
export class LineRepository {
  constructor(private readonly mysqlService: MysqlService) {}

  async findOne(id: number): Promise<LineRaw> {
    const { connection } = this.mysqlService;

    return new Promise<LineRaw>((resolve, reject) => {
      connection.query(
        `SELECT * FROM \`lines\` WHERE line_cd = ? AND NOT line_cd = ${NEX_ID}`,
        [id],
        (err, results) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }
          return resolve(results[0]);
        },
      );
    });
  }

  async getByGroupId(groupId: number): Promise<LineRaw[]> {
    const { connection } = this.mysqlService;
    if (!connection) {
      return [];
    }

    return new Promise<LineRaw[]>((resolve, reject) => {
      connection.query(
        `SELECT * FROM \`lines\` WHERE line_cd IN (SELECT line_cd FROM stations WHERE station_g_cd = ?) AND NOT line_cd = ${NEX_ID}`,
        [groupId],
        (err, results) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }

          return resolve(results);
        },
      );
    });
  }
}
