import { Injectable } from '@nestjs/common';
import { uniqBy } from 'lodash';
import { RowDataPacket } from 'mysql2';
import { NEX_ID } from 'src/constants/ignore';
import { TrainType } from 'src/graphql';
import { LineRepository } from 'src/line/line.repository';
import { MysqlService } from 'src/mysql/mysql.service';
import { RawService } from 'src/raw/raw.service';
import { TrainTypeWithLineRaw } from 'src/trainType/models/TrainTypeRaw';
import { TrainTypeRepository } from 'src/trainType/trainType.repository';
import { StationRaw } from './models/StationRaw';

@Injectable()
export class StationRepository {
  constructor(
    private readonly mysqlService: MysqlService,
    private readonly rawService: RawService,
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
            const isEveryTrainTypeSame = typeCds.every((typeCd, idx, arr) =>
              arr[idx + 1] ? arr[idx + 1] === typeCd : true,
            );

            const isAllLinesOperatedSameCompany = filteredAllTrainTypes.every(
              (tt, idx, arr) => tt.company_cd === arr[idx + 1]?.company_cd,
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
                if (isEveryTrainTypeSame) {
                  return `${r.type_name}(${filteredAllTrainTypes
                    .map(
                      (tt) => `${tt.line_name.replace(parenthesisRegexp, '')}`,
                    )
                    .filter((tt) => !!tt)
                    .filter((tt, idx, arr) =>
                      arr[idx + 1] ? tt !== arr[idx + 1] : true,
                    )
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
                      !isAllLinesOperatedSameCompany
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
                if (isEveryTrainTypeSame) {
                  return `${r.type_name_r}(${filteredAllTrainTypes
                    .map(
                      (tt) =>
                        `${tt.line_name_r.replace(parenthesisRegexp, '')}`,
                    )
                    .filter((tt) => !!tt)
                    .filter((tt, idx, arr) =>
                      arr[idx + 1] ? tt !== arr[idx + 1] : true,
                    )
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
                      !isAllLinesOperatedSameCompany
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

  async getByIds(ids: number[]): Promise<StationRaw[]> {
    const { connection } = this.mysqlService;

    return new Promise<StationRaw[]>((resolve, reject) => {
      connection.query(
        `
          SELECT *
          FROM stations
          WHERE station_cd in (?)
          AND e_status = 0
          AND NOT line_cd = ${NEX_ID}
        `,
        [ids],
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

  async getTrainTypesByIds(ids: number[]): Promise<TrainType[][]> {
    const { connection } = this.mysqlService;

    return new Promise<TrainType[][]>((resolve, reject) => {
      connection.query(
        `
        SELECT sst.type_cd,
            sst.id,
            sst.line_group_cd,
            sst.station_cd,
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
          WHERE sst.station_cd in (?)
            AND sst.type_cd = t.type_cd
            AND sst.station_cd = s.station_cd
            AND l.line_cd = s.line_cd
            AND sst.pass IN (0, 2, 3, 4)
        `,
        [ids],
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
            const isEveryTrainTypeSame = typeCds.every((typeCd, idx, arr) =>
              arr[idx + 1] ? arr[idx + 1] === typeCd : true,
            );

            const isAllLinesOperatedSameCompany = filteredAllTrainTypes.every(
              (tt, idx, arr) => tt.company_cd === arr[idx + 1]?.company_cd,
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
                if (isEveryTrainTypeSame) {
                  return `${r.type_name}(${filteredAllTrainTypes
                    .map(
                      (tt) => `${tt.line_name.replace(parenthesisRegexp, '')}`,
                    )
                    .filter((tt) => !!tt)
                    .filter((tt, idx, arr) =>
                      arr[idx + 1] ? tt !== arr[idx + 1] : true,
                    )
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
                      !isAllLinesOperatedSameCompany
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
                if (isEveryTrainTypeSame) {
                  return `${r.type_name_r}(${filteredAllTrainTypes
                    .map(
                      (tt) =>
                        `${tt.line_name_r.replace(parenthesisRegexp, '')}`,
                    )
                    .filter((tt) => !!tt)
                    .filter((tt, idx, arr) =>
                      arr[idx + 1] ? tt !== arr[idx + 1] : true,
                    )
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
                      !isAllLinesOperatedSameCompany
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
              ids.map((id) =>
                Promise.all(
                  results
                    .filter((r) => r.station_cd === id)
                    .map(
                      async (r): Promise<TrainType> => {
                        const {
                          typesName,
                          typesNameR,
                          filteredAllTrainTypes,
                          allTrainTypes,
                        } = await getReplacedTrainTypeName(r);

                        const lineGroupIds = allTrainTypes.map(
                          (tt) => tt.line_group_cd,
                        );
                        const linesRaw = await this.trainTypeRepo.getBelongingLines(
                          lineGroupIds,
                        );
                        const companies = await this.lineRepo.getCompaniesByLineIds(
                          linesRaw.map((l) => l.line_cd),
                        );

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
                          lines: linesRaw.map((l, i) =>
                            this.rawService.convertLine(l, companies[i]),
                          ),
                          direction: r.direction,
                        };
                      },
                    ),
                ),
              ),
            ),
          );
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
                currentLine: await this.lineRepo.findOneStationId(r.station_cd),
                lines: await this.lineRepo.getByGroupId(r.station_g_cd),
              })),
            ),
          );
        },
      );
    });
  }

  async getByLineIds(lineIds: number[]): Promise<StationRaw[]> {
    const { connection } = this.mysqlService;
    if (!connection) {
      return [];
    }

    return new Promise<StationRaw[]>((resolve, reject) => {
      connection.query(
        `
          SELECT *
          FROM stations
          WHERE line_cd in (?)
          AND e_status = 0
          AND NOT line_cd = ${NEX_ID}
          ORDER BY e_sort, station_cd
        `,
        [lineIds],
        (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
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

  async getByName(name: string): Promise<StationRaw[]> {
    const { connection } = this.mysqlService;
    if (!connection) {
      return [];
    }

    return new Promise<StationRaw[]>((resolve, reject) => {
      connection.query(
        `
          SELECT * FROM stations
          WHERE (station_name LIKE "%"?"%"
          OR station_name_r LIKE "%"?"%"
          OR station_name_k LIKE "%"?"%"
          OR station_name_zh LIKE "%"?"%"
          OR station_name_ko LIKE "%"?"%")
          AND e_status = 0
          AND NOT line_cd = ${NEX_ID}
          ORDER BY e_sort, station_cd
        `,
        [name, name, name, name, name],
        async (err, results: RowDataPacket[]) => {
          if (err) {
            return reject(err);
          }
          if (!results.length) {
            return resolve([]);
          }

          const lineIds = results.map((r) => r.line_cd);
          const lines = await this.lineRepo.getByIds(lineIds);

          return resolve(
            results.map(
              (r) =>
                ({
                  ...r,
                  lines: lines.filter((l) => l.line_cd === r.line_cd),
                } as StationRaw),
            ),
          );
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

          return resolve({
            ...results[0],
            currentLine: await this.lineRepo.findOneStationId(
              results[0]?.station_cd,
            ),
            lines: await this.lineRepo.getByGroupId(results[0].station_g_cd),
          } as StationRaw);
        },
      );
    });
  }
}
