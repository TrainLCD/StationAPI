import { Test, TestingModule } from '@nestjs/testing';
import { TrainTypeResolver } from './trainType.resolver';

describe('TrainTypeResolver', () => {
  let resolver: TrainTypeResolver;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [TrainTypeResolver],
    }).compile();

    resolver = module.get<TrainTypeResolver>(TrainTypeResolver);
  });

  it('should be defined', () => {
    expect(resolver).toBeDefined();
  });
});
