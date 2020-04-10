# Mothra: LibP2P for Dummies

**⚠️ This is a work in progress! ⚠️**

Mothra was created to house bindings between [Rust-LibP2P](https://github.com/libp2p/rust-libp2p) and any number of other languages.  The languages that are supported (so far) are:

- C
- Java
- .Net
- Rust (duh)

### The Thing That Should Not Be

![mothra](./resources/mothra.jpg)
  
Mothra providea a simple (slightly opinionated) API that requires no previous knowledge of libP2P.  The API consists of:

- Basic network configuration settings 
  - ip addresses
  - ports
  - logging
  - etc
- GossipSub
- RPC
- Discv5



### MacOS Prereqs

Rust:

Install rustup so you can switch between Rust versions:

```sh

> brew install rustup

```

Install the Rust compiler and package manager:

```sh

> rustup-init

```

DotNet:

Install dotnet sdk [here](https://download.visualstudio.microsoft.com/download/pr/749db4bc-73c3-4ffb-a545-c315dc9a0ca8/5281258f5dcae636efe557b8b305e20b/dotnet-sdk-3.1.101-osx-x64.pkg)

Once the dotnet sdk is installed open a new Terminal and type:

```sh
> dotnet
Usage: dotnet [options]
Usage: dotnet [path-to-application]

Options:
  -h|--help         Display help.
  --info            Display .NET Core information.
  --list-sdks       Display the installed SDKs.
  --list-runtimes   Display the installed runtimes.

path-to-application:
  The path to an application .dll file to execute.
```

> Note: if you receive an error message make sure you are working from a new Terminal session


`tmux` (optional):

Installing this will make running the demo easier:

```sh

> brew install tmux

```

On Windows:

Follow the instructions to install rust-up on Windows.

You will need Visual Studio installed; the linker is used by Rust, and the compiler for the C-bindings and example, and dotnet core for the dotnet example.

You will also need vcpkg installed in a sibling folder of Mothra. Clone it from 'https://github.com/microsoft/vcpkg' and then run the bootstrap to configure.

Then use it to install the OpenSSL libraries:

```
vcpkg install openssl:x64-windows
```

### Build Mothra

Building is easy.  First, clone the repo:

```sh

> git clone https://github.com/prrkl/mothra.git

```

#### Build for Rust

Next cd into the project's root dir and build:

```sh

> make rust

```

#### Build for C

Next cd into the project's root dir and build:

```sh

> make c

```

#### Build for Java

Next cd into the project's root dir and build:

```sh

> make java

```

#### Build for DotNet

Next cd into the project's root dir and build:

```sh

> make dotnet

```

#### Build on Windows

Run the build scripts a in Visual Studio x64 Native Tools Command Prompt.

A set of batch files, conveniently called Make.cmd, are provided for running on Windows.

```
> Make.cmd
```

This will make the C example and dotnet example. (It's not a make file, so no separate targets, just a batch file.)

There is no tmux, but there is a PeerDemo.cmd batch file in the dotnet example which will launch and run two dotnet sessions.

You can also grab the ENR and run a demo between C and dotnet.


### Sample App

Here is a screenshot of the sample app in action:

![demo](./resources/demo.jpeg)


#### Run Sample App (Rust)

The sample app demonstrates two clients using Disv5 to find each other and the use of GossipSub to send messages back and forth.

If you have `tmux` installed, it is a little simpler to run:

```sh

> cd examples/rust && sh peerDemo.sh
```

#### Run Sample App (C)

The sample app demonstrates two clients using Disv5 to find each other and the use of GossipSub to send messages back and forth.

If you have `tmux` installed, it is a little simpler to run:

```sh

> cd examples/c && sh peerDemo.sh

```

#### Run Sample App (Java)

The sample app demonstrates two clients using Disv5 to find each other and the use of GossipSub to send messages back and forth.

If you have `tmux` installed, it is a little simpler to run:

```sh

> cd examples/java && sh peerDemo.sh

```

#### Run Sample App (DotNet)

The sample app demonstrates two clients using Disv5 to find each other and the use of GossipSub to send messages back and forth.

If you have `tmux` installed, it is a little simpler to run:

```sh

> cd examples/dotnet && sh peerDemo.sh

```

### Credits/Acknowledgements

- A big thanks to the [Lighthouse](https://github.com/sigp/lighthouse) crew.  Not only does Mothra shamelessly borrow from their project, but I literally learned Rust by looking at their code.

- Since Mothra is essentially a wrapper around [Rust-LibP2P](https://github.com/libp2p/rust-libp2p), they deserve a fist bump too.≠≠≠
