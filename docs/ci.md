# CI et tests client WASM

Le workflow GitHub Actions `.github/workflows/ci.yml` s'exécute sur les
pull requests et les pushes vers `main`. Il utilise Rust 1.100.0 et Trunk
0.21.14, et toutes les commandes Cargo imposent le `Cargo.lock` versionné
avec `--locked`. Un lockfile à mettre à jour fait donc échouer la CI.

La suite `#[test]` du client est actuellement compilée pour
`wasm32-unknown-unknown` (`cargo test --no-run`), mais n'est pas exécutée :
elle n'utilise pas `wasm-bindgen-test` et le dépôt ne configure aucun runner
WASM. Pour valider son exécution, migrer les tests concernés vers
`wasm_bindgen_test` avec `wasm-bindgen-test-runner` (ou un runner navigateur
équivalent), installer un navigateur headless compatible sur le runner CI,
puis exécuter `cargo test --locked -p client --target wasm32-unknown-unknown`
avec `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner`,
`WASM_BINDGEN_USE_BROWSER=1` et `MOZ_HEADLESS=1`. Installer Firefox et
`geckodriver` sur le runner (WebDriver pour Firefox) et ajouter le test
