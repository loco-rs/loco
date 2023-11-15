PG_DB=postgres://localhost:5432/blo_app
sea-orm-cli generate entity --database-url $PG_DB --output-dir src/models/_entities
