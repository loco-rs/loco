+++
title = "Deploying Rust App with Terraform on AWS Fargate"
description = "Learn how to deploy a Loco app with Terraform (IaC). Generate a deployment with Loco generators and set it up step-by-step."
date = 2023-12-20T16:04:40+00:00
updated = 2023-12-16T04:20:40+00:00
draft = false
template = "blog/page.html"

[taxonomies]
authors = ["Antonio Souza"]

+++

In today's rapidly evolving technological landscape, Infrastructure as Code (IaC) has become a cornerstone for efficient, scalable, and maintainable cloud infrastructure deployment. IaC involves managing and provisioning computing infrastructure through machine-readable script files, rather than through physical hardware configuration or interactive configuration tools. This allows for the automation of infrastructure deployment and management, which in turn reduces the risk of human error and increases the speed of deployment.

In this article, we will explore how to deploy a Rust app built with [loco](https://loco.rs) on AWS Fargate using Terraform. We will start by creating a new project and selecting the `Rest API` template:

````sh

```sh
$ cargo install loco
$ loco new
✔ ❯ App name? · myapp
? ❯ What would you like to build? ›
  lightweight-service (minimal, only controllers and views)
❯ Rest API (with DB and user auth)
  SaaS app (with DB and user auth)
````

## Prerequisites

To deploy our app on AWS Fargate, we will need to have the following tools installed:

- [Docker](https://docs.docker.com/get-docker/) - Docker is a containerization platform that allows you to package your application and all of its dependencies into a standardized unit for software development.
- [Terraform](https://learn.hashicorp.com/tutorials/terraform/install-cli) - Terraform is an open-source infrastructure as code software tool that enables you to safely and predictably create, change, and improve infrastructure.
- [AWS CLI](https://docs.aws.amazon.com/cli/latest/userguide/install-cliv2.html) - The AWS Command Line Interface (CLI) is a unified tool to manage your AWS services.

## Creating the Docker Image

To create the Docker image for our app, we will use the loco CLI. The `cargo loco generate deployment` command will create a Docker image for our app. It will also create a `Dockerfile` for us, which we can use to build the image.

```sh
$ cargo loco generate deployment
? ❯ Choose your deployment ›
❯ Docker
  Shuttle

added: "Dockerfile"
added: ".dockerignore"
```

Now, we can build the Docker image which will be used to deploy our app on AWS Fargate.

```sh
$ docker build -t myapp .

[+] Building 237.1s (16/16) FINISHED                                                                                                               docker:desktop-linux
 => [internal] load build definition from Dockerfile                                                                                                               0.0s
 => => transferring Dockerfile: 331B                                                                                                                               0.0s
 ...
 => => writing image sha256:07416ca8195e4026ab65bc567f990ea83141aa10890f8443deb8f54a8bae7f0a                                                                       0.0s
 => => naming to docker.io/library/myapp
```

## Setting up AWS

To deploy our app on AWS Fargate, we will need to create an AWS account and set up the AWS CLI. You can create an AWS account [here](https://portal.aws.amazon.com/billing/signup#/start/email).

You will also need to install the AWS CLI. You can find instructions on how to do this [here](https://docs.aws.amazon.com/cli/latest/userguide/install-cliv2.html).

Finally, you need to create an IAM user to use with the AWS CLI. You can find instructions on how to do this [here](https://docs.aws.amazon.com/IAM/latest/UserGuide/id_users_create.html).

Now, we can configure the AWS CLI with the credentials of the IAM user we just created.

```sh
$ aws configure
AWS Access Key ID [None]: <your access key id>
AWS Secret Access Key [None]: <your secret access key>
Default region name [None]: <your region>
Default output format [None]: json
```

## Creating the repository on ECR

To deploy our app on AWS Fargate, we will need to create a repository on ECR. You can do this by running the following command:

```sh
$ aws ecr create-repository --repository-name myapp

{
    "repository": {
        "repositoryArn": "arn:aws:ecr:us-east-1:123456789012:repository/myapp",
        "registryId": "123456789012",
        "repositoryName": "myapp",
        "repositoryUri": "123456789012.dkr.ecr.us-east-1.amazonaws.com/myapp",
        "createdAt": 1627981234.0,
        "imageTagMutability": "MUTABLE",
        "imageScanningConfiguration": {
            "scanOnPush": false
        }
    }
}
```

## Pushing the Docker image to ECR

Now, we can push the Docker image to ECR. You can do this by running the following commands:

-1. Log in to ECR

```sh
$ aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin 123456789012.dkr.ecr.us-east-1.amazonaws.com
```

-2. Tag the Docker image

```sh
$ docker tag myapp:latest 123456789012.dkr.ecr.us-east-1.amazonaws.com/myapp:latest
```

-3. Push the Docker image to ECR

```sh
$ docker push 123456789012.dkr.ecr.us-east-1.amazonaws.com/myapp:latest
```

## Creating the main.tf file for Terraform

This is the main Terraform file that will be used to deploy our app on AWS Fargate. It will create the following resources:

```hcl
terraform {
  required_providers {
    aws = {
      source = "hashicorp/aws"
      version = "~> 4.0"
    }
    archive = {
      source = "hashicorp/archive"
      version = "~> 2.2.0"
    }
  }

  required_version = "~> 1.0"
}

# Configure the AWS Provider
provider "aws" {
  region = "us-east-1" // Change this to your region
  access_key = "<your access key>" // Change this to your access key
  secret_key = "your secret key" // Change this to your secret key
}

resource "aws_ecr_repository" "myapp" {
  name = "myapp"
}

resource "aws_ecs_cluster" "myapp_cluster" {
  name = "myapp_cluster"
}

resource "aws_cloudwatch_log_group" "myapp" {
  name = "/ecs/myapp"
}

resource "aws_ecs_task_definition" "myapp_task" {
  family                   = "myapp-task"
  container_definitions    = <<DEFINITION
  [
    {
      "name": "myapp-task",
      "image": "${aws_ecr_repository.myapp.repository_url}",
      "essential": true,
      "portMappings": [
        {
          "containerPort": 5150
        }
      ],
      "command": ["start"],
      "memory": 512,
      "cpu": 256,
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-region": "us-east-2",
          "awslogs-group": "/ecs/myapp",
          "awslogs-stream-prefix": "ecs"
        }
      }
    }
  ]
  DEFINITION
  requires_compatibilities = ["FARGATE"]
  network_mode             = "awsvpc"
  memory                   = 512
  cpu                      = 256
  execution_role_arn       = aws_iam_role.ecsTaskExecutionRole.arn
}

