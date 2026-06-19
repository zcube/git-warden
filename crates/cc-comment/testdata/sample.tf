# 웹 서버 인스턴스 정의 — 운영 환경 기본 설정
resource "aws_instance" "web" {
  ami           = "ami-0c55b159cbfafe1f0" // 기본 AMI 식별자
  instance_type = var.instance_type

  /* 블록 주석:
     태그는 비용 추적을 위해 필수입니다. */
  tags = {
    Name = "web-${var.env == "prod" ? "운영" : "개발"}-server"
    Note = "literal $${not_interpolation} 와 url#fragment 는 문자열입니다"
  }

  # 초기화 스크립트 — heredoc 본문은 언어 검사 대상이 아닙니다
  user_data = <<-EOF
    #!/bin/bash
    # this hash inside heredoc is not a comment
    echo "started ${var.env}"
  EOF
}
