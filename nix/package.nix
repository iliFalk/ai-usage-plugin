{
  lib,
  makeWrapper,
  nasm,
  procps,
  rustPlatform,
  stdenv,
  xdg-utils,
}:

let
  cargoToml = builtins.fromTOML (builtins.readFile ../Cargo.toml);
  linuxRuntimePath = lib.makeBinPath [
    procps
    xdg-utils
  ];
in
rustPlatform.buildRustPackage {
  pname = "ai-usagebar";
  version = cargoToml.package.version;

  src = lib.fileset.toSource {
    root = ../.;
    fileset = lib.fileset.unions [
      ../Cargo.toml
      ../Cargo.lock
      ../src
      ../config.example.toml
      ../README.md
      ../LICENSE
    ];
  };

  cargoLock.lockFile = ../Cargo.lock;

  nativeBuildInputs =
    lib.optionals stdenv.hostPlatform.isx86_64 [ nasm ]
    ++ lib.optionals stdenv.hostPlatform.isLinux [ makeWrapper ];

  postInstall = ''
    install -Dm644 config.example.toml \
      "$out/share/ai-usagebar/config.example.toml"
    install -Dm644 README.md \
      "$out/share/doc/ai-usagebar/README.md"
    install -Dm644 LICENSE \
      "$out/share/licenses/ai-usagebar/LICENSE"
  ''
  + lib.optionalString stdenv.hostPlatform.isLinux ''
    wrapProgram "$out/bin/ai-usagebar" \
      --prefix PATH : "${linuxRuntimePath}"
  '';

  meta = {
    description = "Data source for the Omarchy AI Usage panel: quota and balance for OpenCode Go, DeepSeek and OpenRouter";
    homepage = "https://github.com/iliFalk/ai-usage-plugin";
    license = lib.licenses.mit;
    mainProgram = "ai-usagebar";
    platforms = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
  };
}
