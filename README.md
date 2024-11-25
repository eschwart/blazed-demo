# blazed-demo

A client-server 3D renderer featuring real-time player movement and view synchronization within a dynamic 128-tick rate system. Built with [Rust](https://www.rust-lang.org/) using [OpenGL](https://crates.io/crates/glow) and [SDL2](https://crates.io/crates/sdl2).

## Build
Retrieve the repository:
```bash
git clone https://github.com/splurf/blazed-demo
```

## Running the Client
Before attempting to compile the client, you will need to configure [SDL2](https://github.com/Rust-SDL2/rust-sdl2?tab=readme-ov-file#windows-msvc). Afterwards, you should have the library files placed into your toolchain and the `.dll` file at `client\SDL2.dll` in this project's directory.
```bash
cargo r --release --manifest-path client\Cargo.toml
```

## Additional Usage
**Client**
```bash
Usage: client server --remote-tcp-addr <REMOTE_TCP_ADDR> --local-udp-addr <LOCAL_UDP_ADDR> --remote-udp-addr <REMOTE_UDP_ADDR>

Options:
      --remote-tcp-addr <REMOTE_TCP_ADDR>  Remote TCP IP address
      --local-udp-addr <LOCAL_UDP_ADDR>    Local IP address
      --remote-udp-addr <REMOTE_UDP_ADDR>  Remote IP address
  -h, --help                               Print help
```

**Server**
```bash
Usage: server [OPTIONS] --tcp-addr <TCP_ADDR> --udp-addr <UDP_ADDR>

Options:
  -t, --tcp-addr <TCP_ADDR>  TCP IP address
  -u, --udp-addr <UDP_ADDR>  UDP IP address
      --tps <TPS>            [default: 128]
  -h, --help                 Print help
```

## Notes
- The server has a `--tps` flag, which will allow you to specify the tick rate (min: 1, max: 255).