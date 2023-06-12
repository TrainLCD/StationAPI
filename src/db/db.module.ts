import { Module } from '@nestjs/common';
// eslint-disable-next-line @typescript-eslint/no-var-requires
const mysql = require('mysql2');

export const DB_CONNECTION = 'DB_CONNECTION';

const dbProvider = {
  provide: DB_CONNECTION,
  useValue: mysql.createConnection(process.env.DATABASE_URL),
};

@Module({
  providers: [dbProvider],
  exports: [dbProvider],
})
export class DbModule {}
