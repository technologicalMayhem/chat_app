# Chat App

## About
This is a simple chat app composed of a server and a client application. I worked on this project mostly to familiarize myself with [diesel](https://diesel.rs/), [rocket.rs](https://rocket.rs/) and [tui](https://crates.io/crates/tui).

## Features

- Server and client communicating via HTTP
- Login and register system
- Client updates messages in real time
- Client supports multiple logins at once

## Installation
If you want to build this project, make sure you have rust installed, then just clone the project and run ``cargo build``. No further setup should be required.

Otherwise you can grab the prebuilt binaries from the [releases page](https://github.com/technologicalMayhem/chat_app/releases). There is a windows and linux version available.

## Usage
Just run the server binary for the server to start the server. By default it only bind to ``127.0.0.1`` on port ``8888``. If you want to change that, create a file called ``Rocket.toml`` and add the following to it:
```
[default]
address = "127.0.0.1"
port = 8000
```
Just change the address and port entry to whatever you want. Make sure that when you start the server, the configuration file is located in your working directory.

Once the server is running, you can connect to it using the client. Simply enter the server address, your username and password. Then select whether you want to register as a new user or login as a existing one. If that's the first time you connect to the server you need to register since there are by default no accounts created.