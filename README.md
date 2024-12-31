# Ferrix

Ferrix is a high-performance reverse proxy and load balancer designed specifically for Kubernetes environments. Built on top of Cloudflare's Pingora framework, it provides efficient HTTP routing and load balancing capabilities while seamlessly integrating with Kubernetes through Custom Resource Definitions (CRDs).

## Features

- High-performance HTTP routing powered by Pingora
- Native Kubernetes integration via custom IngressRoute resources
- Dynamic configuration updates through Kubernetes API watches
- Round-robin load balancing
- Support for TLS termination
- Efficient memory management and low latency
- Simple and declarative configuration

## Architecture

Ferrix consists of two main components:

### Proxy Server

The proxy server handles incoming HTTP traffic and routes it to the appropriate backend services. It leverages Pingora's high-performance networking stack and provides:

- HTTP/1.1 and HTTP/2 support
- Efficient connection pooling
- Request routing based on host and path matching
- Load balancing across multiple backend instances

### Kubernetes Controller

The Kubernetes controller watches for IngressRoute resources and dynamically updates the proxy's routing configuration. It:

- Monitors the Kubernetes API for changes to IngressRoute resources
- Maintains the routing table in real-time
- Handles service discovery and endpoint updates
- Manages TLS certificate configuration

## Installation

1. First, install the Custom Resource Definition for IngressRoute:

```bash
kubectl apply -f crds/k8s/ingressroute.yaml
```

2. Deploy Ferrix using the provided manifest:

```bash
kubectl apply -f deploy/ferrix.yaml
```

## Configuration

### IngressRoute Resource

Ferrix uses IngressRoute CRDs to define routing rules. Here's an example:

```yaml
apiVersion: ferrix.com/v1
kind: IngressRoute
metadata:
  name: example-route
  namespace: default
spec:
  entrypoint: web
  route:
    host: example.com
    rules:
      - matches: Path(`/api`)
        service:
          name: api-service
          port: 8080
  tls: default-tls
```

### Server Configuration

The proxy server is configured through a YAML file:

```yaml
entry_points:
  - name: web
    port: 6190
    secure: false
server:
  threads: 1
```

## Development

Ferrix is written in Rust and uses several key dependencies:

- Pingora: High-performance HTTP proxy framework
- kube-rs: Kubernetes client and controller runtime
- tokio: Asynchronous runtime
- clap: Command line argument parsing

To build the project:

```bash
cargo build --release
```

Run tests:

```bash
cargo test
```

## Performance Considerations

Ferrix is designed for high performance:

- Zero-copy networking when possible
- Efficient connection pooling
- Minimal memory allocations
- Lock-free data structures for concurrent access
- Asynchronous I/O throughout the stack

## Contributing

Contributions are welcome! Please feel free to submit pull requests. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the terms of the [LICENSE](LICENSE.md) file

## Project Status

This project is under active development. While it's functional, it should be considered beta software. The API and configuration format may change as we gather more real-world usage feedback.
