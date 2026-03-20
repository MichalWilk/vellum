# Vellum

# -- Dev --

dev:
    docker compose up redis keycloak -d
    @echo ""
    @echo "Infra ready:"
    @echo "  just be    # Rust API on :3000"
    @echo "  just fe    # SvelteKit on :5173"
    @echo ""

be:
    cargo run --manifest-path src/backend/Cargo.toml

fe:
    npm --prefix src/frontend run dev

fe-install:
    npm --prefix src/frontend install

# -- Auth --

auth-oidc:
    @sed -i 's/^mode = "none"/mode = "oidc"/' config.local.toml
    @echo "Auth: oidc. Restart be. Login: user / user"

auth-none:
    @sed -i 's/^mode = "oidc"/mode = "none"/' config.local.toml
    @echo "Auth: none. Restart be."

# -- Check --

check:
    cargo check --manifest-path src/backend/Cargo.toml

check-fe:
    npm --prefix src/frontend run check

check-all: check check-fe

# -- Build --

build:
    cargo build --manifest-path src/backend/Cargo.toml --release

build-be:
    docker build -f deploy/Dockerfile.be .

build-fe:
    docker build -f deploy/Dockerfile.fe .

# -- Production --

up:
    docker compose --profile app up -d

down:
    docker compose --profile app down

logs:
    docker compose logs -f

# -- Clean --

clean:
    cargo clean --manifest-path src/backend/Cargo.toml
