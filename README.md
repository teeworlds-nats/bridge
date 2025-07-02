# DDraceNetwork (DDNet) Bridge

A robust messaging bridge connecting DDNet game servers to Telegram via NATS JetStream.

---

## 🚀 Quick Start

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Run services
cargo run -- econ -c econ.yaml
cargo run -- handler -c handler.yaml
cargo run -- tg reader -c tg.yaml
```

---

## 📦 Microservices

| Service   | Description                  |
|-----------|------------------------------|
| `econ`    | DDNet ECON connector → NATS  |
| `handler` | Message processor and router |
| `tg`      | NATS <-> Telegram            |

---

## 🛠 Deployment

### Kubernetes

[k8s.md](https://github.com/teeworlds-nats/bridge/wiki/k8s)

### Manual

```bash
cargo build --release
./target/release/bridge econ -c econ.yaml
./target/release/bridge handler -c handler.yaml
```

---

Examples of configs on [wiki](https://github.com/teeworlds-nats/bridge/wiki)

---
## 🤝 Contributing

1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Open a pull request