resource "aws_iam_role" "ecsTaskExecutionRole" {
  name               = "ecsTaskExecutionRoleMyapp"
  assume_role_policy = data.aws_iam_policy_document.assume_role_policy.json
}

data "aws_iam_policy_document" "assume_role_policy" {
  statement {
    actions = ["sts:AssumeRole"]

    principals {
      type        = "Service"
      identifiers = ["ecs-tasks.amazonaws.com"]
    }
  }
}

resource "aws_iam_role_policy_attachment" "ecsTaskExecutionRole_policy" {
  role       = aws_iam_role.ecsTaskExecutionRole.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

resource "aws_alb" "myapp" {
  name               = "myapp-lb"
  internal           = false
  load_balancer_type = "application"
  enable_deletion_protection = true

  subnets = [
    aws_subnet.public_d.id,
    aws_subnet.public_e.id,
  ]

  security_groups = [
    aws_security_group.http.id,
    aws_security_group.https.id,
    aws_security_group.egress_all.id,
  ]

  depends_on = [aws_internet_gateway.igw]
}


resource "aws_security_group" "load_balancer_security_group" {
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }
  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
resource "aws_lb_target_group" "myapp" {
  name        = "myapp-tg"
  port        = 5150
  protocol    = "HTTP"
  target_type = "ip"
  vpc_id      = aws_vpc.myapp_vpc.id

  health_check {
    enabled = true
    path    = "/_health"
    matcher = "200,202"
  }

  depends_on = [aws_alb.myapp]
}

resource "aws_alb_listener" "myapp_http" {
  load_balancer_arn = aws_alb.myapp.arn
  port              = "80"
  protocol          = "HTTP"

  default_action {
    type =  "redirect"
    redirect {
      port        = "443"
      protocol    = "HTTPS"
      status_code = "HTTP_301"
    }
  }
}

resource "aws_alb_listener" "myapp_https" {
  load_balancer_arn = aws_alb.myapp.arn
  port              = "443"
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-2016-08"

  certificate_arn = "<your arn for the certificate>" // Change this to your certificate ARN

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.myapp.arn
  }
}

output "alb_url" {
  value = "https://${aws_alb.myapp.dns_name}"
}
resource "aws_ecs_service" "myapp" {
  name            = "myapp-service"
  cluster         = aws_ecs_cluster.myapp_cluster.id
  task_definition = aws_ecs_task_definition.myapp_task.arn
  launch_type     = "FARGATE"
  desired_count   = 1

  load_balancer {
    target_group_arn = aws_lb_target_group.myapp.arn
    container_name   = aws_ecs_task_definition.myapp_task.family
    container_port   = 5150
  }

  network_configuration {
    assign_public_ip = false

    security_groups = [
      aws_security_group.egress_all.id,
      aws_security_group.ingress_api.id,
    ]

    subnets = [
    aws_subnet.private_d.id,
    aws_subnet.private_e.id,
    ]
  }
}


