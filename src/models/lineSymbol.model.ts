import { Field, ObjectType } from '@nestjs/graphql';

@ObjectType()
export class LineSymbol {
  @Field()
  lineSymbol: string;
  @Field()
  lineSymbolColor: string;
}
