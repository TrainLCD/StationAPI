import { Injectable } from '@nestjs/common';
import { Connection } from 'mysql2';
// eslint-disable-next-line @typescript-eslint/no-var-requires
const mysql = require('mysql2');

@Injectable()
export class MysqlService {
  private conn: Connection;

  constructor() {
    this.conn = mysql.createPool({
      connectionLimit: 10,
      host: process.env.MYSQL_HOST,
      user: process.env.MYSQL_USER,
      password: process.env.MYSQL_PASSWORD,
      database: process.env.MYSQL_DATABASE,
    });
  }

  get connection(): Connection | undefined {
    return this.conn;
  }

  rawDataPacketToJSON(data: unknown) {
    return JSON.parse(JSON.stringify(data));
  }
}
