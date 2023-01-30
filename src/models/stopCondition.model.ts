import { registerEnumType } from '@nestjs/graphql';

export enum StopCondition {
  ALL = 'ALL',
  NOT = 'NOT',
  PARTIAL = 'PARTIAL',
  WEEKDAY = 'WEEKDAY',
  HOLIDAY = 'HOLIDAY',
  PARTIAL_STOP = 'PARTIAL_STOP',
}

registerEnumType(StopCondition, {
  name: 'StopCondition',
});
