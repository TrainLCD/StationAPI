
/** ------------------------------------------------------
 * THIS FILE WAS AUTOMATICALLY GENERATED (DO NOT MODIFY)
 * -------------------------------------------------------
 */

/* tslint:disable */
/* eslint-disable */
export abstract class IQuery {
    abstract line(id: string): Line | Promise<Line>;

    abstract station(id: string): Station | Promise<Station>;

    abstract stationByGroupId(groupId: string): Station | Promise<Station>;

    abstract stationByCoords(latitude: number, longitude: number): Station | Promise<Station>;

    abstract stationsByLineId(lineId: string): Station[] | Promise<Station[]>;

    abstract stationsByName(name: string): Station[] | Promise<Station[]>;

    abstract trainType(id: string, excludePass?: boolean): TrainType | Promise<TrainType>;
}

export class Line {
    id?: number;
    companyId?: number;
    latitude?: number;
    longitude?: number;
    lineColorC?: string;
    lineColorT?: string;
    name?: string;
    nameH?: string;
    nameK?: string;
    nameR?: string;
    lineType?: number;
    zoom?: number;
}

export class Station {
    id?: number;
    address?: string;
    distance?: number;
    latitude?: number;
    longitude?: number;
    lines?: Line[];
    openYmd?: string;
    postalCode?: string;
    prefId?: number;
    groupId?: number;
    name?: string;
    nameK?: string;
    nameR?: string;
    trainTypes?: TrainType[];
    pass?: boolean;
}

export class TrainType {
    id?: number;
    groupId?: number;
    name?: string;
    nameK?: string;
    nameR?: string;
    stations?: Station[];
}
