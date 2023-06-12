# syntax=docker/dockerfile:1
FROM node:18-alpine

RUN apk add alpine-sdk mysql-client

RUN mkdir /app
WORKDIR /app

COPY . /app

RUN npm install
RUN npm run build

RUN cd ./scripts && npm install
RUN node ./scripts/sqlgen.js
COPY ./tmp.sql .

EXPOSE 3000

CMD ["sh", "./scripts/start.sh"]

