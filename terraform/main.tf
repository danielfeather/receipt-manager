terraform {
  backend "s3" {
  }
}

provider "kubernetes" {
  config_path    = "~/.kube/config"
  config_context = "default"
}

resource "kubernetes_namespace_v1" "this" {
  metadata {
    name = "receipt-manager"
  }
}

resource "kubernetes_deployment_v1" "this" {
  wait_for_rollout = false
  metadata {
    name = "receipt-manager"
    namespace = "receipt-manager"
    labels = {
      "app.kubernetes.io/name" = "receipt-manager"
    }
  }
  spec {
    replicas = 1
    selector {
      match_labels = {
        "app.kubernetes.io/name" = "receipt-manager"
      }
    }
    template {
        metadata {
          labels = {
            "app.kubernetes.io/name" = "receipt-manager"
          }
        }
        spec {
          security_context {
            run_as_non_root = true
            run_as_user = 1000
            run_as_group = 1000
          }
          container {
            name = "receipt-manager"
            image = var.image
            image_pull_policy = "Always"
            port {
              name = "http"
              container_port = 3000
              protocol = "TCP"
            }

            env_from {
              secret_ref {
                name = "aws-credentials"
              }
            }
            
            liveness_probe {
              http_get {
                path = "/"
                port = "http"
              }
            }
            readiness_probe {
              http_get {
                path = "/"
                port = "http"
              }
            }
          }
        }
    }
  }
}

resource "kubernetes_service_v1" "this" {
  metadata {
    name = "receipt-manager"
    namespace = "receipt-manager"
    labels = {
      "app.kubernetes.io/name" = "receipt-manager" 
    }
  }
  spec {
    type = "ClusterIP"
    session_affinity = "None"
    port {
      port = 3000
      target_port = "http"
      protocol = "TCP"
      name = "http"
    }
    selector = {
      "app.kubernetes.io/name" = kubernetes_deployment_v1.this.spec.0.template.0.metadata.0.labels["app.kubernetes.io/name"]
    }
  }
}

resource "kubernetes_manifest" "middleware" {
  manifest = yamldecode(file("middleware.yaml"))
}

resource "kubernetes_ingress_v1" "this" {
  depends_on = [ kubernetes_manifest.middleware ]
  metadata {
    name = "receipt-manager"
    namespace = "receipt-manager"
    annotations = {
      "traefik.ingress.kubernetes.io/router.entrypoints" = "websecure"
      "traefik.ingress.kubernetes.io/router.middlewares" = "receipt-manager-traefik-forward-auth@kubernetescrd"
    }
  }
  spec {
    rule {
      host = var.hostname
      http {
        path {
          path = "/"
          path_type = "Prefix"
          backend {
            service {
              name = "receipt-manager"
              port {
                number = 3000
              }
            }
          }
        }
      }
    }
    tls {
      hosts = [ var.hostname ]
    }
  }
}