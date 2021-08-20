# Hack as a Service API

## Running locally

1. Install [Docker](https://docker.com) and Docker Compose.
2. Run `docker-compose up -d` to start up your development instance.
3. Run `docker-compose exec main diesel migration run` to update the database (you only need to run this once, unless you modify the database schema)
4. Visit http://localhost:3000

Other commands:

- `docker-compose logs -f main` streams logs from the Rocket app
- `docker-compose exec db psql -U postgres` opens a shell into the PostgreSQL database
- `docker-compose stop` stops the dev environment
