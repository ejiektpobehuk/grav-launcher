default:
    @just --list

clippy:
    cargo clippy -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used

nix-clippy:
    nix develop --command bash -c 'cargo clippy -- -W clippy::pedantic -W clippy::nursery -W clippy::unwrap_used'

nix-build:
    nix develop --command bash -c 'cargo build'

release:
    cargo build --release

nix-release:
    nix develop --command bash -c 'cargo build --release'

patch: release
    patchelf --set-interpreter /usr/lib64/ld-linux-x86-64.so.2 target/release/grav-launcher

nix-patch: nix-release
    nix develop --command bash -c 'patchelf --set-interpreter /usr/lib64/ld-linux-x86-64.so.2 target/release/grav-launcher'
