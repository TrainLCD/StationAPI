FROM node:16.13.2-alpine

RUN apk add alpine-sdk python3 mysql-client

RUN mkdir /app
WORKDIR /app

COPY . /app
COPY ./scripts/migration.sh /app
COPY ./scripts/start.prod.sh /app

RUN npm install
RUN npm run build

EXPOSE 3000

CMD ["sh", "./scripts/start.prod.sh"]
