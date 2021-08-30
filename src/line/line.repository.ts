import { Injectable } from '@nestjs/common';
import { RowDataPacket } from 'mysql2';
import { NEX_ID } from 'src/constants/ignore';
import { MysqlService } from 'src/mysql/mysql.service';
import { CompanyRaw, LineRaw } from './models/LineRaw';

@Injectable()
export class LineRepository {
  constructor(private readonly mysqlService: MysqlService) {}

  async findOne(id: number): Promise<LineRaw> {
    const { connection } = this.mysqlService;

    return new Promise<LineRaw>((resolve, reject) => {
      connection.query(
        `SELECT *
        FROM \`lines\`
        WHERE line_cd = ?
        AND NOT line_cd = ${NEX_ID}
        AND e_status = 0`,
        [id],
        (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }
          return resolve(results[0] as LineRaw);
        },
      );
    });
  }

  async findOneCompany(lineId: number): Promise<CompanyRaw> {
    const { connection } = this.mysqlService;

    return new Promise<CompanyRaw>((resolve, reject) => {
      connection.query(
        `SELECT c.*
        FROM \`lines\` as l, \`companies\` as c
        WHERE l.line_cd = ?
        AND l.e_status = 0
        AND c.company_cd = l.company_cd`,
        [lineId],
        (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }
          return resolve(results[0] as CompanyRaw);
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
        `SELECT l.*
        FROM \`lines\` as l, companies as c
        WHERE l.line_cd
        IN (SELECT line_cd FROM stations WHERE station_g_cd = ?)
        AND NOT l.line_cd = ${NEX_ID}
        AND l.e_status = 0
        AND l.company_cd = c.company_cd`,
        [groupId],
        (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }

          return resolve(results as LineRaw[]);
        },
      );
    });
  }
}
