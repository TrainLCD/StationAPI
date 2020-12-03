import { Test, TestingModule } from '@nestjs/testing';
import { RawService } from './raw.service';

describe('RawService', () => {
  let service: RawService;

  beforeEach(async () => {
    const module: TestingModule = await Test.createTestingModule({
      providers: [RawService],
    }).compile();

    service = module.get<RawService>(RawService);
  });

  it('should be defined', () => {
    expect(service).toBeDefined();
  });
});
