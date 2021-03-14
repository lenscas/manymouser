## Manymouser
A rust version/wrapper of https://icculus.org/manymouse/.

Right now, manymouse is always statically linked and used as a backend. The plan is to rewrite most if not all of it to Rust as time goes on.
However, I have no plans of removing `manymouse` as a possible backend. 

At this point in time, Linux_evdev got an early port to rust. It however still uses A LOT of unsafe code as I tried to keep it close to the original C code for testing.

The other platforms are handled by `manymouse`.

## TODO
1. Demo
2. Windows support
3. X11 input
4. Expose it to more languages like Lua (mlua?) and C#.