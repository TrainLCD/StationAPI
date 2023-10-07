#!/bin/bash
mysql -u$MYSQL_USER -p$MYSQL_PASSWORD --default-character-set=utf8 -S$MYSQL_SOCKET $MYSQL_DATABASE < ./tmp.sql
/usr/local/bin/stationapi
