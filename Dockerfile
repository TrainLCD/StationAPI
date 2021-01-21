FROM node:14.15.1-alpine

RUN apk add alpine-sdk python

RUN mkdir /app
WORKDIR /app

COPY . /app

RUN npm install
RUN npm run build

EXPOSE 3000

CMD ["npm", "run", "start:prod"]
