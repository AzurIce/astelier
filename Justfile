serve:
    nix develop -c tools/bin/dx serve --addr 127.0.0.1 --port 8230 --open false

check:
    nix develop -c sh -c 'cargo check --features server && cargo check --target wasm32-unknown-unknown --features web'

clean:
    rm -rf target dist .dx