resource "aws_security_group" "service_security_group" {
  ingress {
    from_port       = 0
    to_port         = 0
    protocol        = "-1"
    security_groups = ["${aws_security_group.load_balancer_security_group.id}"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
```

This file will create the following resources:

- An ECR repository for our app
- An ECS cluster for our app
- An ECS task definition for our app
- An ECS service for our app

Now, we need to create a `network.tf` file to define the network configuration for our app. This file will create the following resources:

```hcl
resource "aws_vpc" "myapp_vpc" {
  cidr_block = "10.0.0.0/16"
}

resource "aws_subnet" "public_d" {
  vpc_id            = aws_vpc.myapp_vpc.id
  cidr_block        = "10.0.1.0/25"
  availability_zone = "us-east-2a"

  tags = {
    "Name" = "public | us-east-2a"
  }
}

resource "aws_subnet" "private_d" {
  vpc_id            = aws_vpc.myapp_vpc.id
  cidr_block        = "10.0.2.0/25"
  availability_zone = "us-east-2b"

  tags = {
    "Name" = "private | us-east-2b"
  }
}

resource "aws_subnet" "public_e" {
  vpc_id            = aws_vpc.myapp_vpc.id
  cidr_block        = "10.0.1.128/25"
  availability_zone = "us-east-2c"

  tags = {
    "Name" = "public | us-east-2c"
  }
}

resource "aws_subnet" "private_e" {
  vpc_id            = aws_vpc.myapp_vpc.id
  cidr_block        = "10.0.2.128/25"
  availability_zone = "us-east-2c"

  tags = {
    "Name" = "private | us-east-2c"
  }
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.myapp_vpc.id
  tags = {
    "Name" = "public"
  }
}

resource "aws_route_table" "private" {
  vpc_id = aws_vpc.myapp_vpc.id
  tags = {
    "Name" = "private"
  }
}

resource "aws_route_table_association" "public_d_subnet" {
  subnet_id      = aws_subnet.public_d.id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table_association" "private_d_subnet" {
  subnet_id      = aws_subnet.private_d.id
  route_table_id = aws_route_table.private.id
}

resource "aws_route_table_association" "public_e_subnet" {
  subnet_id      = aws_subnet.public_e.id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table_association" "private_e_subnet" {
  subnet_id      = aws_subnet.private_e.id
  route_table_id = aws_route_table.private.id
}

resource "aws_eip" "nat" {
  vpc = true
}

resource "aws_internet_gateway" "igw" {
  vpc_id = aws_vpc.myapp_vpc.id
}

resource "aws_nat_gateway" "ngw" {
  subnet_id     = aws_subnet.public_d.id
  allocation_id = aws_eip.nat.id

  depends_on = [aws_internet_gateway.igw]
}

resource "aws_route" "public_igw" {
  route_table_id         = aws_route_table.public.id
  destination_cidr_block = "0.0.0.0/0"
  gateway_id             = aws_internet_gateway.igw.id
}

resource "aws_route" "private_ngw" {
  route_table_id         = aws_route_table.private.id
  destination_cidr_block = "0.0.0.0/0"
  nat_gateway_id         = aws_nat_gateway.ngw.id
}

resource "aws_security_group" "http" {
  name        = "http"
  description = "HTTP traffic"
  vpc_id      = aws_vpc.myapp_vpc.id

  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "TCP"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_security_group" "https" {
  name        = "https"
  description = "HTTPS traffic"
  vpc_id      = aws_vpc.myapp_vpc.id

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "TCP"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_security_group" "egress_all" {
  name        = "egress-all"
  description = "Allow outbound traffic"
  vpc_id      = aws_vpc.myapp_vpc.id

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }
}

resource "aws_security_group" "ingress_api" {
  name        = "ingress-api"
  description = "Allow ingress to App"
  vpc_id      = aws_vpc.myapp_vpc.id

  ingress {
    from_port   = 5150
    to_port     = 5150
    protocol    = "TCP"
    cidr_blocks = ["0.0.0.0/0"]
  }
}
```

The network configuration will be responsible for creating all the infrastructure needed to deploy our app on AWS Fargate in terms of networking. I recommend you to read the [AWS Fargate documentation](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/AWS_Fargate.html) to understand how it works, also you can read the Terraform documentation for [AWS Fargate](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/ecs_task_definition) and [AWS VPC](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/vpc).

So, now we have the main Terraform file and the network configuration file for our app. We can now deploy our app on AWS Fargate.

## Deploying the app on AWS Fargate

To deploy our app on AWS Fargate, we will need to run the following commands:

-1. Initialize Terraform

```sh
$ terraform init
```

-2. Plan the deployment

```sh
$ terraform plan
```

-3. Apply the deployment

````sh
$ terraform apply
```****

Theses commands will create all the resources we need to deploy our app on AWS Fargate. After running you will see the url from our alb_url output.

```sh
Apply complete! Resources: 20 added, 0 changed, 0 destroyed.

Outputs:

alb_url = https://myapp-lb-1234567890.us-east-2.elb.amazonaws.com
````

Now, we can access our app by going to the url from our alb_url output.

## Conclusion

In this article, we explored how to deploy a Rust app built with loco on AWS Fargate using Terraform. We started by creating a new project and selecting the `Rest API` template. Then, we created the Docker image for our app and pushed it to ECR. Finally, we created the main Terraform file and the network configuration file for our app and deployed it on AWS Fargate.

This approach allows us to deploy our app on AWS Fargate in a fast and reliable way. It also allows us to easily scale our app by adding more instances of it.
