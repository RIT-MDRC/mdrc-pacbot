Welcome to MDRC Pacbot!

Our team focuses on developing a small fully autonomous robot to play the arcade game Pac-Man on a physical field.
You can read more about the game and grid system in our [book](https://rit-mdrc.github.io/mdrc-pacbot/).

Our code is divided into parts to improve maintainability:

- [core_pb](https://rit-mdrc.github.io/mdrc-pacbot/api/core_pb/) contains structs and logic that are used by multiple
apps, such as code that runs both on the robot or in the simulator, or messages that are passed from clients to servers.
- [pico_pb](pico_pb/README.md) contains code that runs directly on the robot. Note: `pico_pb` is not part of the Rust
workspace because it builds on a special target with special dependency and `core_pb` features.
- `sim_pb` contains code to run a simulator that can emulate multiple robots and a game server. From the POV of `server_pb`,
there is no difference between connecting to a robot from the simulator vs a robot in real life. The simulated game server, however,
does offer extra functionality not available in the official game server (but it is compatible with the official web client).
- `server_pb` handles networking between the other apps, as well as high level strategy
- [gui_pb](gui_pb/README.md) works both as a native Rust app and a WASM app to display the user interface

Additionally, the [official Pacbot competition code](https://github.com/Pacbot-Competition/Pacbot-2) offers a game server and a web
client that will be used during competition. Our simulator and gui offer the same functionality and more, but it is important
that our code can work with the official software.

Finally, we use Python code to create machine learning models to play Pacman [here](https://github.com/qxzcode/pacbot-rl).