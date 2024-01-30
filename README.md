# POP Booking

This is so far a hastily written frontend and backend for the new pop booking system. The idea was to minimize external dependencies
and create a piece of software that will last for a long time. The only real dependency there is in the program currently is the 
login server from knet. It also relies on being able to compile rust code.

It compiles and runs with the command `cargo run --release --offline`. For debugging symbols leave out the release option.

Environment variables are to be specified in the `.env` file in the backend directory. An example file is included.

The available resources can be changed in the resources.json file. Hopefully this becomes a piece of software which is both robust and 
is easy to change and improve.