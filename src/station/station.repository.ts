import { Injectable } from '@nestjs/common';
import { uniqBy } from 'lodash';
import { RowDataPacket } from 'mysql2';
import { NEX_ID } from 'src/constants/ignore';
import { TrainType } from 'src/graphql';
import { LineRepository } from 'src/line/line.repository';
import { MysqlService } from 'src/mysql/mysql.service';
import { TrainTypeWithLineRaw } from 'src/trainType/models/TrainTypeRaw';
import { TrainTypeRepository } from 'src/trainType/trainType.repository';
import { isJRLine } from 'src/utils/jr';
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
            t.type_name_zh,
            t.type_name_ko,
            t.color,
            t.direction,
            l.line_cd
          FROM station_station_types as sst,
            types as t,
            stations as s,
            \`lines\` as l
          WHERE sst.station_cd = ?
            AND sst.type_cd = t.type_cd
            AND sst.station_cd = s.station_cd
            AND l.line_cd = s.line_cd
            AND sst.pass IN (0, 2, 3, 4)
        `,
        [id],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }

          const parenthesisRegexp = /\([^()]*\)/g;

          const getReplacedTrainTypeName = async (
            r: RowDataPacket,
          ): Promise<{
            typesName: string;
            typesNameR: string;
            filteredAllTrainTypes: TrainTypeWithLineRaw[];
            allTrainTypes: TrainTypeWithLineRaw[];
          }> => {
            const allTrainTypes = await this.trainTypeRepo.getAllLinesTrainTypes(
              r.line_group_cd,
            );

            const filteredAllTrainTypes = allTrainTypes.filter(
              (tt) => tt.line_cd !== r.line_cd,
            );

            const typeCds = filteredAllTrainTypes.map((tt) => tt.type_cd);
            const isEveryTrainTypeSame = typeCds.includes(r.type_cd);

            const isAllLinesOperatedSameCompany = filteredAllTrainTypes.every(
              (tt, idx, arr) => tt.company_cd === arr[idx + 1]?.company_cd,
            );
            // JR東日本線直通と出さないようにしたいため(湘南新宿ラインと上野東京ラインは違う)
            const isAllLinesOperatedJR = filteredAllTrainTypes.every((tt) =>
              isJRLine(tt.company_cd),
            );

            const getIsPrevStationOperatedSameCompany = (
              arr: TrainTypeWithLineRaw[],
              trainType: TrainTypeWithLineRaw,
              index: number,
            ) => arr[index - 1]?.company_cd === trainType.company_cd;
            const getIsNextStationOperatedSameCompany = (
              arr: TrainTypeWithLineRaw[],
              trainType: TrainTypeWithLineRaw,
              index: number,
            ) => arr[index + 1]?.company_cd === trainType.company_cd;

            // 上り下り共用種別
            if (r.direction == 0) {
              const typesName = (() => {
                if (isEveryTrainTypeSame && !isAllLinesOperatedJR) {
                  return `${r.type_name}(${filteredAllTrainTypes
                    .map((tt, idx, arr) => {
                      const isPrevStationOperatedSameCompany = getIsPrevStationOperatedSameCompany(
                        arr,
                        tt,
                        idx,
                      );
                      const isNextStationOperatedSameCompany = getIsNextStationOperatedSameCompany(
                        arr,
                        tt,
                        idx,
                      );
                      if (isPrevStationOperatedSameCompany) {
                        return null;
                      }
                      if (isNextStationOperatedSameCompany) {
                        return `${tt.company_name}線`;
                      }

                      return `${tt.line_name.replace(parenthesisRegexp, '')}`;
                    })
                    .filter((tt) => !!tt)
                    .join('/')}直通)`;
                }

                return `${r.type_name}(${filteredAllTrainTypes
                  .map((tt, idx, arr) => {
                    const isPrevStationOperatedSameCompany = getIsPrevStationOperatedSameCompany(
                      arr,
                      tt,
                      idx,
                    );
                    const isNextStationOperatedSameCompany = getIsNextStationOperatedSameCompany(
                      arr,
                      tt,
                      idx,
                    );
                    if (isPrevStationOperatedSameCompany) {
                      return null;
                    }
                    if (
                      isNextStationOperatedSameCompany &&
                      !isAllLinesOperatedSameCompany &&
                      !isAllLinesOperatedJR
                    ) {
                      return `${tt.company_name}線${tt.type_name.replace(
                        parenthesisRegexp,
                        '',
                      )}`;
                    }

                    return `${tt.line_name.replace(
                      parenthesisRegexp,
                      '',
                    )}${tt.type_name.replace(parenthesisRegexp, '')}`;
                  })
                  .filter((tt) => !!tt)
                  .join('/')})`;
              })();
              const typesNameR = (() => {
                if (isEveryTrainTypeSame && !isAllLinesOperatedJR) {
                  return `${r.type_name_r}(${filteredAllTrainTypes
                    .map((tt, idx, arr) => {
                      const isPrevStationOperatedSameCompany = getIsPrevStationOperatedSameCompany(
                        arr,
                        tt,
                        idx,
                      );
                      const isNextStationOperatedSameCompany = getIsNextStationOperatedSameCompany(
                        arr,
                        tt,
                        idx,
                      );
                      if (isPrevStationOperatedSameCompany) {
                        return null;
                      }
                      if (isNextStationOperatedSameCompany) {
                        return `${tt.company_name_en} Line`;
                      }

                      return `${tt.line_name.replace(parenthesisRegexp, '')}`;
                    })
                    .filter((tt) => !!tt)
                    .join('/')})`;
                }

                return `${r.type_name_r}(${filteredAllTrainTypes
                  .map((tt, idx, arr) => {
                    const isPrevStationOperatedSameCompany = getIsPrevStationOperatedSameCompany(
                      arr,
                      tt,
                      idx,
                    );
                    const isNextStationOperatedSameCompany = getIsNextStationOperatedSameCompany(
                      arr,
                      tt,
                      idx,
                    );
                    if (isPrevStationOperatedSameCompany) {
                      return null;
                    }
                    if (
                      isNextStationOperatedSameCompany &&
                      !isAllLinesOperatedSameCompany &&
                      !isAllLinesOperatedJR
                    ) {
                      return `${
                        tt.company_name_en
                      } Line ${tt.type_name_r.replace(parenthesisRegexp, '')}`;
                    }

                    return `${tt.line_name_r.replace(
                      parenthesisRegexp,
                      '',
                    )}${tt.type_name_r.replace(parenthesisRegexp, '')}`;
                  })
                  .filter((tt) => !!tt)
                  .join('/')})`;
              })();
              return {
                typesName,
                typesNameR,
                filteredAllTrainTypes,
                allTrainTypes,
              };
            }

            const typesName = (() => {
              switch (r.direction) {
                case 1:
                  return `${r.type_name}(上り)`;
                case 2:
                  return `${r.type_name}(下り)`;
                default:
                  return r.type_name;
              }
            })();

            const typesNameR = (() => {
              switch (r.direction) {
                case 1:
                  return `${r.type_name_r}(Inbound)`;
                case 2:
                  return `${r.type_name_r}(Outbound)`;

                default:
                  return r.type_name_r;
              }
            })();

            return {
              typesName,
              typesNameR,
              filteredAllTrainTypes,
              allTrainTypes,
            };
          };

          return resolve(
            Promise.all(
              results.map(
                async (r): Promise<TrainType> => {
                  const {
                    typesName,
                    typesNameR,
                    filteredAllTrainTypes,
                    allTrainTypes,
                  } = await getReplacedTrainTypeName(r);

                  return {
                    // キャッシュが重複しないようにするため。もっとうまい方法あると思う
                    id: r.id + r.type_cd + r.line_group_cd,
                    typeId: r.type_cd,
                    groupId: r.line_group_cd,
                    name: !filteredAllTrainTypes.length
                      ? r.type_name
                      : typesName,
                    nameK: r.type_name_k,
                    nameR: !filteredAllTrainTypes.length
                      ? r.type_name_r
                      : typesNameR,
                    nameZh: r.type_name_zh,
                    nameKo: r.type_name_ko,
                    color: r.color,
                    allTrainTypes: allTrainTypes.map((tt) => ({
                      id: tt.type_cd + tt.line_cd,
                      typeId: tt.type_cd,
                      groupId: r.line_group_cd,
                      name: tt.type_name,
                      nameK: tt.type_name_k,
                      nameR: tt.type_name_r,
                      nameZh: tt.type_name_zh,
                      nameKo: tt.type_name_ko,
                      color: tt.color,
                      line: {
                        id: tt.line_cd,
                        companyId: tt.company_cd,
                        latitude: tt.lat,
                        longitude: tt.lon,
                        lineColorC: tt.line_color_c,
                        lineColorT: tt.line_color_t,
                        name: tt.line_name,
                        nameH: tt.line_name_h,
                        nameK: tt.line_name_k,
                        nameR: tt.line_name_r,
                        nameZh: tt.line_name_zh,
                        nameKo: tt.line_name_ko,
                        lineType: tt.line_type,
                        zoom: tt.zoom,
                      },
                    })),
                    lines: await this.trainTypeRepo.getBelongingLines(
                      r.line_group_cd,
                    ),
                    direction: r.direction,
                  };
                },
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

  /**
   * @deprecated Use `getByCoords` instead.
   */
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

  async getByCoords(
    latitude: number,
    longitude: number,
    limit?: number,
  ): Promise<StationRaw[]> {
    const { connection } = this.mysqlService;

    // if (limit > 10) {
    //   throw new Error('Invalid limit');
    // }

    return new Promise<StationRaw[]>((resolve, reject) => {
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
        stations as s1
        WHERE
        e_status = 0
        AND
        station_cd = (
          SELECT station_cd 
          FROM stations as s2
          WHERE s1.station_g_cd = s2.station_g_cd
          LIMIT 1
        )
        ORDER BY
        distance
        LIMIT ?
        `,
        [latitude, longitude, latitude, limit],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve(null);
          }

          return resolve(
            Promise.all(
              results.map(async (r) => ({
                ...(r as StationRaw),
                lines: await this.lineRepo.getByGroupId(r.station_g_cd),
              })),
            ),
          );
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
          OR station_name_k LIKE "%${name}%"
          OR station_name_zh LIKE "%${name}%"
          OR station_name_ko LIKE "%${name}%")
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

  async getAll(): Promise<StationRaw[]> {
    const { connection } = this.mysqlService;

    return new Promise<StationRaw[]>((resolve, reject) => {
      connection.query(
        `
          SELECT *
          FROM stations
          WHERE e_status = 0
          AND NOT line_cd = ${NEX_ID}
          AND station_g_cd IN (
            SELECT DISTINCT station_g_cd
            FROM stations
          )
        `,
        [],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }
          const filtered = uniqBy(results, 'station_g_cd');
          return resolve(filtered as StationRaw[]);
        },
      );
    });
  }

  async getRandomly(): Promise<StationRaw> {
    const { connection } = this.mysqlService;

    return new Promise<StationRaw>((resolve, reject) => {
      connection.query(
        `
        SELECT *
        FROM stations
        WHERE e_status = 0
        AND NOT line_cd = ${NEX_ID}
        AND station_g_cd IN (
          SELECT DISTINCT station_g_cd
          FROM stations
        )
        ORDER BY RAND()
        LIMIT 1
      `,
        [],
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
}
