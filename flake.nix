{
  description = "Abstract dynamic-pricing simulator for linear-Leios (Haskell)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        # Pin the compiler so the dev shell is reproducible. Bump this in one
        # place when you want a newer GHC. Must match the GHC version of the
        # Stackage snapshot in abstract-sim-hs/stack.yaml (currently lts-24/43 -> 9.10.3).
        ghc = pkgs.haskell.packages.ghc9103;
      in
      {
        devShells.default = pkgs.mkShell {
          name = "arc-tiered-pricing";
          # Nix provides the toolchain; Stack (system-ghc) reuses this GHC and
          # manages the Hackage library deps via the snapshot.
          packages = [
            ghc.ghc
            pkgs.stack
            ghc.haskell-language-server
          ];
        };
      });
}
