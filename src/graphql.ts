
/** ------------------------------------------------------
 * THIS FILE WAS AUTOMATICALLY GENERATED (DO NOT MODIFY)
 * -------------------------------------------------------
 */

/* tslint:disable */
/* eslint-disable */
export enum StopCondition {
    ALL = "ALL",
    NOT = "NOT",
    PARTIAL = "PARTIAL",
    WEEKDAY = "WEEKDAY",
    HOLIDAY = "HOLIDAY",
    PARTIAL_STOP = "PARTIAL_STOP"
}

export enum TrainDirection {
    BOTH = "BOTH",
    INBOUND = "INBOUND",
    OUTBOUND = "OUTBOUND"
}

export abstract class IQuery {
    abstract line(id: string): Line | Promise<Line>;

    abstract station(id: string): Station | Promise<Station>;

    abstract stationByGroupId(groupId: string): Station | Promise<Station>;

    abstract nearbyStations(latitude: number, longitude: number, limit?: number): Station[] | Promise<Station[]>;

    abstract stationsByLineId(lineId: string): Station[] | Promise<Station[]>;

    abstract stationsByName(name: string): Station[] | Promise<Station[]>;

    abstract random(): Station | Promise<Station>;

    abstract trainType(id: string): TrainType | Promise<TrainType>;
}

export class LineSymbol {
    lineSymbol?: string;
    lineSymbolColor?: string;
}

export class Line {
    id?: number;
    companyId?: number;
    latitude?: number;
    longitude?: number;
    lineColorC?: string;
    lineColorT?: string;
    lineSymbols?: LineSymbol[];
    name?: string;
    nameH?: string;
    nameK?: string;
    nameR?: string;
    nameZh?: string;
    nameKo?: string;
    lineType?: number;
    zoom?: number;
    company?: Company;
}

export class Company {
    id?: number;
    railroadId?: number;
    name?: string;
    nameK?: string;
    nameH?: string;
    nameR?: string;
    nameEn?: string;
    url?: string;
    companyType?: number;
}

export class StationNumber {
    lineSymbol?: string;
    lineSymbolColor?: string;
    stationNumber?: string;
}

export class Station {
    id?: number;
    address?: string;
    distance?: number;
    latitude?: number;
    longitude?: number;
    lines?: Line[];
    currentLine?: Line;
    openYmd?: string;
    postalCode?: string;
    prefId?: number;
    groupId?: number;
    name?: string;
    nameK?: string;
    nameR?: string;
    nameZh?: string;
    nameKo?: string;
    trainTypes?: TrainType[];
    pass?: boolean;
    stopCondition?: StopCondition;
    stationNumbers?: StationNumber[];
    threeLetterCode?: string;
}

export class StationOnly {
    id?: number;
    address?: string;
    distance?: number;
    latitude?: number;
    longitude?: number;
    openYmd?: string;
    postalCode?: string;
    prefId?: number;
    groupId?: number;
    name?: string;
    nameK?: string;
    nameR?: string;
    nameZh?: string;
    nameKo?: string;
}

export class TrainTypeMinimum {
    id?: number;
    typeId?: number;
    groupId?: number;
    name?: string;
    nameK?: string;
    nameR?: string;
    nameZh?: string;
    nameKo?: string;
    color?: string;
    line?: Line;
}

export class TrainType {
    id?: number;
    typeId?: number;
    groupId?: number;
    name?: string;
    nameK?: string;
    nameR?: string;
    nameZh?: string;
    nameKo?: string;
    color?: string;
    stations?: Station[];
    lines?: Line[];
    allTrainTypes?: TrainTypeMinimum[];
    direction?: TrainDirection;
}
