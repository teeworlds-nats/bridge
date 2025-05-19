# Example configs
[econ](src/econ/config_examples)
[handler](src/handler/config_examples)
[bots/reader](src/bots/reader/config_examples)

# how to run a program in [kubernetes](https://kubernetes.io/)
Run [nats](https://github.com/nats-io) through the [helm](https://helm.sh)
```
helm repo add nats https://nats-io.github.io/k8s/helm/charts/
helm repo update

helm upgrade --install nats nats/nats \
  --set config.jetstream.enabled=true \
  --set config.jetstream.memoryStore.enabled=true \
  --set config.cluster.enabled=true --wait \
  --namespace=nats --create-namespace

helm upgrade --install nack nats/nack \
  --set jetstream.nats.url=nats://nats:4222 --wait \
  --namespace=nats --create-namespace
```


Stream
```
apiVersion: jetstream.nats.io/v1beta2
kind: Stream
metadata:
  name: tw
spec:
  name: tw
  subjects: ["tw.>"]
  storage: memory
  maxAge: 5m
  maxMsgs: 1000
```

```bash
k create namespace bridge
```

bridge-econ.yaml
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
      server: nats.nats:4222
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
        image: bridge:latest
        volumeMounts:
        - name: config
          mountPath: /tw/config.yaml
          subPath: config.yaml
      imagePullSecrets:
        - name: secret
      volumes:
      - name: config
        secret:
          secretName: ddnet
```

bridge-handler.yaml
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
      server: nats.nats:4222
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
        image: bridge:latest
        command:
          - /tw/bridge
          - handler
        volumeMounts:
        - name: config
          mountPath: /tw/config.yaml
          subPath: config.yaml
      imagePullSecrets:
        - name: secret
      volumes:
      - name: config
        secret:
          secretName: handler
```

```bash
k apply -f bridge-econ.yaml
k apply -f bridge-handler.yaml
```