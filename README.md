# tui-patch
Execute commands over SSH on multiple hosts

## Installation of Rust
Go to https://rustup.rs/ follow the instructions to install the rust compiler.
Clone this repository and run following command inside the repository to compile:
```sh
cargo build 
```

## Usage example
Make sure your SSH keys are loaded with a SSH agent.
All Hosts in a file will run simultaniously. It will create a folder in current directory ./log/ with log files from the SSH output for each host.
The log files will be written when each command terminates.

```sh
./target/debug/tui-patch ./examples/ubuntu_packages_upgrade.yaml
```

## Configuration example
Each config file must exist of a tasks and targets section. See for more details in examples folder.
