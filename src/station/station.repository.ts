import { Injectable } from '@nestjs/common';
import { MysqlService } from 'src/mysql/mysql.service';
import { LineRaw } from './models/LineRaw';
import { StationRaw } from './models/StationRaw';

@Injectable()
export class StationRepository {
  constructor(private readonly mysqlService: MysqlService) {}
  private readonly NEX_ID = 11328;

  async findOneById(id: number): Promise<StationRaw> {
    const { connection } = this.mysqlService;

    return new Promise<StationRaw>((resolve, reject) => {
      connection.query(
        `
          SELECT *
          FROM stations
          WHERE station_cd = ?
          AND e_status = 0
          AND NOT line_cd = ${this.NEX_ID}
        `,
        [id],
        async (err, results) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }

          const lines = await this.getLinesByGroupId(results[0].station_g_cd);
          return resolve({
            ...results[0],
            lines,
          });
        },
      );
    });
  }

  async findOneByGroupId(groupId: number): Promise<StationRaw> {
    const { connection } = this.mysqlService;

    return new Promise<StationRaw>((resolve, reject) => {
      connection.query(
        `
          SELECT *
          FROM stations
          WHERE station_g_cd = ?
          AND e_status = 0
          AND NOT line_cd = ${this.NEX_ID}
        `,
        [groupId],
        async (err, results) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }
          const lines = await this.getLinesByGroupId(results[0].station_g_cd);
          return resolve({
            ...results[0],
            lines,
          });
        },
      );
    });
  }

  async findOneByCoords(
    latitude: number,
    longitude: number,
  ): Promise<StationRaw> {
    const { connection } = this.mysqlService;

    return new Promise<StationRaw>((resolve, reject) => {
      connection.query(
        `
        SELECT *,
        (
          6371 * acos(
          cos(radians(?))
          * cos(radians(lat))
          * cos(radians(lon) - radians(?))
          + sin(radians(?))
          * sin(radians(lat))
          )
        ) AS distance
        FROM
        stations
        WHERE
        e_status = 0
        ORDER BY
        distance
        LIMIT 1
        `,
        [latitude, longitude, latitude],
        async (err, results) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }
          const lines = await this.getLinesByGroupId(results[0].station_g_cd);
          return resolve({
            ...results[0],
            lines,
          });
        },
      );
    });
  }

  async getByLineId(lineId: number): Promise<StationRaw[]> {
    const { connection } = this.mysqlService;
    if (!connection) {
      return [];
    }

    return new Promise<StationRaw[]>((resolve, reject) => {
      connection.query(
        `
          SELECT *
          FROM stations
          WHERE line_cd = ?
          AND e_status = 0
          AND NOT line_cd = ${this.NEX_ID}
          ORDER BY e_sort, station_cd
        `,
        [lineId],
        async (err, results) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }

          const map = await Promise.all<StationRaw>(
            results.map(async (r) => {
              const lines = await this.getLinesByGroupId(r.station_g_cd);
              return {
                ...r,
                lines,
              };
            }),
          );

          return resolve(map);
        },
      );
    });
  }

  async getByName(name: string): Promise<StationRaw[]> {
    const { connection } = this.mysqlService;
    if (!connection) {
      return [];
    }

    return new Promise<StationRaw[]>((resolve, reject) => {
      connection.query(
        `
          SELECT * FROM stations
          WHERE (station_name LIKE "%${name}%"
          OR station_name_r LIKE "%${name}%"
          OR station_name_k LIKE "%${name}%")
          AND e_status = 0
          AND NOT line_cd = ${this.NEX_ID}
          ORDER BY e_sort, station_cd
        `,
        [],
        async (err, results) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }

          const map = await Promise.all<StationRaw>(
            results.map(async (r) => {
              const line = await this.findOneLine(r.line_cd);
              return {
                ...r,
                lines: [line],
              };
            }),
          );

          return resolve(map);
        },
      );
    });
  }

  async findOneLine(id: number): Promise<LineRaw> {
    const { connection } = this.mysqlService;

    return new Promise<LineRaw>((resolve, reject) => {
      connection.query(
        `SELECT * FROM \`lines\` WHERE line_cd = ? AND NOT line_cd = ${this.NEX_ID}`,
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

  async getLinesByGroupId(groupId: number): Promise<LineRaw[]> {
    const { connection } = this.mysqlService;
    if (!connection) {
      return [];
    }

    return new Promise<LineRaw[]>((resolve, reject) => {
      connection.query(
        `SELECT * FROM \`lines\` WHERE line_cd IN (SELECT line_cd FROM stations WHERE station_g_cd = ?) AND NOT line_cd = ${this.NEX_ID}`,
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
