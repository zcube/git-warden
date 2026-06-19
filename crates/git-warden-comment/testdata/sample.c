#include <stdio.h>
#include <stdlib.h>
#include "utils.h"
#include "config.h"

/* 프로그램의 진입점입니다.
   명령행 인자를 처리하고 메인 루프를 실행합니다.
*/
int main(int argc, char *argv[]) {
    // 인자 수 확인
    if (argc < 2) {
        fprintf(stderr, "사용법: %s <파일명>\n", argv[0]);
        return EXIT_FAILURE;
    }

    const char *filename = argv[1]; // 첫 번째 인자를 파일명으로 사용
    char *error_msg = "파일을 열 수 없습니다";

    FILE *fp = fopen(filename, "r");
    if (!fp) {
        /* 파일 열기 실패 처리:
           오류 메시지를 출력하고 종료합니다.
        */
        fprintf(stderr, "%s: %s\n", error_msg, filename);
        return EXIT_FAILURE;
    }

    fclose(fp);
    return EXIT_SUCCESS;
}
