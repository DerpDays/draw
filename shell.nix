{
  pkgs ? import <nixpkgs> { },
}:
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    # mostly for wgpu
    libxkbcommon
    wayland
    xorg.libX11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    alsa-lib
    fontconfig
    freetype
    shaderc
    directx-shader-compiler
    pkg-config
    cmake
    mold

    dbus

    libGL
    vulkan-headers
    vulkan-loader
    vulkan-tools
    vulkan-tools-lunarg
    vulkan-extension-layer
    vulkan-validation-layers
  ];

  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${builtins.toString (pkgs.lib.makeLibraryPath buildInputs)}";
  '';
}
