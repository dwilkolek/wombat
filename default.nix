let
  pkgs = import <nixpkgs> { };
in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo-tauri.hook
    pkg-config
    gobject-introspection
    cargo
    cargo-tauri
    nodejs_22
    pnpm
  ];

  buildInputs = with pkgs;[
    at-spi2-atk
    atkmm
    cairo
    gdk-pixbuf
    glib
    gtk3
    harfbuzz
    librsvg
    libsoup_3
    pango
    webkitgtk_4_1
    openssl
    glib-networking
  ];
}
