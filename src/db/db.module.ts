import { Module } from '@nestjs/common';
// eslint-disable-next-line @typescript-eslint/no-var-requires
const mysql = require('mysql2');

export const DB_CONNECTION = 'DB_CONNECTION';

const dbProviderGCP = {
  provide: DB_CONNECTION,
  useValue: mysql.createPool({
    user: process.env.MYSQL_USER,
    password: process.env.MYSQL_PASSWORD,
    database: process.env.MYSQL_DATABASE,
    socketPath: `${process.env.MYSQL_SOCKET}/${process.env.INSTANCE_CONNECTION_NAME}`,
  }),
};

const dbProviderLocal = {
  provide: DB_CONNECTION,
  useValue: mysql.createPool({
    connectionLimit: 10,
    host: process.env.MYSQL_HOST,
    user: process.env.MYSQL_USER,
    password: process.env.MYSQL_PASSWORD,
    database: process.env.MYSQL_DATABASE,
  }),
};

@Module({
  providers: [process.env.IS_GCP ? dbProviderGCP : dbProviderLocal],
  exports: [process.env.IS_GCP ? dbProviderGCP : dbProviderLocal],
})
export class DbModule {}
