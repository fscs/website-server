FROM ubuntu:24.04

ARG DB_HOST
ARG DB_USER
ARG DB_PASS
ARG DB_NAME

RUN apt-get update && apt-get install -y ca-certificates tzdata
WORKDIR /app
COPY ./target/release/fscs-website-backend .

COPY ./target/release/public ./static


# ENTRYPOINT [ "./backend", "--host", "0.0.0.0", "--database-url", "postgres://${DB_USER}:${DB_PASS}@${DB_HOST}/${DB_NAME}" ]
