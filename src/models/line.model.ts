import { Field, Float, Int, ObjectType } from '@nestjs/graphql';
import { Company } from './company.model';
import { LineSymbol } from './lineSymbol.model';
import { Station } from './station.model';

@ObjectType()
export class Line {
  @Field((type) => Int)
  id: number;
  @Field((type) => Int)
  companyId: number;
  @Field((type) => Float)
  latitude: number;
  @Field((type) => Float)
  longitude: number;
  @Field()
  lineColorC: string;
  @Field()
  lineColorT: string;
  @Field((type) => [LineSymbol])
  lineSymbols: LineSymbol[];
  @Field()
  name: string;
  @Field()
  nameH: string;
  @Field()
  nameK: string;
  @Field()
  nameR: string;
  @Field()
  nameZh: string;
  @Field()
  nameKo: string;
  @Field((type) => Int)
  lineType: number;
  @Field((type) => Int)
  zoom: number;
  @Field((type) => Company, { nullable: true })
  company: Company;
  @Field((type) => Station, { nullable: true })
  transferStation: Station;
}
