.PHONY: server client check test clean build-wasm help

## Lance le serveur de jeu (Axum + Tokio, port 3000)
server:
	cargo run -p server

## Lance le client WASM avec hot-reload (Trunk, port 8080)
client:
	cd crates/client && trunk serve

## Vérifie la compilation de shared + simulation + server (natif)
check:
	cargo check -p shared -p simulation -p server

## Vérifie la compilation du client (cible WASM)
check-client:
	cargo check -p client --target wasm32-unknown-unknown

## Vérifie tout
check-all: check check-client

## Lance les tests unitaires du protocole, de la simulation et du serveur
test:
	cargo test -p shared -p simulation -p server

## Build WASM release (artefacts dans crates/client/dist/)
build-wasm:
	cd crates/client && trunk build --release

## Nettoie les artefacts Cargo et Trunk
clean:
	cargo clean
	rm -rf crates/client/dist crates/client/.trunk

## Affiche cette aide
help:
	@grep -E '^##' Makefile | sed 's/## //'
