# tui-patch
Execute commands over SSH on multiple hosts

## Usage example
Make sure your SSH keys are loaded with a SSH agent.
All Hosts in a file will run simultaniously. It will create a folder in current directory ./log with log files from the SSH output for each host.
The log files will be written when each command terminates.

```sh
./tui-patch update_servers.yaml 
````

## Configuration example
Each config file must exist of a tasks and targets section. See for more details in examples folder.
