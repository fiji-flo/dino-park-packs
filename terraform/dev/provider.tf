#---
# Provider Configuration
#---

provider "aws" {
  region = "us-west-2"
}

terraform {
  required_version = "~> 0.12"

  backend "s3" {
    bucket = "eks-terraform-shared-state"
    key    = "stage/us-west-2/apps/dino-park-packs-dev/terraform.tfstate"
    region = "us-west-2"
  }
}

