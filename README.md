# blazed-demo

A fast 3D first-person multiplayer renderer built entirely in Rust with performance as its core focus.
Built with [Rust](https://www.rust-lang.org/) using [OpenGL](https://crates.io/crates/glow) and [SDL2](https://crates.io/crates/sdl2).

## Features
- full body *translation* (up, down, left, right, forward, back) and *rotation* (yaw, pitch)
- n-tick rate (TPS) server (min: 1, max: 1024, default: 128).
- multiplayer (insecure)

## Graphics
- instance-based rendering (currently 2 groups).
- basic lighting (ambient + diffuse + specular).
- back-face culling (more to be implemented).

## Bugs
- Multiplayer is incredibly buggy (mostly just buggy movement).
- When FPS is uncapped and there are many objects (thousands), keyboard input handling gets funky.
  - maybe a bottleneck with certain channels..
- The server tends to fall apart with 3+ connections.
- In-and-out clipping with translucent objects.
  - preventing object clipping (collision) entirely (objects inside one another) might fix this.

## Todo
1. Add/improve documentation.
2. Stabilize `server` (entire multiplayer impl).
3. Implement culling techniques (frustrum & occlusion).
4. Implement [AABB](https://developer.mozilla.org/en-US/docs/Games/Techniques/3D_collision_detection)-based collision (look into [kiddo](https://crates.io/crates/kiddo)).
5. Implement client-side server interpolation.
6. Improve instanced-based rendering.

## Development
Retrieve the repository:
```bash
git clone https://github.com/splurf/blazed-demo
```

## Running the Client
Before attempting to compile the client, you will need to configure [SDL2](https://github.com/Rust-SDL2/rust-sdl2?tab=readme-ov-file#windows-msvc). Afterwards, you should have the library files placed into your toolchain and the `.dll` file at `client\SDL2.dll` in this project's directory.
```bash
cargo r --release --manifest-path client\Cargo.toml
```

## Running the Server
```bash
cargo r --release --manifest-path server\Cargo.toml
```

## Additional Usage
**Client**
```rs
Usage: client [OPTIONS]

Options:
      --fps <FPS>                          Specify the FPS [default: 120]
      --offline                            Do not attempt to connect to server
      --remote-tcp-addr <REMOTE_TCP_ADDR>  Remote TCP IP address [default: 127.0.0.1:54269]
      --local-udp-addr <LOCAL_UDP_ADDR>    Local UDP IP address (optional)
      --remote-udp-addr <REMOTE_UDP_ADDR>  Remote UDP IP address [default: 127.0.0.1:54277]
  -h, --help                               Print help
```

**Server**
```rs
Usage: server [OPTIONS]

Options:
  -t, --tcp-addr <TCP_ADDR>  Local TCP IP address [default: 127.0.0.1:54269]
  -u, --udp-addr <UDP_ADDR>  Local UDP IP address [default: 127.0.0.1:54277]
      --tps <TPS>            Server ticks/sec [default: 128]
  -h, --help                 Print help
```