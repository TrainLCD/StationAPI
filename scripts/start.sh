#!/bin/bash
if [ -e "./tmp.sql" ]; then
    sh ./scripts/migration.sh
fi
npm run start:prod