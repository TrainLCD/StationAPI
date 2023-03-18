import { Field, ObjectType } from '@nestjs/graphql';

@ObjectType()
export class StationNumber {
  @Field()
  lineSymbol: string;
  @Field()
  lineSymbolColor: string;
  @Field()
  lineSymbolShape: string;
  @Field()
  stationNumber: string;
}
