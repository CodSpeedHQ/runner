{
  description = "An empty flake template that you can adapt to your own environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};
        lib = pkgs.lib;
      in {
        devShells = {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [
              # Rust
              mold
              rustup
              openssl
              perl
              pkg-config
              just
              llvmPackages.bintools # lld

              # eBPF dependencies
              elfutils
              zlib
              clang_21
              llvm_21
              bpftools
              libbpf
            ];
            # Set environment variables for all subprocesses, not just the shell
            LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
            LD_LIBRARY_PATH = "${lib.getLib pkgs.elfutils}/lib:${pkgs.zlib}/lib:${pkgs.libbpf}/lib";
            NIX_HARDENING_ENABLE = "";
            shellHook = ''
              echo "Welcome to the Rust development environment!"
            '';
          };
        };
      }
    );
}
