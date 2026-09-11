# Helm Deployment Guide: HermesGate

This guide details how to package, deploy, and manage **HermesGate** on a Kubernetes cluster using Helm.

---

## 1. Prerequisites

- Kubernetes cluster (v1.24+)
- [Helm](https://helm.sh/) v3.10+
- A container image built and pushed to a registry (or loaded into your local cluster e.g. Minikube / Kind / k3s)
- A persistent storage provisioner (e.g. standard ReadWriteOnce StorageClass)

---

## 2. Building the Container Image

Build the container image using Docker or Podman:
```bash
# Using Docker
docker build -t your-registry/hermesgate:0.1.0 .
docker push your-registry/hermesgate:0.1.0

# Or using Podman
podman build -t your-registry/hermesgate:0.1.0 .
podman push your-registry/hermesgate:0.1.0
```

---

## 3. Quick Deployment (Default Values)

Deploy using the local chart in `charts/hermesgate`:

```bash
helm install hermesgate ./charts/hermesgate \
  --namespace hermesgate \
  --create-namespace \
  --set image.repository=your-registry/hermesgate \
  --set image.tag=0.1.0
```

---

## 4. Production Configuration Example

Create a custom `my-values.yaml`:

```yaml
replicaCount: 1

image:
  repository: ghcr.io/yourorg/hermesgate
  tag: "0.1.0"
  pullPolicy: IfNotPresent

# Configure a fixed Master Admin Token
config:
  adminToken: "sms_adm_your_super_secret_admin_token"

# Persistent Volume Claim for SQLite storage
persistence:
  enabled: true
  size: 10Gi
  storageClass: "gp3" # or local-path, standard, etc.

# Ingress configuration with TLS (cert-manager)
ingress:
  enabled: true
  className: "nginx"
  annotations:
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "3600"
    nginx.ingress.kubernetes.io/proxy-send-timeout: "3600"
  hosts:
    - host: sms.example.com
      paths:
        - path: /
          pathType: Prefix
  tls:
    - secretName: hermesgate-tls
      hosts:
        - sms.example.com

# Resource requests & limits
resources:
  requests:
    cpu: 100m
    memory: 128Mi
  limits:
    cpu: 500m
    memory: 512Mi
```

Deploy with your custom values:
```bash
helm upgrade --install hermesgate ./charts/hermesgate \
  --namespace hermesgate \
  --create-namespace \
  -f my-values.yaml
```

---

## 5. Post-Deployment Verification

### Access the Web UI Dashboard
If using port-forward:
```bash
kubectl --namespace hermesgate port-forward svc/hermesgate 8080:80
```
Open `http://localhost:8080` in your browser.

### Check Service Health
```bash
curl http://localhost:8080/api/v1/health
```

### View Admin Token from Pod Logs
If no static admin token was configured:
```bash
kubectl --namespace hermesgate logs -l "app.kubernetes.io/name=hermesgate" | grep "Master Admin Token"
```

---

## 6. Upgrading and Uninstalling

### Upgrading the Release
```bash
helm upgrade hermesgate ./charts/hermesgate \
  --namespace hermesgate \
  -f my-values.yaml
```

### Uninstalling
```bash
helm uninstall hermesgate --namespace hermesgate
```
*(Note: Your PersistentVolumeClaim and SQLite database remain intact unless explicitly deleted).*
