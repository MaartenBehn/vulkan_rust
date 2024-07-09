{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell rec {

  name = "vulkan-rust";
  RUSTC_VERSION = "stable";

  shellHook = ''
    export PATH=$PATH:''${CARGO_HOME:-~/.cargo}/bin
    export PATH=$PATH:''${RUSTUP_HOME:-~/.rustup}/toolchains/$RUSTC_VERSION-x86_64-unknown-linux-gnu/bin/
    '';

  packages = with pkgs; [
    rustup
    clang
    pkg-config
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    glslang
    linuxPackages_latest.perf
    hotspot
    cmake
    fontconfig
    vulkan-tools
  ];

  buildInputs = with pkgs; [

  ];

  LD_LIBRARY_PATH = with pkgs; lib.makeLibraryPath [
   # load external libraries that you need in your rust project here
   libxkbcommon
   wayland-scanner.out
  ];

  # Add precompiled library to rustc search path
  RUSTFLAGS = (builtins.map (a: ''-L ${a}/lib'') [
    # add libraries here (e.g. pkgs.libvmi)
    pkgs.vulkan-headers
    pkgs.vulkan-loader
    pkgs.vulkan-validation-layers

  ]);

  VULKAN_SDK = "${pkgs.vulkan-headers}";
  VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
}
