# how to run a program in [kubernetes](https://kubernetes.io/)
Run [nats](https://github.com/nats-io) through the [helm](https://helm.sh)
```
helm repo add nats https://nats-io.github.io/k8s/helm/charts/
helm install nats nats/nats --set=config.jetstream.enabled=true --namespace=nats --create-namespace
helm install nack nats/nack --set jetstream.nats.url=nats://nats:4222 --namespace=nats --create-namespace
```

```bash
k create namespace telegram
```

ddnet-econ-secret.yaml
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: ddnet
  namespace: telegram
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
      message_thread_id: <thread-telegram-id> "1379" # str only
```

bridge-handler-secret.yaml
```yaml
apiVersion: v1
kind: Secret
metadata:
  name: handler
  namespace: telegram
type: Opaque
stringData:
  config.yaml: |
    nats:
      server: nats.nats:4222
    paths:
      - from: tw.econ.read.*
        regex:
          - "\\[.*?]\\[chat]: \\d+:-?\\d+:(.*): (.*)" # trainfngChatRegex
          - "\\[.*]\\[.*]: \\*\\*\\* '(.*)' (.*)" # trainfngJoinRegex
          - "\\[chat]: \\d+:-?\\d+:(.*): (.*)" # teeworldsChatRegex
          - "^\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2} I chat: (\\d+:-?\\d+:[^:]+): (.*)$" # ddnetChatRegex
          - "^\\d{4}-\\d{2}-\\d{2} \\d{2}:\\d{2}:\\d{2} I chat: \\*\\*\\* '(.*?)' (.*)$" # ddnetJoinRegex
        to:
          - tw.tg.{{message_thread_id}}
      - from: tw.econ.read.*
        regex:
          - "\\[game]: team_join player='\\d+:(.*)' team=0" # teeworldsJoinRegex
        to:
          - tw.tg.{{message_thread_id}}
        template: "{{text_join}}"
```

ddnet-econ-deployment.yaml
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ddnet
  namespace: telegram
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

bridge-handler-deployment.yaml
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: handler
  namespace: telegram
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
k apply -f ddnet-econ-secret.yaml 
k apply -f bridge-handler-secret.yaml 
k apply -f ddnet-econ-deployment.yaml 
k apply -f bridge-handler-deployment.yaml
```

# Additionally
[bot](https://github.com/teeworlds-nats/nats-to-telegram) for sending messages from [nats](https://github.com/nats-io) to [telegram](https://telegram.org)
