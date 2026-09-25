---
title: Розгортаємо Rust-застосунок з Terraform на AWS Fargate
description: Дізнайтеся, як розгорнути Loco-застосунок за допомогою Terraform (IaC). Згенеруйте розгортання генераторами Loco та налаштуйте все крок за кроком.
pubDate: 2023-12-20
updatedDate: 2023-12-16
authors:
  - antonio-souza
---

У сьогоднішньому стрімко мінливому технологічному ландшафті Infrastructure as Code (IaC) стала наріжним каменем ефективного, масштабованого та підтримуваного розгортання хмарної інфраструктури. IaC передбачає керування та provision'ing обчислювальної інфраструктури через машинозчитувані файли скриптів, а не через фізичне налаштування обладнання чи інтерактивні інструменти конфігурації. Це дає змогу автоматизувати розгортання та керування інфраструктурою, що, своєю чергою, знижує ризик людських помилок і пришвидшує розгортання.

У цій статті ми розглянемо, як розгорнути Rust-застосунок, побудований на [loco](https://loco.rs), на AWS Fargate за допомогою Terraform. Почнемо зі створення нового проєкту та вибору шаблону `Rest API`:

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

## Передумови

Щоб розгорнути наш застосунок на AWS Fargate, нам знадобляться такі інструменти:

- [Docker](https://docs.docker.com/get-docker/) — Docker — це платформа контейнеризації, яка дозволяє запакувати ваш застосунок і всі його залежності в стандартизований блок для розробки ПЗ.
- [Terraform](https://learn.hashicorp.com/tutorials/terraform/install-cli) — Terraform — це open-source інструмент infrastructure as code, який дозволяє безпечно та передбачувано створювати, змінювати й покращувати інфраструктуру.
- [AWS CLI](https://docs.aws.amazon.com/cli/latest/userguide/install-cliv2.html) — AWS Command Line Interface (CLI) — це уніфікований інструмент для керування сервісами AWS.

## Створення Docker-образу

Для створення Docker-образу нашого застосунку ми скористаємося loco CLI. Команда `cargo loco generate deployment` створить Docker-образ для нашого застосунку. Вона також створить `Dockerfile`, який ми зможемо використати для збірки образу.

```sh
$ cargo loco generate deployment
? ❯ Choose your deployment ›
❯ Docker

added: "Dockerfile"
added: ".dockerignore"
```

Тепер ми можемо зібрати Docker-образ, який використовуватиметься для розгортання нашого застосунку на AWS Fargate.

```sh
$ docker build -t myapp .

[+] Building 237.1s (16/16) FINISHED                                                                                                               docker:desktop-linux
 => [internal] load build definition from Dockerfile                                                                                                               0.0s
 => => transferring Dockerfile: 331B                                                                                                                               0.0s
 ...
 => => writing image sha256:07416ca8195e4026ab65bc567f990ea83141aa10890f8443deb8f54a8bae7f0a                                                                       0.0s
 => => naming to docker.io/library/myapp
```

## Налаштування AWS

Щоб розгорнути наш застосунок на AWS Fargate, нам знадобиться створити обліковий запис AWS та налаштувати AWS CLI. Створити обліковий запис AWS можна [тут](https://portal.aws.amazon.com/billing/signup#/start/email).

Також потрібно встановити AWS CLI. Інструкції зі встановлення можна знайти [тут](https://docs.aws.amazon.com/cli/latest/userguide/install-cliv2.html).

Нарешті, потрібно створити IAM-користувача для роботи з AWS CLI. Інструкції можна знайти [тут](https://docs.aws.amazon.com/IAM/latest/UserGuide/id_users_create.html).

Тепер ми можемо налаштувати AWS CLI з обліковими даними щойно створеного IAM-користувача.

```sh
$ aws configure
AWS Access Key ID [None]: <your access key id>
AWS Secret Access Key [None]: <your secret access key>
Default region name [None]: <your region>
Default output format [None]: json
```

## Створення репозиторію на ECR

Щоб розгорнути наш застосунок на AWS Fargate, потрібно створити репозиторій на ECR. Це можна зробити, виконавши наступну команду:

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

## Завантаження Docker-образу до ECR

Тепер можна завантажити Docker-образ до ECR. Виконайте наступні команди:

-1. Увійдіть до ECR

```sh
$ aws ecr get-login-password --region us-east-1 | docker login --username AWS --password-stdin 123456789012.dkr.ecr.us-east-1.amazonaws.com
```

-2. Присвойте тег Docker-образу

```sh
$ docker tag myapp:latest 123456789012.dkr.ecr.us-east-1.amazonaws.com/myapp:latest
```

-3. Завантажте Docker-образ до ECR

```sh
$ docker push 123456789012.dkr.ecr.us-east-1.amazonaws.com/myapp:latest
```

## Створення файлу main.tf для Terraform

Це головний файл Terraform, який використовуватиметься для розгортання нашого застосунку на AWS Fargate. Він створить наступні ресурси:

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

Цей файл створить наступні ресурси:

- Репозиторій ECR для нашого застосунку
- Кластер ECS для нашого застосунку
- Визначення завдання ECS (task definition) для нашого застосунку
- Сервіс ECS для нашого застосунку

Тепер потрібно створити файл `network.tf`, щоб визначити мережеву конфігурацію для нашого застосунку. Цей файл створить наступні ресурси:

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

Мережева конфігурація відповідає за створення всієї інфраструктури, потрібної для розгортання нашого застосунку на AWS Fargate у частині мережі. Раджу прочитати [документацію AWS Fargate](https://docs.aws.amazon.com/AmazonECS/latest/developerguide/AWS_Fargate.html), щоб зрозуміти, як це працює; також можна почитати документацію Terraform для [AWS Fargate](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/ecs_task_definition) та [AWS VPC](https://registry.terraform.io/providers/hashicorp/aws/latest/docs/resources/vpc).

Отже, тепер у нас є головний файл Terraform і файл мережевої конфігурації для нашого застосунку. Можемо розгортати наш застосунок на AWS Fargate.

## Розгортання застосунку на AWS Fargate

Щоб розгорнути наш застосунок на AWS Fargate, потрібно виконати наступні команди:

-1. Ініціалізуйте Terraform

```sh
$ terraform init
```

-2. Сплануйте розгортання

```sh
$ terraform plan
```

-3. Застосуйте розгортання

````sh
$ terraform apply
```****

Ці команди створять усі ресурси, потрібні для розгортання нашого застосунку на AWS Fargate. Після виконання ви побачите URL з нашого виводу alb_url.

```sh
Apply complete! Resources: 20 added, 0 changed, 0 destroyed.

Outputs:

alb_url = https://myapp-lb-1234567890.us-east-2.elb.amazonaws.com
````

Тепер ми можемо отримати доступ до нашого застосунку, перейшовши за URL з виводу alb_url.

## Висновок

У цій статті ми розглянули, як розгорнути Rust-застосунок, побудований на loco, на AWS Fargate за допомогою Terraform. Ми почали зі створення нового проєкту та вибору шаблону `Rest API`. Потім створили Docker-образ для нашого застосунку та завантажили його до ECR. Нарешті, ми створили головний файл Terraform і файл мережевої конфігурації для нашого застосунку та розгорнули його на AWS Fargate.

Такий підхід дозволяє розгортати наш застосунок на AWS Fargate швидко та надійно. Він також дає змогу легко масштабувати застосунок, додаючи більше його інстансів.
