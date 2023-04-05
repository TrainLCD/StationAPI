import { Inject, Injectable } from '@nestjs/common';
import { Connection, RowDataPacket } from 'mysql2';
import { DB_CONNECTION } from 'src/db/db.module';
import { CompanyRaw, LineRaw } from './models/LineRaw';

@Injectable()
export class LineRepository {
  constructor(@Inject(DB_CONNECTION) private readonly conn: Connection) {}

  async findOne(id: number): Promise<LineRaw> {
    return new Promise<LineRaw>((resolve, reject) => {
      this.conn.query(
        `SELECT *
        FROM \`lines\`
        WHERE line_cd = ?
        AND e_status = 0
        LIMIT 1`,
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

  async getByIds(ids: number[]): Promise<LineRaw[]> {
    return new Promise<LineRaw[]>((resolve, reject) => {
      this.conn.query(
        `SELECT *
        FROM \`lines\`
        WHERE line_cd in (?)
        AND e_status = 0`,
        [ids],
        (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }
          return resolve(results as LineRaw[]);
        },
      );
    });
  }

  async findOneCompany(lineId: number): Promise<CompanyRaw> {
    return new Promise<CompanyRaw>((resolve, reject) => {
      this.conn.query(
        `SELECT c.*
        FROM \`lines\` as l, \`companies\` as c
        WHERE l.line_cd = ?
        AND l.e_status = 0
        AND c.company_cd = l.company_cd
        LIMIT 1`,
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

  async getCompaniesByLineIds(lineIds: number[]): Promise<CompanyRaw[]> {
    return new Promise<CompanyRaw[]>((resolve, reject) => {
      this.conn.query(
        `SELECT c.*, l.line_cd
        FROM \`lines\` as l, \`companies\` as c
        WHERE l.line_cd in (?)
        AND l.e_status = 0
        AND c.company_cd = l.company_cd`,
        [lineIds],
        (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }
          return resolve(results as CompanyRaw[]);
        },
      );
    });
  }

  async getByGroupId(groupId: number): Promise<LineRaw[]> {
    return new Promise<LineRaw[]>((resolve, reject) => {
      this.conn.query(
        `SELECT *
        FROM \`lines\`
        WHERE line_cd
        IN (SELECT line_cd FROM stations WHERE station_g_cd = ? AND e_status = 0)
        AND e_status = 0`,
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

  async findOneStationId(stationId: number): Promise<LineRaw> {
    return new Promise<LineRaw>((resolve, reject) => {
      this.conn.query(
        `SELECT *
        FROM \`lines\`
        WHERE line_cd
        IN (SELECT line_cd FROM stations WHERE station_cd = ? AND e_status = 0)
        AND e_status = 0
        LIMIT 1`,
        [stationId],
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
}
