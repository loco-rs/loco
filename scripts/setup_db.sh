#!/usr/bin/env bash
docker run -d -p 5432:5432 -e POSTGRES_USER=loco -e POSTGRES_DB=loco_app -e POSTGRES_PASSWORD="loco" postgres:15.3-alpine
psql --host=localhost --port=5432 --username=postgres --command="CREATE ROLE loco WITH LOGIN PASSWORD 'loco';"