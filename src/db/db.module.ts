import { Module } from '@nestjs/common';
// eslint-disable-next-line @typescript-eslint/no-var-requires
const mysql = require('mysql2');

export const DB_CONNECTION = 'DB_CONNECTION';

const dbProvider = {
  provide: DB_CONNECTION,
  useValue: mysql.createPool({
    connectionLimit: 10,
    socketPath: process.env.MYSQL_SOCKET,
    user: process.env.MYSQL_USER,
    password: process.env.MYSQL_PASSWORD,
    database: process.env.MYSQL_DATABASE,
  }),
};

@Module({
  providers: [dbProvider],
  exports: [dbProvider],
})
export class DbModule {}
