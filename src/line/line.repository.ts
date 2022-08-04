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
        AND NOT line_cd = ?
        AND e_status = 0
        LIMIT 1`,
        [id, NEX_ID],
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
    const { connection } = this.mysqlService;

    return new Promise<LineRaw[]>((resolve, reject) => {
      connection.query(
        `SELECT *
        FROM \`lines\`
        WHERE line_cd in (?)
        AND NOT line_cd = ?
        AND e_status = 0`,
        [ids, NEX_ID],
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
    const { connection } = this.mysqlService;

    return new Promise<CompanyRaw>((resolve, reject) => {
      connection.query(
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
    const { connection } = this.mysqlService;

    return new Promise<CompanyRaw[]>((resolve, reject) => {
      connection.query(
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
    const { connection } = this.mysqlService;
    if (!connection) {
      return [];
    }

    return new Promise<LineRaw[]>((resolve, reject) => {
      connection.query(
        `SELECT *
        FROM \`lines\`
        WHERE line_cd
        IN (SELECT line_cd FROM stations WHERE station_g_cd = ?)
        AND NOT line_cd = ?
        AND e_status = 0`,
        [groupId, NEX_ID],
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
    const { connection } = this.mysqlService;
    if (!connection) {
      return null;
    }

    return new Promise<LineRaw>((resolve, reject) => {
      connection.query(
        `SELECT *
        FROM \`lines\`
        WHERE line_cd
        IN (SELECT line_cd FROM stations WHERE station_cd = ?)
        AND NOT line_cd = ?
        AND e_status = 0
        LIMIT 1`,
        [stationId, NEX_ID],
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

  async getBySrcAndDstGroupId(
    srcGID: number,
    dstGID: number,
  ): Promise<LineRaw[]> {
    const { connection } = this.mysqlService;
    if (!connection) {
      return [];
    }

    return new Promise<LineRaw[]>((resolve, reject) => {
      connection.query(
        `SELECT *
        FROM \`lines\`
        WHERE
          line_cd IN (
            SELECT line_cd
            FROM stations
            WHERE station_g_cd = ?
          )
          AND
          line_cd IN (
            SELECT line_cd
            FROM stations
            WHERE station_g_cd = ?
          )
          AND
          e_status = 0
          AND NOT
          line_cd = ?
        `,
        [srcGID, dstGID, NEX_ID],
        async (err, results: RowDataPacket[]) => {
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
