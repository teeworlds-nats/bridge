# Running NATS with JetStream in Kubernetes using Helm

## Prerequisites

- Kubernetes cluster
- Helm installed

## 1. Install NATS with JetStream and NACK

```bash
# Add NATS Helm repository
helm repo add nats https://nats-io.github.io/k8s/helm/charts/
helm repo update

# Install NATS with JetStream enabled
helm upgrade --install nats nats/nats \
  --set config.jetstream.enabled=true \
  --set config.jetstream.memoryStore.enabled=true \
  --set config.cluster.enabled=true --wait \
  --set config.jetstream.fileStore.pvc.enabled=false \
  --namespace=nats --create-namespace

# Install NACK (NATS Controller for Kubernetes)
helm upgrade --install nack nats/nack \
  --set jetstream.nats.url=nats://nats:4222 --wait \
  --namespace=nats --create-namespace
```

## 2. Create Stream Definition

```yaml
apiVersion: jetstream.nats.io/v1beta2
kind: Stream
metadata:
  name: tw
spec:
  name: tw
  subjects: [ "tw.>" ]
  storage: memory
  maxMsgs: 1000
```

## 3. Deploy Bridge Components

### Create Namespace

```bash
kubectl create namespace bridge
```

### Bridge ECON Configuration (bridge-econ.yaml)

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: ddnet
  namespace: bridge
type: Opaque
stringData:
  config.yaml: |
    nats:
      server:
        - nats://nats:4222
    econ:
      host: <server>:8303
      password: amogus
    args:
      server_name: ddnet
      message_thread_id: "<thread-telegram-id>"
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ddnet
  namespace: bridge
  labels:
    type: ddnet
spec:
  replicas: 1
  selector:
    matchLabels:
      type: ddnet
  strategy:
    type: Recreate
  template:
    metadata:
      labels:
        type: ddnet
    spec:
      containers:
        - name: bridge
          image: <IMAGE_HERE>
          volumeMounts:
            - name: config
              mountPath: /tw/config.yaml
              subPath: config.yaml
      volumes:
        - name: config
          secret:
            secretName: ddnet
```

### Bridge Handler Configuration (bridge-handler.yaml)

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: handler
  namespace: bridge
type: Opaque
stringData:
  config.yaml: |
    nats:
      server:
        - nats://nats:4222
    paths:
      - from: tw.econ.read.*
        regex:
          - "^\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2} I chat: \\d+:-?\\d+:([^:]+): (.*)$" # ddnetChatRegex
          - "^\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2} I chat: \\*\\*\\* '(.*?)' (.*)$" # ddnetJoinRegex
        to:
          - tw.tg.{{message_thread_id}}
        args:
             server_name: Test
---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: handler
  namespace: bridge
  labels:
    type: handler
spec:
  replicas: 1
  selector:
    matchLabels:
      type: handler
  strategy:
    type: Recreate
  template:
    metadata:
      labels:
        type: handler
    spec:
      containers:
        - name: handler
          image: <IMAGE_HERE>
          command:
            - /tw/bridge
            - handler
          volumeMounts:
            - name: config
              mountPath: /tw/config.yaml
              subPath: config.yaml
      volumes:
        - name: config
          secret:
            secretName: handler
```

## 4. Apply Configurations

```bash
kubectl apply -f bridge-econ.yaml
kubectl apply -f bridge-handler.yaml
```

## Notes

- Replace all placeholder values in angle brackets (< >) with actual values
