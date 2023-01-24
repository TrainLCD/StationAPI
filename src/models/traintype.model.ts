import { Field, ID, Int, ObjectType, registerEnumType } from '@nestjs/graphql';
import { Line } from './line.model';
import { Station } from './station.model';
import { TrainDirection } from './trainDirection.model';

registerEnumType(TrainDirection, {
  name: 'TrainDirection',
});

@ObjectType()
export class TrainTypeMinimum {
  @Field((type) => ID)
  id: number;
  @Field((type) => Int)
  typeId: number;
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
  @Field()
  color: string;
  @Field((type) => Line)
  line: Line;
}

@ObjectType()
export class TrainType {
  @Field((type) => ID)
  id: number;
  @Field((type) => Int)
  typeId: number;
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
  @Field()
  color: string;
  @Field(() => [Station])
  stations: Station[];
  @Field(() => [Line])
  lines: Line[];
  @Field(() => [TrainTypeMinimum])
  allTrainTypes: TrainTypeMinimum[];
  @Field(() => TrainDirection)
  direction: TrainDirection;
}
