<div align="center">

# Rcat
### A better netcat for hackers

</div>

### Overview

__Rcat__ is a modern _netcat_ written in Rust, packed with features for hackers.

<img src="images/rcat_curl.svg">

Here we listen on the port `tcp:9001`, and recieve an HTTP request made with `curl`.

### Install

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
No need to [type 7 commands](https://blog.ropnop.com/upgrading-simple-shells-to-fully-interactive-ttys/) every type you obtain a reverse shell.

<img src="images/rcat_pwn.svg">

#### TLS support

Support of __TLS__ with `-t` or `--tls`.  
Here we do an HTTPS request. We use `-t` to establish a _TLS connection_, and `-c` to send _CRLF_ newlines.

<img src="images/rcat_tls.svg">

