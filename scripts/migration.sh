#!/bin/bash

mysql -f -u$MYSQL_USER -p$MYSQL_PASSWORD -h$MYSQL_HOST -e "CREATE DATABASE ${MYSQL_DATABASE} /*\!40100 DEFAULT CHARACTER SET utf8 */;" || true
mysql -u$MYSQL_USER -p$MYSQL_PASSWORD -h$MYSQL_HOST $MYSQL_DATABASE < ./migrations/create_table.sql
node ./scripts/sqlgen.js
mysql -u$MYSQL_USER -p$MYSQL_PASSWORD -h$MYSQL_HOST $MYSQL_DATABASE < ./tmp.sql