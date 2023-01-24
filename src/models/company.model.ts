import { Field, ID, Int, ObjectType } from '@nestjs/graphql';

@ObjectType()
export class Company {
  @Field((type) => ID)
  id: number;
  @Field((type) => Int)
  railroadId: number;
  @Field()
  name: string;
  @Field()
  nameK: string;
  @Field()
  nameH: string;
  @Field()
  nameR: string;
  @Field()
  nameEn: string;
  @Field()
  url: string;
  @Field((type) => Int)
  companyType: number;
}
