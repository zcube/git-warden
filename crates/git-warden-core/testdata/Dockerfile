# 빌드 스테이지: Go 소스를 컴파일합니다
FROM golang:1.26-alpine AS builder

WORKDIR /app

# 의존성 파일을 먼저 복사하여 레이어 캐시를 활용합니다
COPY go.mod go.sum ./
RUN go mod download

COPY . .

# 바이너리를 정적 링크로 빌드합니다
RUN CGO_ENABLED=0 GOOS=linux go build -o /app/server ./cmd/server

# 실행 스테이지: 최소 이미지로 운영 환경을 구성합니다
FROM alpine:3.24

# 타임존과 인증서를 설치합니다
RUN apk add --no-cache tzdata ca-certificates

WORKDIR /app

COPY --from=builder /app/server .

# 기본 포트 노출
EXPOSE 8080

ENTRYPOINT ["./server"]
