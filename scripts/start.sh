#!/bin/bash
if [ -e "./tmp.sql" ]; then
    echo "Migration file exists"
    sh ./scripts/migration.sh
fi
/usr/local/bin/stationapi