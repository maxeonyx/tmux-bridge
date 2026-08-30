{ pkgs, ... }:

{
  packages = [
    pkgs.actionlint
    pkgs.cargo
    pkgs.clippy
    pkgs.curl
    pkgs.fish
    pkgs.gcc
    pkgs.gh
    pkgs.git
    pkgs.pkg-config
    pkgs.rustc
    pkgs.rustfmt
    pkgs.tmux
  ];

  enterTest = ''
    actionlint
    cargo fmt --check
    cargo clippy -- -D warnings
    cargo ratchet
  '';
}
