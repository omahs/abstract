workspaces := "./framework ./modules ./app-template"

# Pull a specific repo from its main remote
pull repo:
    git subtree pull --prefix={{repo}} {{repo}} main

# Push the local repo to a specific branch
push repo branch:
    git subtree push --prefix={{repo}} {{repo}} {{branch}}

# Run a cargo command in all the workspace repos
cargo-all *command:
  #!/usr/bin/env bash
  set -e;
  for path in {{workspaces}}
  do 
    (cd $path; cargo {{command}}); 
  done
  set +e

check path:
    (cd {{path}}; cargo check)

check-all path:
    (cd {{path}}; cargo check --all-features)

nightly-build:
  just cargo-all build --all-features

# Wasms all the workspaces that can be wasm'd
wasm-all:
  ./scripts/wasm-all.sh

# Wasms all the workspaces that can be wasm'd
wasm-all-ci:
  ./scripts/wasm-all-ci.sh

# Generates JSON schemas for all the contracts in the repo.
schema:
  #!/usr/bin/env bash
  set -e
  (cd app-template; cargo run --example schema --features schema)
  sh scripts/modules-schema.sh
  sh scripts/framework-schema.sh
  set +e
