## Manymouser
A rust version of https://icculus.org/manymouse/.
The code so far is an almost line to line translation of https://github.com/NoobsArePeople2/manymouse

Currently, only the evdev backend is ported. Plan is to support windows and X11 input as well. Also, ideally I have the manymouse library itself be able to act as a backend but no idea how to do that yet.

## TODO
1. Demo
2. Windows support
3. X11 input
4. Manymouse itself as a backend?
5. Expose it to more languages like Lua (mlua) and C#.