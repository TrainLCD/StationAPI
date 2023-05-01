# syntax=docker/dockerfile:1
FROM node:18 AS builder
WORKDIR /app
COPY . /app
RUN npm install
RUN npm run build
RUN cd ./scripts && npm install
RUN node ./scripts/sqlgen.js

FROM mysql:8 AS migration
WORKDIR /app
COPY --from=builder /app/scripts/migration.sh .
COPY --from=builder /app/migrations/create_table.sql ./migrations
COPY --from=builder /app/tmp.sql .
RUN sh ./migration.sh

FROM gcr.io/distroless/nodejs18:latest
WORKDIR /app
COPY --from=builder /app .
EXPOSE 3000
CMD ["./dist/main.js"]
