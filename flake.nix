{
  description = "logos-net-proxy — fail-closed proxyable HTTP client constructor (the wallet's single outbound chokepoint). Building runs the fail-closed test suite.";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      systems = [ "aarch64-darwin" "x86_64-darwin" "aarch64-linux" "x86_64-linux" ];
      forAll = f: nixpkgs.lib.genAttrs systems (s: f nixpkgs.legacyPackages.${s});
      crate = pkgs: pkgs.rustPlatform.buildRustPackage {
        pname = "logos-net-proxy";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;
        # buildRustPackage's checkPhase runs `cargo test` by default, so a
        # successful build IS a green run of the fail-closed invariant suite.
      };
    in
    {
      packages = forAll (pkgs: { default = crate pkgs; });
      checks = forAll (pkgs: { default = crate pkgs; });
      devShells = forAll (pkgs: {
        default = pkgs.mkShell { packages = [ pkgs.cargo pkgs.rustc pkgs.pkg-config ]; };
      });
    };
}
