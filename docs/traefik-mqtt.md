# Traefik MQTT entry points

The control plane and devices use MQTT semantics. Traefik is an edge proxy for
that protocol; it does not convert HTTP requests into MQTT messages.

## Network boundary

- `app` and `worker` may use `mqtt://mosquitto:1883` only across the private
  Compose network, with `MQTT_ALLOW_PLAINTEXT_INTERNAL=true` set explicitly.
- Devices use `mqtts://mqtt.example.com:8883`, or `wss://mqtt.example.com/mqtt`
  when port 443 is the only reachable port.
- Do not publish Mosquitto port `1883` on the host in production.
- Device credentials are per-device; broker ACLs permit only that device's own
  `glance_deck/<device_id>/#` namespace. The control-plane identity is allowed
  across namespaces.

## TCP TLS termination

Run this dynamic configuration on the Traefik instance that owns the public
certificate. It terminates TLS on 8883 and forwards raw MQTT to a private
Mosquitto listener.

```yaml
tcp:
  routers:
    glance-deck-mqtt:
      entryPoints: [mqtts]
      rule: HostSNI(`mqtt.example.com`)
      service: glance-deck-mqtt
      tls: {}
  services:
    glance-deck-mqtt:
      loadBalancer:
        servers:
          - address: mosquitto:1883
```

## WebSocket TLS termination

If the network permits only 443, enable a Mosquitto WebSocket listener on a
private port and route it as HTTP. Devices then use `wss://mqtt.example.com/mqtt`.
The protocol remains MQTT over WebSocket, including QoS, retained messages and
LWT; it is not REST or an HTTP-to-MQTT gateway.

```yaml
http:
  routers:
    glance-deck-mqtt-ws:
      entryPoints: [websecure]
      rule: Host(`mqtt.example.com`) && PathPrefix(`/mqtt`)
      service: glance-deck-mqtt-ws
      tls: {}
  services:
    glance-deck-mqtt-ws:
      loadBalancer:
        servers:
          - url: http://mosquitto:9001
```

Use either the TCP listener or WSS per device. Both must validate the Traefik
certificate on the device; firmware rejects `mqtt://` and `ws://` broker
endpoints received through enrollment.
