# POP Booking [![Build and Release](https://github.com/netgrp/pop-booking/actions/workflows/CICD.yml/badge.svg)](https://github.com/netgrp/pop-booking/actions/workflows/CICD.yml)

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

`docker build -t <imagename> .`

And then run it while binding the necessary port and volumes, along with the environment file.

`docker run -p 8080:8080 -v ./db:/app/db -v ./config:/app/config --env-file .env <imagename>`

## Deploying as a systemd service with podman quadlet

For deploying this as a systemd service there is an included quadlet file `pop-booking.container`. For a rootless deployment place this file in `$HOME/.config/containers/systemd/` then reload the deamon to generate a new service `systemctl --user daemon-reload`. If the service isn't generated check the output of /usr/libexec/podman/quadlet -dryrun -user to see that it's generating, and that podman is set up as a systemd user generator. 

After reloading, if everything has been configured correctly, the service can be started with `systemctl --user start pop-booking`. To allow the service to start itself you will need to enable login linger for the user running podman, which is done by running `loginctl enable-linger <userame>`.
