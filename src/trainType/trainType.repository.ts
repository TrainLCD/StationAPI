import { Injectable } from '@nestjs/common';
import { RowDataPacket } from 'mysql2';
import { StationRaw } from 'src/models/stationRaw';
import { MysqlService } from 'src/mysql/mysql.service';
import { LineRepository } from '../line/line.repository';
import { LineRaw } from '../line/models/LineRaw';
import { TrainTypeRaw, TrainTypeWithLineRaw } from './models/TrainTypeRaw';

@Injectable()
export class TrainTypeRepository {
  constructor(
    private readonly mysqlService: MysqlService,
    private lineRepo: LineRepository,
  ) {}

  async getStationsByGroupIds(groupIds: number[]): Promise<StationRaw[]> {
    const { connection } = this.mysqlService;

    return new Promise<StationRaw[]>((resolve, reject) => {
      connection.query(
        `
          SELECT *
          FROM stations
          WHERE station_g_cd in (?)
          AND e_status = 0
        `,
        [groupIds],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([] as StationRaw[]);
          }

          return resolve(
            Promise.all(
              results.map(async (r) => ({
                ...(r as StationRaw),
                currentLine: await this.lineRepo.findOneStationId(r.station_cd),
                lines: await this.lineRepo.getByGroupId(r.station_g_cd),
              })),
            ),
          );
        },
      );
    });
  }

  async getByIds(lineGroupIds: number[]): Promise<TrainTypeRaw[]> {
    const { connection } = this.mysqlService;

    return new Promise<TrainTypeRaw[]>((resolve, reject) => {
      connection.query(
        `SELECT *
        FROM types as t, station_station_types as sst
        WHERE sst.line_group_cd in (?)
          AND t.type_cd = sst.type_cd
        LIMIT 1`,
        [lineGroupIds],
        (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }
          return resolve(results as TrainTypeRaw[]);
        },
      );
    });
  }

  async getBelongingStations(lineGroupIds: number[]): Promise<StationRaw[]> {
    const { connection } = this.mysqlService;

    return new Promise<StationRaw[]>((resolve, reject) => {
      connection.query(
        `
          SELECT *
          FROM station_station_types as sst, stations as s
          WHERE sst.line_group_cd in (?)
          AND s.station_cd = sst.station_cd
          AND s.e_status = 0
          ORDER BY sst.id
        `,
        [lineGroupIds],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }

          const belongStations = await this.getStationsByGroupIds(
            results.map((r) => r.station_g_cd),
          );

          return resolve(
            Promise.all(
              results.map(
                async (r): Promise<StationRaw> => ({
                  ...(r as StationRaw),
                  currentLine: await this.lineRepo.findOneStationId(
                    r.station_cd,
                  ),
                  lines: (await this.lineRepo.getByGroupId(r.station_g_cd)).map(
                    (l) => ({
                      ...l,
                      transferStation: belongStations.find(
                        (bs) =>
                          bs.station_g_cd === r.station_g_cd &&
                          bs.line_cd === l.line_cd,
                      ),
                    }),
                  ),
                }),
              ),
            ),
          );
        },
      );
    });
  }

  async getBelongingLines(lineGroupIds: number[]): Promise<LineRaw[]> {
    const { connection } = this.mysqlService;

    return new Promise<LineRaw[]>((resolve, reject) => {
      connection.query(
        `SELECT DISTINCT l.*
        FROM \`lines\` as l, stations as s, station_station_types as sst
        WHERE sst.line_group_cd = ?
          AND s.station_cd = sst.station_cd
          AND l.line_cd = s.line_cd
          AND s.e_status = 0`,
        [lineGroupIds[0]],
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

  async getAllLinesTrainTypes(
    lineGroupIds: number[],
  ): Promise<TrainTypeWithLineRaw[]> {
    const { connection } = this.mysqlService;

    return new Promise<TrainTypeWithLineRaw[]>((resolve, reject) => {
      connection.query(
        `SELECT DISTINCT t.*, l.*, c.company_name, c.company_name_en, sst.line_group_cd
        FROM \`lines\` as l,
        \`types\` as t,
        stations as s,
        station_station_types as sst,
        companies as c
        WHERE sst.line_group_cd in (?)
          AND s.station_cd = sst.station_cd
          AND sst.type_cd = t.type_cd
          AND s.e_status = 0
          AND l.line_cd = s.line_cd
          AND l.company_cd = c.company_cd`,
        [lineGroupIds],
        (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }
          return resolve(results as TrainTypeWithLineRaw[]);
        },
      );
    });
  }
}
