import { Test, TestingModule } from '@nestjs/testing';
import { TrainTypeService } from './trainType.service';

describe('LineService', () => {
  let service: TrainTypeService;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [TrainTypeService],
    }).compile();

    service = module.get<TrainTypeService>(TrainTypeService);
  });

  it('should be defined', () => {
    expect(service).toBeDefined();
  });
});
