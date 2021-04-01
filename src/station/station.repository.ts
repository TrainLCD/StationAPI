import { Injectable } from '@nestjs/common';
import { RowDataPacket } from 'mysql2';
import { NEX_ID } from 'src/constants/ignore';
import { TrainType } from 'src/graphql';
import { LineRepository } from 'src/line/line.repository';
import { MysqlService } from 'src/mysql/mysql.service';
import { TrainTypeRepository } from 'src/trainType/trainType.repository';
import { StationRaw } from './models/StationRaw';

@Injectable()
export class StationRepository {
  constructor(
    private readonly mysqlService: MysqlService,
    private readonly lineRepo: LineRepository,
    private readonly trainTypeRepo: TrainTypeRepository,
  ) {}

  async findOneById(id: number): Promise<StationRaw> {
    const { connection } = this.mysqlService;

    return new Promise<StationRaw>((resolve, reject) => {
      connection.query(
        `
          SELECT *
          FROM stations
          WHERE station_cd = ?
          AND e_status = 0
          AND NOT line_cd = ${NEX_ID}
        `,
        [id],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }

          const lines = await this.lineRepo.getByGroupId(
            results[0].station_g_cd,
          );
          return resolve({
            ...(results[0] as StationRaw),
            lines,
          });
        },
      );
    });
  }

  async findTrainTypesById(id: number): Promise<TrainType[]> {
    const { connection } = this.mysqlService;

    return new Promise<TrainType[]>((resolve, reject) => {
      connection.query(
        `
          SELECT sst.type_cd,
            sst.id,
            sst.line_group_cd,
            t.type_name,
            t.type_name_k,
            t.type_name_r,
            t.color
          FROM station_station_types as sst, types as t
          WHERE sst.station_cd = ?
            AND sst.type_cd = t.type_cd
            AND sst.pass != 1
        `,
        [id],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }
          return resolve(
            Promise.all(
              await results.map(
                async (r): Promise<TrainType> => ({
                  id: r.type_cd,
                  groupId: r.line_group_cd,
                  name: r.type_name,
                  nameK: r.type_name_k,
                  nameR: r.type_name_r,
                  color: r.color,
                  lines: await this.trainTypeRepo.getBelongingLines(
                    r.line_group_cd,
                  ),
                }),
              ),
            ),
          );
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
          AND NOT line_cd = ${NEX_ID}
        `,
        [groupId],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }
          const lines = await this.lineRepo.getByGroupId(
            results[0].station_g_cd,
          );
          return resolve({
            ...(results[0] as StationRaw),
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
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }
          const lines = await this.lineRepo.getByGroupId(
            results[0].station_g_cd,
          );
          return resolve({
            ...(results[0] as StationRaw),
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
          AND NOT line_cd = ${NEX_ID}
          ORDER BY e_sort, station_cd
        `,
        [lineId],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }

          const map = await Promise.all<StationRaw>(
            results.map(async (r) => {
              const lines = await this.lineRepo.getByGroupId(r.station_g_cd);
              return {
                ...r,
                lines,
              } as StationRaw;
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
          AND NOT line_cd = ${NEX_ID}
          ORDER BY e_sort, station_cd
        `,
        [],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }

          const map = await Promise.all<StationRaw>(
            results.map(async (r) => {
              const line = await this.lineRepo.findOne(r.line_cd);
              return {
                ...r,
                lines: [line],
              } as StationRaw;
            }),
          );

          return resolve(map);
        },
      );
    });
  }
}
