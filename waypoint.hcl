project = "rust-library"

app "rust-library" {
  build {
    use "docker" {}

    registry {
      use "docker" {
        image = "jakule/rust-library"
        tag   = gitrefpretty()
      }
    }

  }

  deploy {
    use "kubernetes-apply" {
      path        = templatefile("${path.app}/k8s/deployment.yml")
      prune_label = "app=backend"
    }
  }
}
