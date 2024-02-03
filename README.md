# POP Booking

This is so far a hastily written frontend and backend for the new pop booking system. The idea was to minimize external dependencies
and create a piece of software that will last for a long time. The only real dependency there is in the program currently is the 
login server from knet. It also relies on being able to compile rust code.

Building depends on `build-essential libssl-dev`.
Rustup install command `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`.

It compiles and runs with the command `cargo run --release --offline`. For debugging symbols leave out the release option.

Environment variables are to be specified in the `.env` file in the backend directory. An example file is included.

The available resources can be changed in the resources.json file. Hopefully this becomes a piece of software which is both robust and 
is easy to change and improve.

## Building with docker/podman

If deploying with docker/podman, build the image with the dockerfile

`docker build -t imagename .`

And then run it while binding the necessary port and volumes, along with the environment file.

`docker run -it  -p 8080:8080 -v $(pwd)/db:/app/db -v $(pwd)/config:/app/config --env-file .env imagename`
