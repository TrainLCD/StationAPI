import { Field, Float, Int, ObjectType } from '@nestjs/graphql';
import { Line } from './line.model';
import { StationNumber } from './stationNumber.model';
import { StopCondition } from './stopCondition.model';
import { TrainType } from './traintype.model';

@ObjectType()
export class Station {
  @Field((type) => Int)
  id: number;
  @Field()
  address: string;
  @Field((type) => Float, { nullable: true })
  distance: number;
  @Field((type) => Float)
  latitude: number;
  @Field((type) => Float)
  longitude: number;
  @Field((type) => [Line])
  lines: Line[];
  @Field((type) => Line)
  currentLine: Line;
  @Field()
  openYmd: string;
  @Field()
  postalCode: string;
  @Field((type) => Int)
  prefId: number;
  @Field((type) => Int)
  groupId: number;
  @Field()
  name: string;
  @Field()
  nameK: string;
  @Field()
  nameR: string;
  @Field()
  nameZh: string;
  @Field()
  nameKo: string;
  @Field((type) => [TrainType])
  trainTypes: TrainType[];
  @Field()
  pass: boolean;
  @Field((type) => StopCondition)
  stopCondition: StopCondition;
  @Field((type) => [StationNumber])
  stationNumbers: StationNumber[];
  @Field()
  threeLetterCode: string;
}
