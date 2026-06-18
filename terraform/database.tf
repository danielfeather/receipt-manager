# resource "kubernetes_persistent_volume_claim_v1" "name" {
#   metadata {
    
#   }
#   spec {
#     access_modes = ["ReadWriteOnce"]
#     resources {
#       requests = {
#         storage = "20Gi"
#       }
#     }
#   }
# }