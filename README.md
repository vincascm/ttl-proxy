# ttl-proxy

a transparent TCP to SOCKSv5 proxy on Linux

```text
ttl-proxy

options:
    -h  show help
    -s <address> assgin a socks5 server address
    -l <address> assgin a listen address
    -V  show version
```

# usage

just run:

```shell
ttl-proxy
```

or show help

```shell
ttl-proxy -h
```

use `iptables` redirect to `ttl-proxy`:

host 
```shell
iptables -t nat -A OUTPUT -p tcp -m set --match-set myips dst -j REDIRECT --to-port 10800
```

or router
```shell
iptables -t nat -A PREROUTING -p tcp -m set --match-set myips dst -j REDIRECT --to-port 10800
```
