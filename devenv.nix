{ pkgs, ... }: {
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" ];
  };

  packages = with pkgs; [ cargo-expand ];

  enterShell = ''
    cargo install --locked cargo-autoinherit
  '';

}
