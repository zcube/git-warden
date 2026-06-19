package sample

import (
	"fmt"
	"strings"

	"github.com/example/somepackage"
)

// 패키지 수준 상수 — 최대 재시도 횟수를 정의합니다.
const MaxRetry = 3

// Config 는 애플리케이션 설정을 담는 구조체입니다.
type Config struct {
	Name string
	Age  int
}

func process(cfg Config) error {
	msg := "처리 시작" // 인라인 주석
	_ = msg

	// 입력 값 유효성 검사
	if cfg.Name == "" {
		return fmt.Errorf("이름이 비어 있습니다")
	}

	result := strings.TrimSpace(cfg.Name)
	_ = somepackage.DoSomething(result)

	/* 블록 주석:
	   여러 줄에 걸친 설명입니다.
	*/
	return nil
}
