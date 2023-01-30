import { registerEnumType } from '@nestjs/graphql';

export enum TrainDirection {
  BOTH = 'BOTH',
  INBOUND = 'INBOUND',
  OUTBOUND = 'OUTBOUND',
}

registerEnumType(TrainDirection, { name: 'TrainDirection' });
