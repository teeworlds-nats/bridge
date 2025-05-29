# DDraceNetwork (DDNet) Bridge

A robust messaging bridge connecting DDNet game servers to Telegram via NATS JetStream.

## 🚀 Quick Start

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Run services
cargo run -- econ -c configs/econ.yaml
cargo run -- handler -c configs/handler.yaml
cargo run -- bots/reader -c configs/reader.yaml
```

## 📦 Microservices

| Service       | Description                         |
|---------------|-------------------------------------|
| `econ`        | DDNet ECON connector → NATS         |
| `handler`     | Message processor and router        |
| `bots/reader` | NATS → Telegram message forwarder   |

## ⚙️ Configuration

```bash
# Basic usage
cargo run -- <service> --config <file.yaml>

# Example
cargo run -- handler -c ./custom-handler-config.yaml
```
## 📁 Example Configurations

Ready-to-use configuration examples for each service:

| Сервис         | Пример конфигурации                                                |
|----------------|--------------------------------------------------------------------|
| `econ`         | [econ/config.yaml](src/econ/config_examples/basic.yaml)            |
| `handler`      | [handler/config.yaml](src/handler/config_examples/basic.yaml)      |
| `bots/reader`  | [reader/config.yaml](src/bots/reader/config_examples/basic.yaml)   |

### 📁 Full directory structure:

Explore our example configurations for each component:

```
📂 src/
├── 📂 econ/
│   └── 📂 config_examples/
│       └── basic.yaml       # ECON connection settings
├── 📂 handler/
│   └── 📂 config_examples/
│       └── basic.yaml       # Message processing rules
└── 📂 bots/
    └── 📂 reader/
        └── 📂 config_examples/
            └── basic.yaml   # Telegram bot setup      
```

## 🛠 Deployment

### Kubernetes
[k8s.md](guide/k8s.md)

### Manual
```bash
cargo build --release
./target/release/econ -c src/econ/basic.yaml
./target/release/handler -c src/handler/basic.yaml
```

## 🤝 Contributing

1. Fork the repository
2. Create your feature branch
3. Commit your changes
4. Push to the branch
5. Open a pull request
