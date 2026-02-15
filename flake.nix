{
  description = "mruby/edge Playground";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = [ "wasm32-unknown-emscripten" ];
        };
      in
      {
        devShells.default = pkgs.mkShell {
          nativeBuildInputs = [
            rustToolchain
            pkgs.emscripten
            pkgs.python3
          ];

          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          BINDGEN_EXTRA_CLANG_ARGS = "-I${pkgs.emscripten}/share/emscripten/cache/sysroot/include";

          CC_wasm32_unknown_emscripten = "emcc";
          AR_wasm32_unknown_emscripten = "emar";

          # Compile C code with wasm-based setjmp/longjmp
          CFLAGS_wasm32_unknown_emscripten = "-sSUPPORT_LONGJMP=wasm";

          shellHook = ''
            export EM_CACHE="$PWD/.emscripten_cache"
            mkdir -p "$EM_CACHE"
          '';
        };
      }
    );
}
