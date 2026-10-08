# Development Environment

## On your PC (Windows)
1. **Rust**: install from https://rustup.rs (the default stable toolchain). Then, in the repository folder:
   - `cargo test --workspace` runs every test;
   - `cargo run --release -p w5k_tools --bin w5k -- render content --out out` renders every part and vehicle into `out\`;
   - `cargo run --release -p w5k_tools --bin w5k -- family content --out out` renders the parametric family sheets.
2. **Godot 4.6.2**: download the standard (not .NET) build from https://godotengine.org/download/archive/4.6.2-stable/.
   The Godot project (`game/`) and the Rust GDExtension arrive in milestone M2; the extension is built with
   `cargo build --release -p w5k_godot`.
3. **Viewing GLB files in Blender**: File > Import > glTF 2.0, pick a `.glb` from the render output. To see the colours,
   switch the viewport to Solid shading and set Color to *Attribute* (it uses `COLOR_0`). Each part's sockets appear as
   empties named `socket_<name>`.

## In a Claude Code cloud session
- Rust and cargo are installed, and crates.io is reachable; everything above works headless.
- Previews come from the forge's own software rasteriser (no GPU needed).
- Godot is not preinstalled. A session can download the Linux editor
  (`Godot_v4.6.2-stable_linux.x86_64.zip`, about 72 MB) into its scratchpad and run it headless (`--headless`), or under
  Xvfb with software OpenGL for screenshots. Do not commit the binary.
- Unity cannot run there: its licence needs an interactive Hub sign-in and its download hosts are blocked.
