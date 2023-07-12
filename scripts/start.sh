#!/bin/bash
if [ -e "./tmp.sql" ]; then
    sh ./scripts/migration.sh
fi
/usr/local/bin/stationapi
