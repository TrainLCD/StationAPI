FROM node:18-slim
RUN apt-get update && \
    apt-get install -y --quiet default-mysql-client && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . /app
RUN npm install
RUN npm run build
RUN cd ./scripts && npm install
RUN node ./scripts/sqlgen.js
ENV PORT 3000
EXPOSE $PORT
CMD ["sh", "./scripts/start.sh"]