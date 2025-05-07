{
  description = "Development environment for grav-launcher";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustc
            cargo
            # DX
            bacon
            just
            # System libraries
            udev # Provides libudev for controller support
            pkg-config # Required for finding libudev
            # Release tooling
            patchelf
          ];

          # Set up environment variables for pkg-config
          shellHook = ''
            # Ensure pkg-config can find libudev
            export PKG_CONFIG_PATH="${pkgs.udev}/lib/pkgconfig:$PKG_CONFIG_PATH"
          '';
        };
      }
    );
}
