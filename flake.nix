{
  description = "pgrx development shell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };
        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "clippy"
            "rust-src"
          ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            bacon
            bison
            cargo-pgrx_0_16_1
            clang
            flex
            pkg-config
            rust
          ];

          buildInputs = with pkgs; [
            icu
            openssl
            readline
            zlib
          ];

          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.glibc.dev}/include";
          BITCOIN_NETWORK = "regtest";
          BITCOIN_RPC_PASSWORD = "pg_bitcoin";
          BITCOIN_RPC_URL = "http://127.0.0.1:18443";
          BITCOIN_RPC_USER = "pg_bitcoin";
          ELECTRUM_URL = "tcp://127.0.0.1:60401";
          FULCRUM_HOST = "127.0.0.1";
          FULCRUM_PORT = "60401";
          FULCRUM_URL = "tcp://127.0.0.1:60401";
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          MEMPOOL_URL = "http://127.0.0.1:8080";
          RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/library";
        };
      }
    );
}
