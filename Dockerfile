FROM elixir:1.8.2-otp-22-alpine

RUN mkdir /app
WORKDIR /app

COPY . /app

RUN yes | mix local.hex
RUN yes | mix archive.install https://github.com/phoenixframework/archives/raw/master/phx_new.ez
RUN mix deps.get