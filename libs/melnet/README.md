# Melnet: Themelio's network layer

## Summary and rationale

Melnet serves as Themelio's peer-to-peer network layer, based on a randomized topology and gossip. Peers are divided into servers, which have a publicly reachable address, and clients, which do not. It's based on a simple RLP request-response protocol, where the only way to "push" a message is to send a request to a server. There is no multiplexing --- the whole thing works like HTTP/1.1. TCP connections are pretty cheap these days.

This also means that clients never receive notifications, and must poll servers.

We chose not to use existing solutions, notably libp2p, since they tend to be unstable and "overengineered" for Themelio's purposes.

This crate doesn't implement any custom verbs; it just provides a basic tool to maintain network topology.

`smol`-based async is used.

## Basic request format

The request is always an RLP three-list:

```
[32-bit length]
[protocolVersion verb payload]
```

where `protocolVersion = 1`

## Response

Response is either `["Ok" response]` or `["Err" error]` or `["NoVerb" ""]`, prefixed with 32-bit length.

## Built-in verbs

### Peer advertisement

```
verb = "new_peer"
payload = ["protocol" "addr"]
```

```
response = Ok("")
         | Err("unreachable")
```